//! Message-related data models

use super::common::{
    ContentBlock, Metadata, Role, StopReason, TextCitation, Tool, ToolChoice, Usage, VecPush,
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

/// Extended thinking configuration for Claude 4 models
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThinkingConfig {
    /// Type of thinking mode ("enabled" or "disabled")
    #[serde(rename = "type")]
    pub thinking_type: String,
    /// Maximum tokens to allocate for thinking (up to 64000 for Claude 4)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
    /// Allow tool use during thinking (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_tool_use: Option<bool>,
}

impl ThinkingConfig {
    /// Create enabled thinking configuration
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
            allow_tool_use: None,
        }
    }

    /// Create enabled thinking configuration with tool use
    pub fn enabled_with_tools(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens: Some(budget_tokens),
            allow_tool_use: Some(true),
        }
    }

    /// Create disabled thinking configuration
    pub fn disabled() -> Self {
        Self {
            thinking_type: "disabled".to_string(),
            budget_tokens: None,
            allow_tool_use: None,
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
    /// High effort / latency.
    High,
    /// Maximum effort / latency.
    Max,
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

    /// Create a configuration for JSON-schema constrained output.
    pub fn json_schema(schema: serde_json::Value) -> Self {
        Self::new().with_format(OutputFormat::json_schema(schema))
    }
}

/// Cache control for prompt caching
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheControl {
    /// Type of cache control ("ephemeral")
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    /// Create ephemeral cache control
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
        }
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
    /// System prompt (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
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
    /// Reusable execution container configuration (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<serde_json::Value>,
    /// Context management configuration (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_management: Option<serde_json::Value>,
    /// MCP server configuration list (beta)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<serde_json::Value>>,
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
            container: None,
            context_management: None,
            mcp_servers: None,
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

    /// Set system prompt
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
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

    /// Enable extended thinking mode (Claude 4 models)
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
    /// Token usage information
    pub usage: Usage,
    /// When the message was created
    pub created_at: DateTime<Utc>,
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

/// Request to count tokens in a message
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenCountRequest {
    /// Model to use for token counting
    pub model: String,
    /// Messages to count tokens for
    pub messages: Vec<Message>,
    /// System prompt to include in token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Tools to include in token count
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

impl TokenCountRequest {
    /// Create a new token count request
    pub fn new() -> Self {
        Self {
            model: crate::config::DEFAULT_MODEL.to_string(),
            messages: Vec::new(),
            system: None,
            tools: None,
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
        self.system = Some(system.into());
        self
    }

    /// Add a tool
    pub fn add_tool(mut self, tool: Tool) -> Self {
        self.tools.push_item(tool);
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

/// Streaming message delta
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageDelta {
    /// Stop reason if the message is complete
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// Stop sequence that caused the message to stop
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    /// Additional delta fields for forward compatibility
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Content block delta for streaming
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Additional delta fields for forward compatibility
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Streaming event types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StreamEvent {
    /// Message started
    MessageStart { message: MessageResponse },
    /// Message delta
    MessageDelta { delta: MessageDelta, usage: Usage },
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
}
