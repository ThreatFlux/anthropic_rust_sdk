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

type Validator = Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>;
type Callback = Arc<dyn Fn(Value) -> BoxFuture<'static, Result<ToolResultContent>> + Send + Sync>;
type ExecutionHook = Arc<dyn Fn(ToolCall) -> BoxFuture<'static, Result<()>> + Send + Sync>;

#[derive(Clone)]
struct RegisteredTool {
    definition: Tool,
    validator: Validator,
    callback: Callback,
}

/// Explicit callbacks and input validators for custom client tools.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, RegisteredTool>,
}
impl ToolRegistry {
    /// Create an empty registry; no tool executes without registration.
    pub fn new() -> Self {
        Self::default()
    }
    /// Register a custom tool, a synchronous decoder/schema validator, and an
    /// async callback. Validators run for every call in a turn before any callback.
    pub fn register<V, F, Fut>(&mut self, definition: Tool, validator: V, callback: F) -> Result<()>
    where
        V: Fn(&Value) -> Result<()> + Send + Sync + 'static,
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolResultContent>> + Send + 'static,
    {
        if definition.name.is_empty()
            || !definition.is_client()
            || !definition
                .input_schema
                .as_ref()
                .is_some_and(Value::is_object)
        {
            return Err(AnthropicError::invalid_input(
                "register a named custom tool with an object input schema",
            ));
        }
        if self.tools.contains_key(&definition.name) {
            return Err(AnthropicError::invalid_input(
                "duplicate registered tool name",
            ));
        }
        // Check flattened reserved keys before saving a definition.
        serde_json::to_value(&definition)?;
        self.tools.insert(
            definition.name.clone(),
            RegisteredTool {
                definition,
                validator: Arc::new(validator),
                callback: Arc::new(move |input| callback(input).boxed()),
            },
        );
        Ok(())
    }
    /// Register a typed input decoder. The caller supplies the corresponding
    /// schema; use `register` to add validation beyond serde's field checks.
    pub fn register_typed<I, F, Fut>(&mut self, definition: Tool, callback: F) -> Result<()>
    where
        I: DeserializeOwned + Send + 'static,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolResultContent>> + Send + 'static,
    {
        let callback = Arc::new(callback);
        self.register(
            definition,
            |value| {
                serde_json::from_value::<I>(value.clone())
                    .map(|_| ())
                    .map_err(|_| {
                        AnthropicError::invalid_input(
                            "tool input does not match registered decoder",
                        )
                    })
            },
            move |value| {
                let callback = callback.clone();
                async move {
                    let input = serde_json::from_value::<I>(value).map_err(|_| {
                        AnthropicError::invalid_input(
                            "tool input does not match registered decoder",
                        )
                    })?;
                    callback(input).await
                }
            },
        )
    }
    /// Definitions offered by the registry, in deterministic name order.
    pub fn definitions(&self) -> Vec<Tool> {
        self.tools
            .values()
            .map(|tool| tool.definition.clone())
            .collect()
    }
    /// Number of explicitly registered callbacks.
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    /// Whether the registry has no callbacks.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

/// A requested client invocation supplied to an optional execution hook.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolCall {
    /// Original identifier used in the corresponding tool result.
    pub id: String,
    /// Explicitly registered name.
    pub name: String,
    /// Complete, already decoded object input.
    pub input: Value,
}

/// Callback error handling. Remote error content never includes callback details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CallbackErrorPolicy {
    /// Stop and return the partial transcript plus the local callback error.
    Stop,
    /// Return fixed sanitized `is_error` content to the model, then continue.
    ReturnSanitizedError,
}
/// How a complete compaction block is replayed on a subsequent turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompactionReplayPolicy {
    /// Stop before callbacks; the caller decides how to continue compacted history.
    Reject,
    /// Keep the full append-only history and compaction block verbatim. Never
    /// manufacture a summary or silently discard messages before the boundary.
    PreserveHistory,
}

/// Finite execution limits. Start from `Default` and change individual fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolRunnerOptions {
    /// Maximum Messages API turns.
    pub max_turns: usize,
    /// Maximum callbacks started across the whole run.
    pub max_tool_calls: usize,
    /// Deadline for an individual execution hook or callback.
    pub call_timeout: Duration,
    /// Deadline for all HTTP, hook, and callback work.
    pub overall_timeout: Duration,
    /// Maximum serialized tool-result content bytes per call.
    pub max_result_bytes: usize,
    /// Maximum concurrently executing callbacks; defaults to one.
    pub max_parallel_calls: usize,
    /// Callback failure handling; defaults to returning a partial result.
    pub callback_errors: CallbackErrorPolicy,
    /// Unknown content replay requires an explicit preservation choice.
    pub unknown_content: ReplayUnknownPolicy,
    /// Explicit handling for server-produced compaction snapshots.
    pub compaction: CompactionReplayPolicy,
}
impl Default for ToolRunnerOptions {
    fn default() -> Self {
        Self {
            max_turns: 20,
            max_tool_calls: 100,
            call_timeout: Duration::from_secs(30),
            overall_timeout: Duration::from_secs(300),
            max_result_bytes: 1024 * 1024,
            max_parallel_calls: 1,
            callback_errors: CallbackErrorPolicy::Stop,
            unknown_content: ReplayUnknownPolicy::Reject,
            compaction: CompactionReplayPolicy::Reject,
        }
    }
}
impl ToolRunnerOptions {
    /// Reject zero or overflowing budgets before constructing a runner.
    pub fn validate(&self) -> Result<()> {
        if self.max_turns == 0
            || self.max_tool_calls == 0
            || self.call_timeout.is_zero()
            || self.overall_timeout.is_zero()
            || self.max_result_bytes == 0
            || self.max_parallel_calls == 0
        {
            return Err(AnthropicError::invalid_input(
                "tool-runner budgets must be nonzero",
            ));
        }
        if tokio::time::Instant::now()
            .checked_add(self.overall_timeout)
            .is_none()
            || tokio::time::Instant::now()
                .checked_add(self.call_timeout)
                .is_none()
        {
            return Err(AnthropicError::invalid_input(
                "tool-runner deadline exceeds supported duration",
            ));
        }
        Ok(())
    }
}

/// Cancellation shared with the caller. Cancelling drops active HTTP/callback
/// futures and prevents subsequent work; already completed side effects remain.
#[derive(Clone)]
pub struct ToolRunnerCancellation {
    signal: watch::Sender<bool>,
}
impl Default for ToolRunnerCancellation {
    fn default() -> Self {
        let (signal, _) = watch::channel(false);
        Self { signal }
    }
}
impl ToolRunnerCancellation {
    /// Create an uncancelled token.
    pub fn new() -> Self {
        Self::default()
    }
    /// Stop active and future work for runs using this token.
    pub fn cancel(&self) {
        self.signal.send_replace(true);
    }
    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        *self.signal.borrow()
    }
    async fn cancelled(&self) {
        let mut receiver = self.signal.subscribe();
        if *receiver.borrow() {
            return;
        }
        while receiver.changed().await.is_ok() {
            if *receiver.borrow() {
                return;
            }
        }
    }
}

/// Why a bounded tool run stopped. Details and usage remain in its result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolRunnerTermination {
    /// Natural completion.
    EndTurn,
    /// A configured stop sequence ended generation.
    StopSequence,
    /// The model refused.
    Refusal,
    /// The response hit its generation limit.
    MaxTokens,
    /// Server tools paused; automatic continuation is not attempted.
    PauseTurn,
    /// The context window was exhausted.
    ContextWindowExceeded,
    /// An unfamiliar reason is returned verbatim.
    UnknownStopReason(String),
    /// A complete response had no stop reason.
    MissingStopReason,
    /// Maximum model turns reached.
    TurnLimit,
    /// The next complete batch of calls would exceed the call limit.
    CallLimit,
    /// Caller cancellation.
    Cancelled,
    /// The whole-run deadline expired.
    OverallTimeout,
    /// Registered callback or execution-hook failure.
    CallbackFailed,
    /// Invalid or incompatible request configuration.
    InvalidRequest,
    /// A tool was unknown, duplicated, disabled, or failed input validation.
    InvalidToolCall,
    /// A response cannot be safely replayed under the selected policy.
    UnsupportedReplay,
    /// `tool_use` was returned without a client tool invocation.
    NoClientToolCalls,
    /// HTTP, response parsing, or incomplete SSE failed.
    TransportFailed,
    /// A callback produced content above the configured byte limit.
    ResultLimit,
}

/// A complete or partial run. Callback/transport failures do not discard state.
#[derive(Debug)]
#[non_exhaustive]
pub struct ToolRunnerResult {
    /// Recorded conversation, including the original request, complete assistant
    /// blocks, and every completed result. Interrupted runs may omit results for
    /// started calls; reconcile those IDs before resuming the conversation.
    pub messages: Vec<Message>,
    /// Complete received response snapshots, with original metadata and usage.
    pub responses: Vec<MessageResponse>,
    /// Per-turn usage, kept separate rather than merging incompatible metadata.
    pub usage: Vec<Usage>,
    /// IDs whose callbacks started. Interrupted callbacks may already have had
    /// external side effects, so callers must not blindly retry these IDs.
    pub started_calls: Vec<String>,
    /// IDs whose callbacks completed successfully or produced sanitized errors.
    pub completed_calls: Vec<String>,
    /// Stop cause.
    pub termination: ToolRunnerTermination,
    /// A local failure, never automatically copied into prompts or logs.
    pub error: Option<AnthropicError>,
}
impl ToolRunnerResult {
    /// Sum the authoritative per-turn token totals using wide counters. Thinking
    /// details and compaction iteration usage are not double-counted.
    pub fn total_tokens(&self) -> u64 {
        self.usage
            .iter()
            .map(|usage| {
                u64::from(usage.input_tokens)
                    + u64::from(usage.cache_creation_input_tokens)
                    + u64::from(usage.cache_read_input_tokens)
                    + u64::from(usage.output_tokens)
            })
            .sum()
    }
    /// The final received response, including a partial-run response if present.
    pub fn last_response(&self) -> Option<&MessageResponse> {
        self.responses.last()
    }
}

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
    async fn run_loop(
        &self,
        mut request: MessageRequest,
        options: Option<RequestOptions>,
        streaming: bool,
        result: &mut ToolRunnerResult,
        started: &Arc<Mutex<Vec<String>>>,
    ) -> ToolRunnerTermination {
        let mut seen_ids = request
            .messages
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect::<HashSet<_>>();
        let offered = request
            .tools
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|tool| tool.is_client())
            .map(|tool| tool.name.clone())
            .collect::<HashSet<_>>();
        for _ in 0..self.options.max_turns {
            if self.cancellation.is_cancelled() {
                return ToolRunnerTermination::Cancelled;
            }
            request.messages = result.messages.clone();
            let received = if streaming {
                match self
                    .client
                    .messages()
                    .create_stream(request.clone(), options.clone())
                    .await
                {
                    Ok(stream) => stream.collect_message().await,
                    Err(error) => Err(error),
                }
            } else {
                self.client
                    .messages()
                    .create(request.clone(), options.clone())
                    .await
            };
            let response = match received {
                Ok(response) => response,
                Err(error) => {
                    result.error = Some(error);
                    return ToolRunnerTermination::TransportFailed;
                }
            };
            result.usage.push(response.usage.clone());
            result
                .messages
                .push(Message::new(Role::Assistant, response.content.clone()));
            result.responses.push(response.clone());
            match response.stop_reason.as_ref() {
                Some(StopReason::ToolUse) => {}
                Some(StopReason::EndTurn) => return ToolRunnerTermination::EndTurn,
                Some(StopReason::StopSequence) => return ToolRunnerTermination::StopSequence,
                Some(StopReason::Refusal) => return ToolRunnerTermination::Refusal,
                Some(StopReason::MaxTokens) => return ToolRunnerTermination::MaxTokens,
                Some(StopReason::PauseTurn) => return ToolRunnerTermination::PauseTurn,
                Some(StopReason::ModelContextWindowExceeded) => {
                    return ToolRunnerTermination::ContextWindowExceeded
                }
                Some(StopReason::Unknown(reason)) => {
                    return ToolRunnerTermination::UnknownStopReason(reason.clone())
                }
                None => return ToolRunnerTermination::MissingStopReason,
            }
            if response
                .content
                .iter()
                .any(|block| matches!(block, ContentBlock::Compaction { .. }))
                && self.options.compaction == CompactionReplayPolicy::Reject
            {
                return ToolRunnerTermination::UnsupportedReplay;
            }
            if let Err(error) = response.to_conversation_message(self.options.unknown_content) {
                result.error = Some(error);
                return ToolRunnerTermination::UnsupportedReplay;
            }
            let calls = response
                .content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::ToolUse {
                        id, name, input, ..
                    } => Some(ToolCall {
                        id: id.clone(),
                        name: name.clone(),
                        input: input.clone(),
                    }),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if calls.is_empty() {
                return ToolRunnerTermination::NoClientToolCalls;
            }
            if calls.len()
                > self
                    .options
                    .max_tool_calls
                    .saturating_sub(started.lock().expect("tool-start ledger poisoned").len())
            {
                return ToolRunnerTermination::CallLimit;
            }
            let mut prepared = Vec::with_capacity(calls.len());
            for call in calls {
                if call.id.is_empty()
                    || !seen_ids.insert(call.id.clone())
                    || !offered.contains(&call.name)
                    || request
                        .tool_choice
                        .as_ref()
                        .is_some_and(|choice| choice.kind() == "none")
                    || request
                        .tool_choice
                        .as_ref()
                        .and_then(ToolChoice::forced_tool_name)
                        .is_some_and(|forced_name| forced_name != call.name)
                {
                    result.error = Some(AnthropicError::invalid_input(
                        "unknown, disabled, duplicate, or mismatched tool invocation",
                    ));
                    return ToolRunnerTermination::InvalidToolCall;
                }
                let tool = self
                    .registry
                    .tools
                    .get(&call.name)
                    .expect("offered registered tool")
                    .clone();
                if let Err(error) = (tool.validator)(&call.input) {
                    result.error = Some(error);
                    return ToolRunnerTermination::InvalidToolCall;
                }
                prepared.push((call, tool));
            }
            if let Some(hook) = &self.hook {
                for (call, _) in &prepared {
                    match tokio::time::timeout(self.options.call_timeout, hook(call.clone())).await
                    {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            result.error = Some(error);
                            return ToolRunnerTermination::CallbackFailed;
                        }
                        Err(_) => {
                            result.error = Some(AnthropicError::timeout(self.options.call_timeout));
                            return ToolRunnerTermination::CallbackFailed;
                        }
                    }
                }
            }
            let parallel = if request
                .tool_choice
                .as_ref()
                .and_then(ToolChoice::disable_parallel_tool_use)
                == Some(true)
            {
                1
            } else {
                self.options.max_parallel_calls
            };
            let execution = futures::stream::iter(prepared.into_iter().enumerate())
                .map(|(index, (call, tool))| {
                    let started = started.clone();
                    async move {
                        started
                            .lock()
                            .expect("tool-start ledger poisoned")
                            .push(call.id.clone());
                        let outcome = tokio::time::timeout(
                            self.options.call_timeout,
                            (tool.callback)(call.input),
                        )
                        .await
                        .unwrap_or_else(|_| {
                            Err(AnthropicError::timeout(self.options.call_timeout))
                        });
                        (index, call.id, self.make_result_block(outcome))
                    }
                })
                .buffer_unordered(parallel);
            tokio::pin!(execution);
            let mut blocks = Vec::new();
            let mut result_message_index = None;
            while let Some((index, id, outcome)) = execution.next().await {
                match outcome {
                    Ok(block) => {
                        result.completed_calls.push(id.clone());
                        blocks.push((index, attach_tool_id(block, id)));
                        blocks.sort_by_key(|(index, _)| *index);
                        let message_index = *result_message_index.get_or_insert_with(|| {
                            let index = result.messages.len();
                            result.messages.push(Message::new(Role::User, Vec::new()));
                            index
                        });
                        // Commit each completed result before awaiting another callback,
                        // so cancellation/deadlines preserve completed external work.
                        result.messages[message_index].content =
                            blocks.iter().map(|(_, block)| block.clone()).collect();
                    }
                    Err((termination, error)) => {
                        result.error = Some(error);
                        return termination;
                    }
                }
            }
        }
        ToolRunnerTermination::TurnLimit
    }
    fn make_result_block(
        &self,
        outcome: Result<ToolResultContent>,
    ) -> std::result::Result<ContentBlock, (ToolRunnerTermination, AnthropicError)> {
        let (content, is_error) = match outcome {
            Ok(ToolResultContent::Json(value)) => {
                (ToolResultContent::Text(value.to_string()), false)
            }
            Ok(content) => (content, false),
            Err(_) if self.options.callback_errors == CallbackErrorPolicy::ReturnSanitizedError => {
                (
                    ToolResultContent::Text("Tool execution failed".into()),
                    true,
                )
            }
            Err(error) => return Err((ToolRunnerTermination::CallbackFailed, error)),
        };
        let bytes = serde_json::to_vec(&content).map_err(|error| {
            (
                ToolRunnerTermination::UnsupportedReplay,
                AnthropicError::from(error),
            )
        })?;
        if bytes.len() > self.options.max_result_bytes {
            return Err((
                ToolRunnerTermination::ResultLimit,
                AnthropicError::invalid_input("tool result exceeds configured byte limit"),
            ));
        }
        let block = ContentBlock::ToolResult {
            tool_use_id: "pending".into(),
            content: Some(content),
            is_error: Some(is_error),
            extra: ExtraFields::new(),
        };
        block
            .checked_replay(&Role::User, self.options.unknown_content)
            .map_err(|error| (ToolRunnerTermination::UnsupportedReplay, error))
    }
}
fn attach_tool_id(mut block: ContentBlock, id: String) -> ContentBlock {
    if let ContentBlock::ToolResult { tool_use_id, .. } = &mut block {
        *tool_use_id = id;
    }
    block
}
