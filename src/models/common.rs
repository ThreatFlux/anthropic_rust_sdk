//! Common data types shared across API models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Cache control for prompt caching.
///
/// Attach to a content block, tool, or system block to mark a cache breakpoint,
/// or set [`crate::models::message::MessageRequest::cache_control`] to auto-cache
/// the last cacheable block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheControl {
    /// Type of cache control (always `"ephemeral"`).
    #[serde(rename = "type")]
    pub cache_type: String,
    /// Time-to-live for the cache entry: `"5m"` (default) or `"1h"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

impl CacheControl {
    /// Create ephemeral cache control with the default 5-minute TTL.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        }
    }

    /// Create ephemeral cache control with a 1-hour TTL.
    pub fn ephemeral_1h() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
        }
    }
}

/// Message role enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// User message
    User,
    /// Assistant message
    Assistant,
    /// System message (for system prompts)
    System,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::User => write!(f, "user"),
            Self::Assistant => write!(f, "assistant"),
            Self::System => write!(f, "system"),
        }
    }
}

/// Citation information attached to text content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TextCitation {
    /// Character span citation inside a document.
    CharLocation {
        cited_text: String,
        document_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_title: Option<String>,
        start_char_index: u32,
        end_char_index: u32,
    },
    /// Page range citation inside a document.
    PageLocation {
        cited_text: String,
        document_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_title: Option<String>,
        start_page_number: u32,
        end_page_number: u32,
    },
    /// Content block index citation for content-based documents.
    ContentBlockLocation {
        cited_text: String,
        document_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_title: Option<String>,
        start_block_index: u32,
        end_block_index: u32,
    },
    /// Citation that references a built-in search result.
    SearchResultLocation {
        search_result_index: u32,
        source: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cited_text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_block_index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_block_index: Option<u32>,
    },
    /// Citation that references a web-search result.
    WebSearchResultLocation {
        #[serde(skip_serializing_if = "Option::is_none")]
        cited_text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        encrypted_index: Option<String>,
    },
}

/// Citation settings for a document input block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentCitations {
    /// Whether citations are enabled for this document.
    pub enabled: bool,
}

impl DocumentCitations {
    /// Enable citations for this document.
    pub fn enabled() -> Self {
        Self { enabled: true }
    }

    /// Disable citations for this document.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }
}

/// Image source types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64 encoded image.
    Base64 { media_type: String, data: String },
    /// Publicly accessible image URL.
    Url { url: String },
    /// Previously uploaded file reference.
    File { file_id: String },
}

impl ImageSource {
    /// Create a base64 image source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    /// Create from image bytes.
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let data = BASE64_STANDARD.encode(bytes);
        Self::base64(media_type, data)
    }

    /// Create a URL image source.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url { url: url.into() }
    }

    /// Create a file-id image source.
    pub fn file(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: file_id.into(),
        }
    }
}

/// Document source types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    /// Base64 encoded document bytes.
    Base64 { media_type: String, data: String },
    /// Publicly accessible document URL.
    Url { url: String },
    /// Previously uploaded file reference.
    File { file_id: String },
    /// Inline text document source.
    Text { media_type: String, data: String },
    /// Inline content-based document source.
    Content { content: Vec<serde_json::Value> },
}

impl DocumentSource {
    /// Create a base64 document source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    /// Create from bytes using base64 encoding.
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let data = BASE64_STANDARD.encode(bytes);
        Self::base64(media_type, data)
    }

    /// Create a URL document source.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url { url: url.into() }
    }

    /// Create a file-id document source.
    pub fn file(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: file_id.into(),
        }
    }

    /// Create an inline text document source.
    pub fn text(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Text {
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    /// Create an inline content document source.
    pub fn content(content: Vec<serde_json::Value>) -> Self {
        Self::Content { content }
    }
}

/// Tool result content representation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Plain text tool result.
    Text(String),
    /// Structured content blocks.
    Blocks(Vec<ContentBlock>),
    /// Arbitrary JSON payload.
    Json(serde_json::Value),
}

/// Content block types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<TextCitation>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
    /// Image content.
    Image { source: ImageSource },
    /// Document content.
    Document {
        source: DocumentSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<DocumentCitations>,
    },
    /// Client tool use content.
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    /// Server tool use content.
    ServerToolUse {
        id: String,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        input: Option<serde_json::Value>,
    },
    /// Client tool result content.
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Built-in web-search tool result.
    WebSearchToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Built-in web-fetch tool result.
    WebFetchToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
    /// Thinking content.
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
    /// Redacted thinking payload.
    RedactedThinking { data: String },
    /// Refusal-fallback marker emitted when a server-side fallback model takes
    /// over a turn (Claude Fable 5 refusal fallbacks).
    Fallback {
        #[serde(default)]
        from: serde_json::Value,
        #[serde(default)]
        to: serde_json::Value,
    },
    /// Unknown content block type.
    #[serde(other)]
    Unknown,
}

impl ContentBlock {
    /// Create a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            citations: None,
            cache_control: None,
        }
    }

    /// Create a text content block with citations.
    pub fn text_with_citations(
        text: impl Into<String>,
        citations: impl IntoIterator<Item = TextCitation>,
    ) -> Self {
        let citations = citations.into_iter().collect::<Vec<_>>();
        Self::Text {
            text: text.into(),
            citations: Some(citations),
            cache_control: None,
        }
    }

    /// Attach a cache-control breakpoint to a text content block (no-op on
    /// other block types).
    pub fn with_cache_control(mut self, cc: CacheControl) -> Self {
        if let Self::Text { cache_control, .. } = &mut self {
            *cache_control = Some(cc);
        }
        self
    }

    /// Create an image content block.
    pub fn image(source: ImageSource) -> Self {
        Self::Image { source }
    }

    /// Create a document content block.
    pub fn document(source: DocumentSource) -> Self {
        Self::Document {
            source,
            title: None,
            context: None,
            citations: None,
        }
    }

    /// Create a tool use content block.
    pub fn tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        input: serde_json::Value,
    ) -> Self {
        Self::ToolUse {
            id: id.into(),
            name: name.into(),
            input,
        }
    }

    /// Create a server tool use content block.
    pub fn server_tool_use(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::ServerToolUse {
            id: id.into(),
            name: name.into(),
            input: None,
        }
    }

    /// Create a text tool result content block.
    pub fn tool_result(tool_use_id: impl Into<String>, content: Option<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.map(ToolResultContent::Text),
            is_error: Some(false),
        }
    }

    /// Create a JSON tool result content block.
    pub fn tool_result_json(tool_use_id: impl Into<String>, content: serde_json::Value) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Json(content)),
            is_error: Some(false),
        }
    }

    /// Create an error tool result content block.
    pub fn tool_error(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(content.into())),
            is_error: Some(true),
        }
    }

    /// Get text content if this is a text block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Get image source if this is an image block.
    pub fn as_image(&self) -> Option<&ImageSource> {
        match self {
            Self::Image { source } => Some(source),
            _ => None,
        }
    }

    /// Get document source if this is a document block.
    pub fn as_document(&self) -> Option<&DocumentSource> {
        match self {
            Self::Document { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Usage statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Usage {
    /// Number of input tokens.
    #[serde(default)]
    pub input_tokens: u32,
    /// Number of output tokens.
    #[serde(default)]
    pub output_tokens: u32,
    /// Input tokens written into cache in this request.
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    /// Input tokens read from cache in this request.
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    /// Cache creation breakdown by TTL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreationUsage>,
    /// Built-in server-tool usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUsage>,
    /// Inference geography used for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// Service tier used for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

/// Cache-creation usage breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CacheCreationUsage {
    /// Input tokens cached with 5-minute TTL.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
    /// Input tokens cached with 1-hour TTL.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,
}

/// Built-in server-tool usage stats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ServerToolUsage {
    /// Number of web-search requests made by the model.
    #[serde(default)]
    pub web_search_requests: u32,
}

impl Usage {
    /// Create new usage stats.
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation: None,
            server_tool_use: None,
            inference_geo: None,
            service_tier: None,
        }
    }

    /// Get total input tokens across uncached and cache-related token usage.
    pub fn total_input_tokens(&self) -> u32 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }

    /// Get total tokens.
    pub fn total_tokens(&self) -> u32 {
        self.total_input_tokens() + self.output_tokens
    }
}

/// Tool definition for client-side function calling and server-side tools.
///
/// Custom tools set `name`, `description`, and `input_schema`. Server tools
/// (web search, code execution, bash, text editor, memory, ...) set `tool_type`
/// to a versioned identifier and a fixed `name`; use the dedicated constructors.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Tool type. Omitted for custom tools; a versioned identifier for server
    /// tools (e.g. `web_search_20260209`, `code_execution_20260120`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    /// Tool name.
    pub name: String,
    /// Tool description (custom tools).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input schema as JSON Schema (custom tools).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// Require schema-valid tool arguments (strict tool use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// Cache control breakpoint for prompt caching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    /// Extra server-tool configuration fields (e.g. `max_uses`, `allowed_domains`).
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Tool {
    /// Create a new custom tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            tool_type: None,
            name: name.into(),
            description: Some(description.into()),
            input_schema: Some(input_schema),
            strict: None,
            cache_control: None,
            extra: HashMap::new(),
        }
    }

    /// Create a server-side tool by type + name.
    pub fn server(tool_type: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            tool_type: Some(tool_type.into()),
            name: name.into(),
            description: None,
            input_schema: None,
            strict: None,
            cache_control: None,
            extra: HashMap::new(),
        }
    }

    /// Built-in web search tool (`web_search_20260209`).
    pub fn web_search() -> Self {
        Self::server("web_search_20260209", "web_search")
    }

    /// Built-in web fetch tool (`web_fetch_20260209`).
    pub fn web_fetch() -> Self {
        Self::server("web_fetch_20260209", "web_fetch")
    }

    /// Built-in code execution tool (`code_execution_20260120`).
    pub fn code_execution() -> Self {
        Self::server("code_execution_20260120", "code_execution")
    }

    /// Built-in bash tool (`bash_20250124`).
    pub fn bash() -> Self {
        Self::server("bash_20250124", "bash")
    }

    /// Built-in text editor tool (`text_editor_20250728`).
    pub fn text_editor() -> Self {
        Self::server("text_editor_20250728", "str_replace_based_edit_tool")
    }

    /// Built-in memory tool (`memory_20250818`).
    pub fn memory() -> Self {
        Self::server("memory_20250818", "memory")
    }

    /// Enable strict tool use (schema-valid arguments).
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }

    /// Attach a cache-control breakpoint to this tool.
    pub fn with_cache_control(mut self, cache_control: CacheControl) -> Self {
        self.cache_control = Some(cache_control);
        self
    }

    /// Set an extra server-tool configuration field.
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

/// Tool choice options
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Auto tool selection
    #[default]
    Auto,
    /// Any tool can be used
    Any,
    /// Specific tool must be used
    Tool { name: String },
}

/// Message metadata
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Metadata {
    /// User ID associated with the message
    pub user_id: Option<String>,
    /// Custom metadata fields
    #[serde(flatten)]
    pub custom: HashMap<String, serde_json::Value>,
}

impl Metadata {
    /// Create new metadata
    pub fn new() -> Self {
        Self::default()
    }

    /// Set user ID
    pub fn with_user_id(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Add custom field
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }
}

/// Stop reason enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Hit maximum tokens limit
    MaxTokens,
    /// Natural end of message
    EndTurn,
    /// Stop sequence encountered
    StopSequence,
    /// Tool use requested
    ToolUse,
    /// Model paused and expects the conversation to continue
    PauseTurn,
    /// Response was declined for safety/policy reasons
    Refusal,
}

/// Structured detail accompanying a `refusal` (and other) stop reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StopDetails {
    /// Detail type (e.g. `"refusal"`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub detail_type: Option<String>,
    /// Policy category, e.g. `"cyber"`, `"bio"`, `"reasoning_extraction"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Human-readable explanation, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    /// Recommended model to retry with, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_model: Option<String>,
    /// Forward-compatible extra fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Model capabilities
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Vision capabilities (can process images)
    Vision,
    /// Tool use capabilities
    ToolUse,
    /// Document analysis
    Documents,
    /// Code generation
    Code,
}

/// Helper trait for adding items to optional vectors
pub trait VecPush<T> {
    /// Push an item to an optional vector, creating the vector if it doesn't exist
    fn push_item(&mut self, item: T);
}

impl<T> VecPush<T> for Option<Vec<T>> {
    fn push_item(&mut self, item: T) {
        self.get_or_insert_with(Vec::new).push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vec_push_none_option() {
        let mut opt_vec: Option<Vec<String>> = None;
        opt_vec.push_item("test".to_string());
        assert_eq!(opt_vec, Some(vec!["test".to_string()]));
    }

    #[test]
    fn test_vec_push_some_option() {
        let mut opt_vec: Option<Vec<String>> = Some(vec!["existing".to_string()]);
        opt_vec.push_item("new".to_string());
        assert_eq!(
            opt_vec,
            Some(vec!["existing".to_string(), "new".to_string()])
        );
    }

    #[test]
    fn test_vec_push_multiple_items() {
        let mut opt_vec: Option<Vec<i32>> = None;
        opt_vec.push_item(1);
        opt_vec.push_item(2);
        opt_vec.push_item(3);
        assert_eq!(opt_vec, Some(vec![1, 2, 3]));
    }

    #[test]
    fn test_tool_choice_default() {
        let choice = ToolChoice::default();
        assert_eq!(choice, ToolChoice::Auto);
    }

    #[test]
    fn test_metadata_creation() {
        let metadata = Metadata::new().with_user_id("user123").with_custom(
            "key".to_string(),
            serde_json::Value::String("value".to_string()),
        );

        assert_eq!(metadata.user_id, Some("user123".to_string()));
        assert!(metadata.custom.contains_key("key"));
    }

    #[test]
    fn test_usage_total_tokens() {
        let usage = Usage::new(100, 200);
        assert_eq!(usage.total_tokens(), 300);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 200);
    }

    #[test]
    fn test_usage_deserializes_partial() {
        let usage: Usage = serde_json::from_str(r#"{"output_tokens":5}"#).unwrap();
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.output_tokens, 5);
        assert_eq!(usage.cache_creation_input_tokens, 0);
        assert_eq!(usage.cache_read_input_tokens, 0);
    }

    #[test]
    fn test_usage_deserializes_extended_fields() {
        let usage: Usage = serde_json::from_str(
            r#"{
                "input_tokens": 10,
                "output_tokens": 5,
                "cache_creation_input_tokens": 3,
                "cache_read_input_tokens": 7,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 1,
                    "ephemeral_1h_input_tokens": 2
                },
                "server_tool_use": {
                    "web_search_requests": 4
                },
                "inference_geo": "us",
                "service_tier": "standard"
            }"#,
        )
        .unwrap();
        assert_eq!(usage.total_input_tokens(), 20);
        assert_eq!(usage.total_tokens(), 25);
        assert_eq!(
            usage
                .cache_creation
                .as_ref()
                .unwrap()
                .ephemeral_1h_input_tokens,
            2
        );
        assert_eq!(usage.server_tool_use.unwrap().web_search_requests, 4);
        assert_eq!(usage.inference_geo.as_deref(), Some("us"));
        assert_eq!(usage.service_tier.as_deref(), Some("standard"));
    }

    #[test]
    fn test_content_block_creators() {
        let text_block = ContentBlock::text("Hello");
        if let ContentBlock::Text { text, .. } = text_block {
            assert_eq!(text, "Hello");
        } else {
            panic!("Expected text block");
        }

        let tool_result = ContentBlock::tool_result("tool1", Some("result".to_string()));
        if let ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } = tool_result
        {
            assert_eq!(tool_use_id, "tool1");
            assert_eq!(content, Some(ToolResultContent::Text("result".to_string())));
            assert_eq!(is_error, Some(false));
        } else {
            panic!("Expected tool result block");
        }

        let error_result = ContentBlock::tool_error("tool1", "error message");
        if let ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } = error_result
        {
            assert_eq!(tool_use_id, "tool1");
            assert_eq!(
                content,
                Some(ToolResultContent::Text("error message".to_string()))
            );
            assert_eq!(is_error, Some(true));
        } else {
            panic!("Expected error result block");
        }
    }

    #[test]
    fn test_image_source_from_bytes() {
        let bytes = b"fake image data";
        let image_source = ImageSource::from_bytes("image/png", bytes);

        let ImageSource::Base64 { media_type, data } = image_source else {
            panic!("Expected base64 image source");
        };
        assert_eq!(media_type, "image/png");
        // Check that data is base64 encoded
        assert!(!data.is_empty());
    }

    #[test]
    fn test_document_source_file() {
        let source = DocumentSource::file("file_123");
        assert!(matches!(source, DocumentSource::File { .. }));

        let block = ContentBlock::document(source);
        assert!(block.as_document().is_some());
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::System.to_string(), "system");
    }

    #[test]
    fn test_server_tool_serialization() {
        let value = serde_json::to_value(Tool::web_search()).unwrap();
        assert_eq!(value["type"], "web_search_20260209");
        assert_eq!(value["name"], "web_search");
        // Server tools omit description/input_schema.
        assert!(value.get("description").is_none());
        assert!(value.get("input_schema").is_none());

        let code = serde_json::to_value(Tool::code_execution()).unwrap();
        assert_eq!(code["type"], "code_execution_20260120");
    }

    #[test]
    fn test_custom_tool_strict_and_cache() {
        let tool = Tool::new(
            "get_weather",
            "Get weather",
            serde_json::json!({"type": "object"}),
        )
        .with_strict(true)
        .with_cache_control(CacheControl::ephemeral());
        let value = serde_json::to_value(&tool).unwrap();
        assert_eq!(value["name"], "get_weather");
        assert_eq!(value["description"], "Get weather");
        assert_eq!(value["strict"], true);
        assert_eq!(value["cache_control"]["type"], "ephemeral");
        assert!(value.get("type").is_none());
    }

    #[test]
    fn test_text_block_cache_control_roundtrip() {
        let block = ContentBlock::text("hello").with_cache_control(CacheControl::ephemeral_1h());
        let value = serde_json::to_value(&block).unwrap();
        assert_eq!(value["type"], "text");
        assert_eq!(value["cache_control"]["type"], "ephemeral");
        assert_eq!(value["cache_control"]["ttl"], "1h");

        let parsed: ContentBlock = serde_json::from_value(value).unwrap();
        assert_eq!(parsed, block);
    }

    #[test]
    fn test_fallback_content_block_parses() {
        let block: ContentBlock = serde_json::from_value(serde_json::json!({
            "type": "fallback",
            "from": {"model": "claude-fable-5"},
            "to": {"model": "claude-opus-4-8"}
        }))
        .unwrap();
        assert!(matches!(block, ContentBlock::Fallback { .. }));
    }
}
