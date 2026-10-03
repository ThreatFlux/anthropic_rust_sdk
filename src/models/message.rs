//! Message-related data models

use super::common::{
    CacheControl, CacheCreationUsage, ContentBlock, Metadata, OutputTokensDetails, Role,
    ServerToolUsage, StopDetails, StopReason, TextCitation, Tool, ToolChoice, Usage, VecPush,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A message in a conversation
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Message role
    pub role: Role,
    /// Message content
    pub content: Vec<ContentBlock>,
    /// Message metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
}

impl Message {
    /// Create a new message
    pub fn new(role: Role, content: Vec<ContentBlock>) -> Self {
        Self {
            role,
            content,
            metadata: None,
        }
    }

    /// Create a user message with text
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, vec![ContentBlock::text(text)])
    }

    /// Create an assistant message with text
    pub fn assistant(text: impl Into<String>) -> Self {
        Self::new(Role::Assistant, vec![ContentBlock::text(text)])
    }

    /// Create a system message with text
    pub fn system(text: impl Into<String>) -> Self {
        Self::new(Role::System, vec![ContentBlock::text(text)])
    }

    /// Add metadata to the message
    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Add content block to the message
    pub fn add_content(mut self, content: ContentBlock) -> Self {
        self.content.push(content);
        self
    }

    /// Get the text content of the message (concatenated if multiple text blocks)
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Thinking configuration.
///
/// Current models (Opus 4.7 / 4.8, Fable 5) require **adaptive** thinking —
/// use [`ThinkingConfig::adaptive`]. `budget_tokens` (`"enabled"`) is deprecated
/// on Opus 4.6 / Sonnet 4.6 and returns a 400 on Opus 4.7 / 4.8 / Fable 5; it is
/// retained only for older models.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Type of thinking mode: `"adaptive"`, `"enabled"`, or `"disabled"`.
    #[serde(rename = "type")]
    pub thinking_type: String,
    /// Maximum tokens to allocate for thinking (`"enabled"`, legacy models only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
    /// Reasoning-summary visibility: `"summarized"` or `"omitted"` (adaptive).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// Allow tool use during thinking (beta; legacy field).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_tool_use: Option<bool>,
    /// Thinking block binding controls (beta; caller selects the beta header).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_binding: Option<serde_json::Value>,
    /// Future thinking controls retained during prompt projection.
    #[serde(default, flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ThinkingConfig {
    /// Create adaptive thinking configuration (recommended for current models).
    ///
    /// Claude decides when and how much to think. Pair with
    /// [`OutputConfig`]'s `effort` to control depth.
    pub fn adaptive() -> Self {
        Self {
            thinking_type: "adaptive".to_string(),
            budget_tokens: None,
            display: None,
            allow_tool_use: None,
            block_binding: None,
            extra: HashMap::new(),
        }
    }

    /// Think between tool calls (Sonnet 5.5; pair with low, medium, or high effort).
    pub fn between_tools() -> Self {
        Self {
            thinking_type: "between_tools".into(),
            budget_tokens: None,
            display: None,
            allow_tool_use: None,
            block_binding: None,
            extra: HashMap::new(),
        }
    }

    /// Validate the exact `between_tools` schema before sending it.
    pub fn validate_between_tools(&self) -> crate::error::Result<()> {
        if self.thinking_type == "between_tools"
            && (self.budget_tokens.is_some()
                || self.display.is_some()
                || self.allow_tool_use.is_some()
                || self.block_binding.is_some()
                || !self.extra.is_empty())
        {
            return Err(crate::error::AnthropicError::invalid_input("between_tools accepts only the type field; budget, display, binding and extra controls are unsupported"));
        }
        Ok(())
    }

    /// Set explicit beta thinking block binding controls.
    pub fn with_block_binding(mut self, binding: serde_json::Value) -> Self {
        self.block_binding = Some(binding);
        self
    }

    /// Adaptive thinking that returns a readable summary of the reasoning.
    pub fn adaptive_summarized() -> Self {
        Self {
            thinking_type: "adaptive".to_string(),
            budget_tokens: None,
            display: Some("summarized".to_string()),
            allow_tool_use: None,
            block_binding: None,
            extra: HashMap::new(),
        }
    }

    /// Set the reasoning-summary display mode (`"summarized"` / `"omitted"`).
    pub fn with_display(mut self, display: impl Into<String>) -> Self {
        self.display = Some(display.into());
        self
    }

    /// Create enabled (fixed-budget) thinking configuration.
    ///
    /// Deprecated on current models — prefer [`ThinkingConfig::adaptive`].
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
            display: None,
            allow_tool_use: None,
            block_binding: None,
            extra: HashMap::new(),
        }
    }

    /// Create enabled thinking configuration with tool use (legacy).
    pub fn enabled_with_tools(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
            display: None,
            allow_tool_use: Some(true),
            block_binding: None,
            extra: HashMap::new(),
        }
    }

    /// Create disabled thinking configuration.
    pub fn disabled() -> Self {
        Self {
            thinking_type: "disabled".to_string(),
            budget_tokens: None,
            display: None,
            allow_tool_use: None,
            block_binding: None,
            extra: HashMap::new(),
        }
    }
}

/// Output quality effort level.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OutputEffort {
    /// Lower effort / latency.
    Low,
    /// Medium effort / latency.
    Medium,
    /// High effort / latency (default).
    High,
    /// Extra-high effort (Opus 4.7+ / Fable 5) — best for coding/agentic work.
    XHigh,
    /// Maximum effort / latency (Opus 4.6+, Sonnet 4.6, Fable 5).
    Max,
}

/// Agentic task budget — a token target the model is aware of and self-moderates
/// against across a full tool-use loop (beta; Opus 4.7+ / Fable 5). Distinct from
/// `max_tokens`, which is an enforced per-response ceiling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskBudget {
    /// Budget type (always `"tokens"`).
    #[serde(rename = "type")]
    pub budget_type: String,
    /// Total token budget for the loop (minimum 20,000).
    pub total: u32,
}

impl TaskBudget {
    /// Create a token task budget.
    pub fn tokens(total: u32) -> Self {
        Self {
            budget_type: "tokens".to_string(),
            total,
        }
    }
}

/// Output format configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputFormat {
    /// Structured JSON output with a JSON Schema.
    JsonSchema { schema: serde_json::Value },
}

impl OutputFormat {
    /// Create a JSON-schema output format.
    pub fn json_schema(schema: serde_json::Value) -> Self {
        Self::JsonSchema { schema }
    }
}

/// Output configuration for generated responses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct OutputConfig {
    /// Model effort level for response generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<OutputEffort>,
    /// Structured output format settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
    /// Agentic task budget (beta).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_budget: Option<TaskBudget>,
}

impl OutputConfig {
    /// Create a new empty output configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set output effort.
    pub fn with_effort(mut self, effort: OutputEffort) -> Self {
        self.effort = Some(effort);
        self
    }

    /// Set output format.
    pub fn with_format(mut self, format: OutputFormat) -> Self {
        self.format = Some(format);
        self
    }

    /// Set an agentic task budget (in tokens).
    pub fn with_task_budget(mut self, total_tokens: u32) -> Self {
        self.task_budget = Some(TaskBudget::tokens(total_tokens));
        self
    }

    /// Create a configuration for JSON-schema constrained output.
    pub fn json_schema(schema: serde_json::Value) -> Self {
        Self::new().with_format(OutputFormat::json_schema(schema))
    }
}

/// A system-prompt text block, which may carry a cache-control breakpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemBlock {
    /// Block type (always `"text"`).
    #[serde(rename = "type")]
    pub block_type: String,
    /// Block text.
    pub text: String,
    /// Cache control breakpoint for prompt caching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// Create a plain system text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: text.into(),
            cache_control: None,
        }
    }

    /// Create a system text block with an ephemeral cache breakpoint.
    pub fn cached(text: impl Into<String>) -> Self {
        Self::text(text).with_cache_control(CacheControl::ephemeral())
    }

    /// Attach a cache-control breakpoint to this block.
    pub fn with_cache_control(mut self, cache_control: CacheControl) -> Self {
        self.cache_control = Some(cache_control);
        self
    }
}

/// System prompt: a plain string or an array of cacheable text blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Plain-text system prompt.
    Text(String),
    /// Structured system prompt with per-block cache control.
    Blocks(Vec<SystemBlock>),
}

impl From<String> for SystemPrompt {
    fn from(text: String) -> Self {
        Self::Text(text)
    }
}

impl From<&str> for SystemPrompt {
    fn from(text: &str) -> Self {
        Self::Text(text.to_string())
    }
}

impl From<Vec<SystemBlock>> for SystemPrompt {
    fn from(blocks: Vec<SystemBlock>) -> Self {
        Self::Blocks(blocks)
    }
}

/// A refusal-fallback model entry for the server-side `fallbacks` parameter
/// (Claude Fable 5). On a policy decline the API re-serves the request on the
/// fallback model in the same call. Requires the `server-side-fallback-2026-06-01`
/// beta header (see [`crate::types::RequestOptions::with_server_side_fallback`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Fallback {
    /// Fallback model id (e.g. `claude-opus-4-8`).
    pub model: String,
    /// Optional per-hop `max_tokens` override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Optional output settings for this fallback hop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    /// Optional thinking configuration for this fallback hop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Optional inference speed for this fallback hop.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// A fallback credit token can be sent as a bare token or with a redemption
/// mode on the current fallback-credit beta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FallbackCreditToken {
    /// The legacy/strict bare-token form.
    Token(String),
    /// Token with an explicit redemption mode (`strict` or `best_effort`).
    Config {
        token: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        mode: Option<String>,
    },
}

impl FallbackCreditToken {
    /// Use the strict bare-token form.
    pub fn new(token: impl Into<String>) -> Self {
        Self::Token(token.into())
    }

    /// Use a token with an explicit redemption mode.
    pub fn with_mode(token: impl Into<String>, mode: impl Into<String>) -> Self {
        Self::Config {
            token: token.into(),
            mode: Some(mode.into()),
        }
    }
}

impl Fallback {
    /// Create a fallback entry for the given model.
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            max_tokens: None,
            output_config: None,
            thinking: None,
            speed: None,
        }
    }
}

/// Server-side fallback selection. The string form `"default"` asks Anthropic
/// to choose the fallback chain, while the array form supplies explicit hops.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Fallbacks {
    /// Explicit fallback model entries.
    Models(Vec<Fallback>),
    /// Anthropic-managed fallback chain (currently `"default"`).
    Default(String),
}

impl Fallbacks {
    /// Use Anthropic's default fallback chain.
    pub fn default_model_chain() -> Self {
        Self::Default("default".to_string())
    }

    /// Create an explicit fallback chain.
    pub fn models(models: Vec<Fallback>) -> Self {
        Self::Models(models)
    }
}

/// Request to create a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageRequest {
    /// Model to use for the message
    pub model: String,
    /// Maximum number of tokens to generate
    pub max_tokens: u32,
    /// List of messages in the conversation
    pub messages: Vec<Message>,
    /// System prompt (string or cacheable text blocks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    /// Sampling temperature (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling parameter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Custom stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Whether to stream the response
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// Tools available for the model to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Tool choice preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Extended thinking configuration (Claude 4 models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Request metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,
    /// Service tier selection (e.g. `auto`, `standard_only`)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// Inference geography routing preference
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// Output configuration (structured outputs and effort settings)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    /// Output speed preference (`standard` or `fast`, where supported).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    /// Structured output format shortcut.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,
    /// Credit token returned by a previous server-side fallback response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_credit_token: Option<FallbackCreditToken>,
    /// Optional diagnostic controls for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<serde_json::Value>,
    /// User profile attribution for this request.
    #[serde(skip)]
    pub user_profile_id: Option<String>,
    /// Reusable execution container configuration (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<serde_json::Value>,
    /// Context management configuration (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_management: Option<serde_json::Value>,
    /// MCP server configuration list (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<serde_json::Value>>,
    /// Top-level cache control — auto-caches the last cacheable block.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    /// Refusal-fallback models (beta; Claude Fable 5).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallbacks: Option<Fallbacks>,
}

impl MessageRequest {
    /// Create a new message request
    pub fn new() -> Self {
        Self {
            model: crate::config::DEFAULT_MODEL.to_string(),
            max_tokens: 1000,
            messages: Vec::new(),
            system: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            stream: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            metadata: None,
            service_tier: None,
            inference_geo: None,
            output_config: None,
            speed: None,
            output_format: None,
            fallback_credit_token: None,
            diagnostics: None,
            user_profile_id: None,
            container: None,
            context_management: None,
            mcp_servers: None,
            cache_control: None,
            fallbacks: None,
        }
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = max_tokens;
        self
    }

    /// Set a plain-text system prompt
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(SystemPrompt::Text(system.into()));
        self
    }

    /// Set a structured system prompt from cacheable text blocks
    pub fn system_blocks(mut self, blocks: Vec<SystemBlock>) -> Self {
        self.system = Some(SystemPrompt::Blocks(blocks));
        self
    }

    /// Set a system prompt as a single cached (ephemeral) text block
    pub fn system_cached(mut self, system: impl Into<String>) -> Self {
        self.system = Some(SystemPrompt::Blocks(vec![SystemBlock::cached(system)]));
        self
    }

    /// Set the system prompt directly
    pub fn system_prompt(mut self, system: SystemPrompt) -> Self {
        self.system = Some(system);
        self
    }

    /// Set a top-level cache-control breakpoint (auto-caches the last block)
    pub fn cache_control(mut self, cache_control: CacheControl) -> Self {
        self.cache_control = Some(cache_control);
        self
    }

    /// Enable automatic prompt caching of the last cacheable block
    pub fn auto_cache(mut self) -> Self {
        self.cache_control = Some(CacheControl::ephemeral());
        self
    }

    /// Replace the refusal-fallback model list
    pub fn fallbacks(mut self, fallbacks: Vec<Fallback>) -> Self {
        self.fallbacks = Some(Fallbacks::Models(fallbacks));
        self
    }

    /// Use Anthropic's default server-side fallback chain.
    pub fn default_fallbacks(mut self) -> Self {
        self.fallbacks = Some(Fallbacks::default_model_chain());
        self
    }

    /// Add a refusal-fallback model
    pub fn add_fallback(mut self, model: impl Into<String>) -> Self {
        match self.fallbacks.take() {
            Some(Fallbacks::Models(mut fallbacks)) => {
                fallbacks.push(Fallback::new(model));
                self.fallbacks = Some(Fallbacks::Models(fallbacks));
            }
            _ => self.fallbacks = Some(Fallbacks::Models(vec![Fallback::new(model)])),
        }
        self
    }

    /// Set temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set top-k
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Add a stop sequence
    pub fn add_stop_sequence(mut self, stop: impl Into<String>) -> Self {
        self.stop_sequences.push_item(stop.into());
        self
    }

    /// Enable/disable streaming
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    /// Add a message
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a user message
    pub fn add_user_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::user(text));
        self
    }

    /// Add an assistant message
    pub fn add_assistant_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::assistant(text));
        self
    }

    /// Add a tool
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push_item(tool);
        self
    }

    /// Set tool choice
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Set service tier
    pub fn service_tier(mut self, tier: impl Into<String>) -> Self {
        self.service_tier = Some(tier.into());
        self
    }

    /// Set inference geography preference
    pub fn inference_geo(mut self, inference_geo: impl Into<String>) -> Self {
        self.inference_geo = Some(inference_geo.into());
        self
    }

    /// Set output config.
    pub fn output_config(mut self, output_config: OutputConfig) -> Self {
        self.output_config = Some(output_config);
        self
    }

    /// Set the output speed preference.
    pub fn speed(mut self, speed: impl Into<String>) -> Self {
        self.speed = Some(speed.into());
        self
    }

    /// Set the structured output format shortcut.
    pub fn output_format(mut self, format: OutputFormat) -> Self {
        self.output_format = Some(format);
        self
    }

    /// Set a server-side fallback credit token.
    pub fn fallback_credit_token(mut self, token: impl Into<String>) -> Self {
        self.fallback_credit_token = Some(FallbackCreditToken::new(token));
        self
    }

    /// Set a fallback credit token with an explicit redemption mode.
    pub fn fallback_credit_token_with_mode(
        mut self,
        token: impl Into<String>,
        mode: impl Into<String>,
    ) -> Self {
        self.fallback_credit_token = Some(FallbackCreditToken::with_mode(token, mode));
        self
    }

    /// Set diagnostic request controls as raw JSON.
    pub fn diagnostics(mut self, diagnostics: serde_json::Value) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    /// Attribute the request to a user profile.
    pub fn user_profile_id(mut self, profile_id: impl Into<String>) -> Self {
        self.user_profile_id = Some(profile_id.into());
        self
    }

    /// Configure JSON-schema constrained output.
    pub fn output_json_schema(mut self, schema: serde_json::Value) -> Self {
        self.output_config = Some(OutputConfig::json_schema(schema));
        self
    }

    /// Set container configuration as raw JSON
    pub fn container(mut self, container: serde_json::Value) -> Self {
        self.container = Some(container);
        self
    }

    /// Set context management configuration as raw JSON
    pub fn context_management(mut self, context_management: serde_json::Value) -> Self {
        self.context_management = Some(context_management);
        self
    }

    /// Replace MCP servers list with raw JSON objects
    pub fn mcp_servers(mut self, mcp_servers: Vec<serde_json::Value>) -> Self {
        self.mcp_servers = Some(mcp_servers);
        self
    }

    /// Add a single MCP server config object
    pub fn add_mcp_server(mut self, mcp_server: serde_json::Value) -> Self {
        self.mcp_servers.push_item(mcp_server);
        self
    }

    /// Enable adaptive thinking (recommended for current models)
    pub fn adaptive_thinking(mut self) -> Self {
        self.thinking = Some(ThinkingConfig::adaptive());
        self
    }

    /// Enable adaptive thinking with a summarized reasoning display
    pub fn adaptive_thinking_summarized(mut self) -> Self {
        self.thinking = Some(ThinkingConfig::adaptive_summarized());
        self
    }

    /// Enable fixed-budget extended thinking (legacy models only)
    pub fn thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking = Some(ThinkingConfig::enabled(budget_tokens));
        self
    }

    /// Enable extended thinking mode with tool use (Claude 4 models)
    pub fn thinking_with_tools(mut self, budget_tokens: u32) -> Self {
        self.thinking = Some(ThinkingConfig::enabled_with_tools(budget_tokens));
        self
    }

    /// Set custom thinking configuration
    pub fn thinking_config(mut self, config: ThinkingConfig) -> Self {
        self.thinking = Some(config);
        self
    }
}

impl Default for MessageRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from creating a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Unique identifier for the message
    pub id: String,
    /// Object type (always "message")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Role of the message (always "assistant" for responses)
    pub role: Role,
    /// Content blocks in the response
    pub content: Vec<ContentBlock>,
    /// Model used for the response
    pub model: String,
    /// Reason the message stopped
    pub stop_reason: Option<StopReason>,
    /// Stop sequence that caused the message to stop
    pub stop_sequence: Option<String>,
    /// Structured stop details (populated on `refusal`)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_details: Option<StopDetails>,
    /// Token usage information
    pub usage: Usage,
    /// Reusable execution container info (code execution; beta)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container: Option<serde_json::Value>,
    /// Diagnostic response metadata, when requested.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<serde_json::Value>,
    /// Applied context-management edits (beta).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_management: Option<serde_json::Value>,
    /// Input transformations applied to this response (beta).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_transformations: Option<serde_json::Value>,
    /// Additional response fields retained for forward compatibility.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
    /// When the message was created (synthesized if absent from the response)
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
}

impl MessageResponse {
    /// Construct an assistant response snapshot with empty content and metadata.
    pub fn new(id: impl Into<String>, model: impl Into<String>, usage: Usage) -> Self {
        Self {
            id: id.into(),
            object_type: "message".into(),
            role: Role::Assistant,
            content: Vec::new(),
            model: model.into(),
            stop_reason: None,
            stop_sequence: None,
            stop_details: None,
            usage,
            container: None,
            diagnostics: None,
            context_management: None,
            input_transformations: None,
            extra: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Preserve response content as an assistant conversation turn, validating
    /// known replay shapes and applying an explicit unknown-block policy.
    pub fn to_conversation_message(
        &self,
        policy: super::common::ReplayUnknownPolicy,
    ) -> crate::error::Result<Message> {
        if self.role != Role::Assistant {
            return Err(crate::error::AnthropicError::invalid_input(
                "Conversation replay requires an assistant response role",
            ));
        }
        let content = self
            .content
            .iter()
            .map(|block| block.checked_replay(&Role::Assistant, policy))
            .collect::<crate::error::Result<Vec<_>>>()?;
        Ok(Message::new(Role::Assistant, content))
    }

    /// Whether the response was declined for safety/policy reasons.
    pub fn is_refusal(&self) -> bool {
        matches!(self.stop_reason, Some(StopReason::Refusal))
    }
}

impl MessageResponse {
    /// Get the text content of the response
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|c| c.as_text())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Prompt configuration accepted by both Messages and token counting.
///
/// Fields stay flat on the wire; this is a reusable construction/projection type.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct PromptOptions {
    /// System prompt, including cacheable blocks.
    pub system: Option<SystemPrompt>,
    /// Available tools.
    pub tools: Option<Vec<Tool>>,
    /// Thinking configuration.
    pub thinking: Option<ThinkingConfig>,
    /// Tool selection policy.
    pub tool_choice: Option<ToolChoice>,
    /// Output configuration, including structured output schemas.
    pub output_config: Option<OutputConfig>,
    /// Automatic prompt cache breakpoint.
    pub cache_control: Option<CacheControl>,
    /// Profile attribution, sent as a header rather than JSON.
    pub user_profile_id: Option<String>,
}

impl PromptOptions {
    /// Extract the countable prompt configuration without validating parity.
    /// Use [`TokenCountRequest::from_message`] for validated projection.
    pub fn from_message(request: &MessageRequest) -> Self {
        Self {
            system: request.system.clone(),
            tools: request.tools.clone(),
            thinking: request.thinking.clone(),
            tool_choice: request.tool_choice.clone(),
            output_config: request.output_config.clone(),
            cache_control: request.cache_control.clone(),
            user_profile_id: request.user_profile_id.clone(),
        }
    }

    /// Apply these fields to a generation request.
    pub fn apply_to_message(self, request: &mut MessageRequest) {
        request.system = self.system;
        request.tools = self.tools;
        request.thinking = self.thinking;
        request.tool_choice = self.tool_choice;
        request.output_config = self.output_config;
        request.cache_control = self.cache_control;
        request.user_profile_id = self.user_profile_id;
    }
}

/// Request to count tokens in a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenCountRequest {
    /// Model to use for token counting
    pub model: String,
    /// Messages to count tokens for
    pub messages: Vec<Message>,
    /// System prompt to include in token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,
    /// Tools to include in token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
    /// Thinking configuration to include in counting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
    /// Tool selection policy to include in counting.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Output configuration, including structured output schemas.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
    /// Top-level automatic prompt caching.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    /// User profile attribution, sent as the Anthropic profile header.
    #[serde(skip)]
    pub user_profile_id: Option<String>,
}

impl TokenCountRequest {
    /// Project a message request into the stable counting endpoint's allowlist.
    ///
    /// Generation controls are omitted. Context management, containers, MCP server
    /// configuration and fallback routing can change the effective prompt but are
    /// not exposed by this counting schema, so projection fails rather than
    /// claiming an equivalent count. Nested beta thinking/output controls are
    /// retained verbatim and require the same explicit beta selection on both
    /// calls. Caller beta/workspace options and model/account eligibility remain
    /// explicit; projection does not select headers or verify account access.
    ///
    /// Verified 2026-10-03 against the official SDK's [stable counting schema](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/message_count_tokens_params.py)
    /// and [beta counting schema](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/beta/message_count_tokens_params.py).
    pub fn from_message(request: &MessageRequest) -> crate::error::Result<Self> {
        use crate::error::AnthropicError;
        for (present, field) in [
            (request.container.is_some(), "container"),
            (request.context_management.is_some(), "context_management"),
            (request.mcp_servers.is_some(), "mcp_servers"),
            (request.fallbacks.is_some(), "fallbacks"),
            (
                request.fallback_credit_token.is_some(),
                "fallback_credit_token",
            ),
        ] {
            if present {
                return Err(AnthropicError::invalid_input(format!(
                    "Cannot project {field} to the stable token-counting schema"
                )));
            }
        }
        let mut prompt = PromptOptions::from_message(request);
        if let Some(format) = &request.output_format {
            let config = prompt
                .output_config
                .get_or_insert_with(OutputConfig::default);
            if config
                .format
                .as_ref()
                .is_some_and(|existing| existing != format)
            {
                return Err(AnthropicError::invalid_input(
                    "Conflicting output_format and output_config.format",
                ));
            }
            config.format = Some(format.clone());
        }
        Ok(Self::new()
            .model(request.model.clone())
            .messages(request.messages.clone())
            .prompt_options(prompt))
    }

    /// Apply shared countable prompt options.
    pub fn prompt_options(mut self, prompt: PromptOptions) -> Self {
        self.system = prompt.system;
        self.tools = prompt.tools;
        self.thinking = prompt.thinking;
        self.tool_choice = prompt.tool_choice;
        self.output_config = prompt.output_config;
        self.cache_control = prompt.cache_control;
        self.user_profile_id = prompt.user_profile_id;
        self
    }

    /// Replace messages to count.
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }
    /// Replace tools to count.
    pub fn tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }
    /// Set structured system prompt blocks.
    pub fn system_blocks(mut self, blocks: Vec<SystemBlock>) -> Self {
        self.system = Some(SystemPrompt::Blocks(blocks));
        self
    }
    /// Set the thinking configuration.
    pub fn thinking(mut self, thinking: ThinkingConfig) -> Self {
        self.thinking = Some(thinking);
        self
    }
    /// Set tool selection.
    pub fn tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }
    /// Set output configuration.
    pub fn output_config(mut self, output_config: OutputConfig) -> Self {
        self.output_config = Some(output_config);
        self
    }
    /// Set automatic prompt caching.
    pub fn cache_control(mut self, cache_control: CacheControl) -> Self {
        self.cache_control = Some(cache_control);
        self
    }

    /// Create a new token count request
    pub fn new() -> Self {
        Self {
            model: crate::config::DEFAULT_MODEL.to_string(),
            messages: Vec::new(),
            system: None,
            tools: None,
            thinking: None,
            tool_choice: None,
            output_config: None,
            cache_control: None,
            user_profile_id: None,
        }
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Add a message
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }

    /// Add a user message
    pub fn add_user_message(mut self, text: impl Into<String>) -> Self {
        self.messages.push(Message::user(text));
        self
    }

    /// Set system prompt
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(SystemPrompt::Text(system.into()));
        self
    }

    /// Add a tool
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push_item(tool);
        self
    }

    /// Attribute token counting to a user profile.
    pub fn user_profile_id(mut self, profile_id: impl Into<String>) -> Self {
        self.user_profile_id = Some(profile_id.into());
        self
    }
}

impl Default for TokenCountRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from counting tokens
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCountResponse {
    /// Number of input tokens
    pub input_tokens: u32,
}

/// A streaming field that distinguishes omission from an explicit JSON null.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FieldUpdate<T> {
    /// Field was absent; retain the previous value.
    #[default]
    Missing,
    /// Field was explicitly null.
    Null,
    /// Field has a replacement value.
    Value(T),
}

impl<T> FieldUpdate<T> {
    /// Whether the field is absent.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }
    /// Whether a non-null value is present.
    pub fn is_some(&self) -> bool {
        matches!(self, Self::Value(_))
    }
    /// Whether no non-null value is present.
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }
    /// Borrow the non-null replacement value.
    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            _ => None,
        }
    }
    /// Apply this update, including explicit null clearing.
    pub fn apply(self, target: &mut Option<T>) {
        match self {
            Self::Missing => {}
            Self::Null => *target = None,
            Self::Value(value) => *target = Some(value),
        }
    }
    /// Apply a non-null replacement; null and omission both retain.
    pub fn apply_non_null(self, target: &mut Option<T>) {
        if let Self::Value(value) = self {
            *target = Some(value);
        }
    }
}

impl<T: Serialize> Serialize for FieldUpdate<T> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Missing | Self::Null => serializer.serialize_none(),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}
impl<'de, T: Deserialize<'de>> Deserialize<'de> for FieldUpdate<T> {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

/// Streaming message delta. Stops clear on explicit null; containers retain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageDelta {
    /// Stop reason update.
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    pub stop_reason: FieldUpdate<StopReason>,
    /// Stop sequence update.
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    pub stop_sequence: FieldUpdate<String>,
    /// Structured stop detail update.
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    pub stop_details: FieldUpdate<StopDetails>,
    /// Container snapshot update (null retains the previous container).
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    pub container: FieldUpdate<serde_json::Value>,
    /// Additional delta fields retained as raw data, without guessed merge rules.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Cumulative usage updates. Omitted or null fields retain the start snapshot.
///
/// Token counts replace previous totals, including zero. They are never added.
/// Verified 2026-10-03 against the official [stable](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/lib/streaming/_messages.py)
/// and [beta](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/lib/streaming/_beta_messages.py)
/// accumulators; start-only metadata has no inferred aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UsageDelta {
    /// Cumulative uncached input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// Cumulative output tokens, including thinking tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    /// Cumulative tokens written into cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Cumulative tokens read from cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
    /// Complete server-tool usage replacement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUsage>,
    /// Complete output token detail replacement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// Complete iteration usage replacement; an empty list replaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iterations: Option<Vec<serde_json::Value>>,
    /// Complete fallback-credit replacement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_credit: Option<serde_json::Value>,
    /// Start-only cache creation metadata, retained here for raw event inspection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreationUsage>,
    /// Start-only geography, retained here for raw event inspection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// Start-only service tier, retained here for raw event inspection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// Additional raw usage fields; collection does not guess their aggregation.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl UsageDelta {
    /// Apply the documented cumulative updates while retaining start-only metadata.
    pub fn apply(self, usage: &mut Usage) {
        if let Some(value) = self.input_tokens {
            usage.input_tokens = value;
        }
        if let Some(value) = self.output_tokens {
            usage.output_tokens = value;
        }
        if let Some(value) = self.cache_creation_input_tokens {
            usage.cache_creation_input_tokens = value;
        }
        if let Some(value) = self.cache_read_input_tokens {
            usage.cache_read_input_tokens = value;
        }
        if self.server_tool_use.is_some() {
            usage.server_tool_use = self.server_tool_use;
        }
        if self.output_tokens_details.is_some() {
            usage.output_tokens_details = self.output_tokens_details;
        }
        if self.iterations.is_some() {
            usage.iterations = self.iterations;
        }
        if self.fallback_credit.is_some() {
            usage.fallback_credit = self.fallback_credit;
        }
    }
}

/// Content block delta for streaming
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ContentBlockDelta {
    /// Type of content block
    #[serde(rename = "type")]
    pub block_type: String,
    /// Text delta (for text blocks)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Partial JSON delta (for tool/server tool input streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_json: Option<String>,
    /// Thinking text delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// Signature delta for thinking blocks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
    /// Citation delta (for text citations during streaming)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<TextCitation>,
    /// Compaction content snapshot; explicit null clears it.
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    pub content: FieldUpdate<String>,
    /// Opaque encrypted compaction snapshot; explicit null clears it.
    #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
    pub encrypted_content: FieldUpdate<String>,
    /// Additional delta fields for forward compatibility.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn take_delta_field<T: serde::de::DeserializeOwned>(
    extra: &mut HashMap<String, serde_json::Value>,
    key: &str,
) -> std::result::Result<Option<T>, serde_json::Error> {
    extra
        .remove(key)
        .map(serde_json::from_value::<Option<T>>)
        .transpose()
        .map(Option::flatten)
}
fn take_delta_update<T: serde::de::DeserializeOwned>(
    extra: &mut HashMap<String, serde_json::Value>,
    key: &str,
) -> std::result::Result<FieldUpdate<T>, serde_json::Error> {
    extra
        .remove(key)
        .map(serde_json::from_value)
        .transpose()
        .map(|value| value.unwrap_or(FieldUpdate::Missing))
}
impl<'de> Deserialize<'de> for ContentBlockDelta {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let serde_json::Value::Object(mut object) = value else {
            return Err(serde::de::Error::custom(
                "content delta requires an object with a string type",
            ));
        };
        let block_type = object
            .remove("type")
            .and_then(|kind| kind.as_str().map(str::to_owned))
            .ok_or_else(|| {
                serde::de::Error::custom("content delta requires an object with a string type")
            })?;
        let mut delta = Self {
            block_type,
            text: None,
            partial_json: None,
            thinking: None,
            signature: None,
            citation: None,
            content: FieldUpdate::Missing,
            encrypted_content: FieldUpdate::Missing,
            extra: object.into_iter().collect(),
        };
        // Only fields belonging to this discriminator are typed. Future keys,
        // even ones used by another known delta, retain their original JSON type.
        match delta.block_type.as_str() {
            "text_delta" => {
                delta.text =
                    take_delta_field(&mut delta.extra, "text").map_err(serde::de::Error::custom)?
            }
            "thinking_delta" => {
                delta.thinking = take_delta_field(&mut delta.extra, "thinking")
                    .map_err(serde::de::Error::custom)?
            }
            "signature_delta" => {
                delta.signature = take_delta_field(&mut delta.extra, "signature")
                    .map_err(serde::de::Error::custom)?
            }
            "input_json_delta" => {
                delta.partial_json = take_delta_field(&mut delta.extra, "partial_json")
                    .map_err(serde::de::Error::custom)?
            }
            "citations_delta" => {
                delta.citation = take_delta_field(&mut delta.extra, "citation")
                    .map_err(serde::de::Error::custom)?
            }
            "compaction_delta" => {
                delta.content = take_delta_update(&mut delta.extra, "content")
                    .map_err(serde::de::Error::custom)?;
                delta.encrypted_content = take_delta_update(&mut delta.extra, "encrypted_content")
                    .map_err(serde::de::Error::custom)?;
            }
            _ => {}
        }
        delta.validate().map_err(serde::de::Error::custom)?;
        Ok(delta)
    }
}

impl ContentBlockDelta {
    /// Reject malformed recognized deltas while preserving future delta types.
    pub fn validate(&self) -> crate::error::Result<()> {
        let valid = match self.block_type.as_str() {
            "text_delta" => self.text.is_some(),
            "thinking_delta" => self.thinking.is_some(),
            "signature_delta" => self.signature.is_some(),
            "input_json_delta" => self.partial_json.is_some(),
            "citations_delta" => self.citation.is_some(),
            "compaction_delta" => true,
            _ => true,
        };
        if !valid {
            return Err(crate::error::AnthropicError::stream(format!(
                "Missing required field in {}",
                self.block_type
            )));
        }
        Ok(())
    }
}

/// Streaming event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
#[non_exhaustive]
pub enum StreamEvent {
    /// Message started
    MessageStart { message: MessageResponse },
    /// Message delta
    MessageDelta {
        /// Message-level stop/container updates.
        delta: MessageDelta,
        /// Cumulative usage updates.
        #[serde(default)]
        usage: UsageDelta,
        /// Beta event-level context management snapshot.
        #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
        context_management: FieldUpdate<serde_json::Value>,
        /// Beta event-level input transformations snapshot.
        #[serde(default, skip_serializing_if = "FieldUpdate::is_missing")]
        input_transformations: FieldUpdate<serde_json::Value>,
        /// Unrecognized event-level fields.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// Message stopped
    MessageStop,
    /// Content block started
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },
    /// Content block delta
    ContentBlockDelta {
        index: usize,
        delta: ContentBlockDelta,
    },
    /// Content block stopped
    ContentBlockStop { index: usize },
    /// Ping event
    Ping,
    /// Unknown SSE event retained without assuming a JSON schema or merge rule.
    Unknown {
        /// Event name supplied by the server.
        event_type: String,
        /// Exact newline-joined SSE data payload.
        data: String,
    },
    /// Error event
    Error {
        error: HashMap<String, serde_json::Value>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_output_config_json_schema_serialization() {
        let request = MessageRequest::new()
            .add_user_message("Return JSON")
            .output_json_schema(json!({
                "type": "object",
                "properties": {
                    "answer": { "type": "string" }
                },
                "required": ["answer"],
                "additionalProperties": false
            }));

        let value = serde_json::to_value(request).unwrap();
        assert_eq!(value["output_config"]["format"]["type"], "json_schema");
        assert_eq!(
            value["output_config"]["format"]["schema"]["properties"]["answer"]["type"],
            "string"
        );
    }

    #[test]
    fn test_output_config_effort_serialization() {
        let request = MessageRequest::new()
            .add_user_message("Summarize this")
            .output_config(OutputConfig::new().with_effort(OutputEffort::High));
        let value = serde_json::to_value(request).unwrap();

        assert_eq!(value["output_config"]["effort"], "high");
    }

    #[test]
    fn test_content_block_delta_with_citation() {
        let delta: ContentBlockDelta = serde_json::from_value(json!({
            "type": "citations_delta",
            "citation": {
                "type": "search_result_location",
                "search_result_index": 0,
                "source": "web_search",
                "title": "Example",
                "cited_text": "snippet"
            }
        }))
        .unwrap();

        assert!(delta.citation.is_some());
    }

    #[test]
    fn test_adaptive_thinking_serialization() {
        let request = MessageRequest::new()
            .model("claude-opus-4-8")
            .add_user_message("hi")
            .adaptive_thinking_summarized();
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["thinking"]["type"], "adaptive");
        assert_eq!(value["thinking"]["display"], "summarized");
        // budget_tokens must NOT be present for adaptive thinking.
        assert!(value["thinking"].get("budget_tokens").is_none());
    }

    #[test]
    fn test_effort_xhigh_and_task_budget_serialization() {
        let request = MessageRequest::new()
            .add_user_message("code")
            .output_config(
                OutputConfig::new()
                    .with_effort(OutputEffort::XHigh)
                    .with_task_budget(128_000),
            );
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["output_config"]["effort"], "xhigh");
        assert_eq!(value["output_config"]["task_budget"]["type"], "tokens");
        assert_eq!(value["output_config"]["task_budget"]["total"], 128_000);
    }

    #[test]
    fn test_system_cached_serializes_as_blocks() {
        let request = MessageRequest::new()
            .add_user_message("q")
            .system_cached("large shared prompt");
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["system"][0]["type"], "text");
        assert_eq!(value["system"][0]["text"], "large shared prompt");
        assert_eq!(value["system"][0]["cache_control"]["type"], "ephemeral");

        // Plain string system still serializes as a bare string.
        let plain = MessageRequest::new()
            .add_user_message("q")
            .system("you are helpful");
        let plain_value = serde_json::to_value(&plain).unwrap();
        assert_eq!(plain_value["system"], "you are helpful");
    }

    #[test]
    fn test_top_level_cache_control_and_fallbacks() {
        let request = MessageRequest::new()
            .model("claude-fable-5")
            .add_user_message("q")
            .auto_cache()
            .add_fallback("claude-opus-4-8");
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["cache_control"]["type"], "ephemeral");
        assert_eq!(value["fallbacks"][0]["model"], "claude-opus-4-8");
    }

    #[test]
    fn test_message_response_without_created_at_and_refusal() {
        // Real Messages API responses do not include `created_at` and may carry
        // a structured `stop_details` on refusal.
        let response: MessageResponse = serde_json::from_value(json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-fable-5",
            "content": [],
            "stop_reason": "refusal",
            "stop_details": {"type": "refusal", "category": "cyber"},
            "usage": {"input_tokens": 3, "output_tokens": 0}
        }))
        .unwrap();
        assert!(response.is_refusal());
        assert_eq!(
            response.stop_details.unwrap().category.as_deref(),
            Some("cyber")
        );
    }
}
