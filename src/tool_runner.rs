//! Optional, bounded execution of explicitly registered client tools.
//!
//! Callbacks run only after a complete Messages response (or terminal SSE
//! collection), never from partial input JSON. Tool results contain strings or
//! supported content blocks; arbitrary JSON is encoded as text. See Anthropic's
//! [tool-runner workflow](https://platform.claude.com/docs/en/agents-and-tools/tool-use/tool-runner),
//! verified 2026-10-03. This community runner has its own explicit safety budgets
//! and does not claim every official SDK convenience.

use crate::{
    builders::message_builder::validate_model_options,
    error::{AnthropicError, Result},
    models::{
        ContentBlock, ExtraFields, Message, MessageRequest, MessageResponse, ReplayUnknownPolicy,
        Role, StopReason, Tool, ToolChoice, ToolResultContent, Usage,
    },
    types::RequestOptions,
    Client,
};
use futures::{future::BoxFuture, FutureExt, StreamExt};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    collections::{BTreeMap, HashSet},
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::watch;

mod cancellation;
mod execution;
mod options;
mod registry;
mod result;
mod turns;

pub use cancellation::ToolRunnerCancellation;
pub use options::{CallbackErrorPolicy, CompactionReplayPolicy, ToolCall, ToolRunnerOptions};
use registry::RegisteredTool;
pub use registry::ToolRegistry;
pub use result::{ToolRunnerResult, ToolRunnerTermination};

type ExecutionHook = Arc<dyn Fn(ToolCall) -> BoxFuture<'static, Result<()>> + Send + Sync>;
type StartedCalls = Arc<Mutex<Vec<String>>>;
type PreparedCalls = Vec<(ToolCall, RegisteredTool)>;
type RunFailure = (ToolRunnerTermination, Option<AnthropicError>);
type RunResult<T> = std::result::Result<T, RunFailure>;

/// Bounded orchestration for explicitly registered client tools.
#[derive(Clone)]
pub struct ToolRunner {
    client: Client,
    registry: ToolRegistry,
    options: ToolRunnerOptions,
    cancellation: ToolRunnerCancellation,
    hook: Option<ExecutionHook>,
}
impl ToolRunner {
    /// Construct a runner with validated finite limits.
    pub fn new(client: Client, registry: ToolRegistry, options: ToolRunnerOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self {
            client,
            registry,
            options,
            cancellation: ToolRunnerCancellation::new(),
            hook: None,
        })
    }
    /// Attach an explicit cancellation token.
    pub fn with_cancellation(mut self, cancellation: ToolRunnerCancellation) -> Self {
        self.cancellation = cancellation;
        self
    }
    /// Call an async user hook before execution. All calls in a turn are checked
    /// and approved before any callback starts. No interactive approval is implicit.
    pub fn with_execution_hook<F, Fut>(mut self, hook: F) -> Self
    where
        F: Fn(ToolCall) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.hook = Some(Arc::new(move |call| hook(call).boxed()));
        self
    }
    /// Execute ordinary Messages turns. Missing tool choice defaults to `auto`;
    /// explicit caller settings and model/thinking configuration are retained.
    ///
    /// ```rust,no_run
    /// use serde_json::json;
    /// use threatflux_anthropic_sdk::{Client, models::{MessageRequest, Tool, ToolResultContent},
    ///     tool_runner::{ToolRegistry, ToolRunner, ToolRunnerOptions}};
    /// # async fn example() -> threatflux_anthropic_sdk::Result<()> {
    /// let mut registry = ToolRegistry::new();
    /// registry.register(Tool::new("lookup", "Look up a value", json!({"type":"object"})),
    ///     |input| if input.is_object() { Ok(()) } else {
    ///         Err(threatflux_anthropic_sdk::AnthropicError::invalid_input("object required"))
    ///     },
    ///     |_input| async { Ok(ToolResultContent::Text("explicit callback result".into())) })?;
    /// let runner = ToolRunner::new(Client::from_env()?, registry, ToolRunnerOptions::default())?;
    /// let result = runner.run(MessageRequest::new().max_tokens(256)
    ///     .add_user_message("Look up a value"), None).await;
    /// println!("{:?}", result.termination);
    /// # Ok(()) }
    /// ```
    pub async fn run(
        &self,
        request: MessageRequest,
        options: Option<RequestOptions>,
    ) -> ToolRunnerResult {
        self.run_mode(request, options, false).await
    }
    /// Collect each streaming turn through terminal `message_stop`, then execute
    /// callbacks. An incomplete SSE response never invokes a tool.
    pub async fn run_streaming(
        &self,
        request: MessageRequest,
        options: Option<RequestOptions>,
    ) -> ToolRunnerResult {
        self.run_mode(request, options, true).await
    }
    async fn run_mode(
        &self,
        mut request: MessageRequest,
        options: Option<RequestOptions>,
        streaming: bool,
    ) -> ToolRunnerResult {
        let mut result = ToolRunnerResult {
            messages: request.messages.clone(),
            responses: Vec::new(),
            usage: Vec::new(),
            started_calls: Vec::new(),
            completed_calls: Vec::new(),
            termination: ToolRunnerTermination::InvalidRequest,
            error: None,
        };
        let started = Arc::new(Mutex::new(Vec::new()));
        let deadline = tokio::time::Instant::now() + self.options.overall_timeout;
        request.stream = Some(streaming);
        if request.tools.is_none() && !self.registry.is_empty() {
            request.tools = Some(self.registry.definitions());
        }
        if request.tool_choice.is_none()
            && request
                .tools
                .as_ref()
                .is_some_and(|tools| !tools.is_empty())
        {
            request.tool_choice = Some(ToolChoice::Auto);
        }
        let validation = self
            .validate_definitions(&request)
            .and_then(|()| validate_model_options(&request));
        if let Err(error) = validation {
            result.error = Some(error);
            return result;
        }
        result.termination = tokio::select! {
            biased;
            () = self.cancellation.cancelled() => ToolRunnerTermination::Cancelled,
            () = tokio::time::sleep_until(deadline) => ToolRunnerTermination::OverallTimeout,
            termination = self.run_loop(request, options, streaming, &mut result, &started) => termination,
        };
        result.started_calls = started.lock().expect("tool-start ledger poisoned").clone();
        result
    }
    fn validate_definitions(&self, request: &MessageRequest) -> Result<()> {
        let mut seen = HashSet::new();
        for tool in request.tools.as_deref().unwrap_or_default() {
            if !seen.insert(&tool.name) {
                return Err(AnthropicError::invalid_input("duplicate offered tool name"));
            }
            if tool.is_client() {
                let registered = self.registry.tools.get(&tool.name).ok_or_else(|| {
                    AnthropicError::invalid_input("offered client tool is not registered")
                })?;
                if tool != &registered.definition {
                    return Err(AnthropicError::invalid_input(
                        "offered client tool differs from registered definition",
                    ));
                }
            }
        }
        Ok(())
    }
}
