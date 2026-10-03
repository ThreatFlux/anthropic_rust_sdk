//! Common data types shared across API models.
//!
//! Content and usage schemas were verified 2026-10-03 against Anthropic's
//! [stable content parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/content_block_param.py)
//! and [beta response blocks](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/beta/beta_content_block.py).
//! Unknown objects round-trip unchanged; known optional null/missing fields may
//! normalize to omission, and nullable cache counters normalize to zero.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Additional protocol fields retained without interpretation.
pub type ExtraFields = HashMap<String, Value>;

/// A complete object with a string discriminator. The payload cannot be mutated
/// independently of its discriminator, so unknown protocol blocks round-trip intact.
#[derive(Debug, Clone, PartialEq)]
pub struct RawContentBlock(Value);

impl RawContentBlock {
    /// Validate a raw protocol object. Service support for an unfamiliar type is unknown.
    pub fn new(value: Value) -> crate::Result<Self> {
        if !value.is_object() || value.get("type").and_then(Value::as_str).is_none() {
            return Err(crate::AnthropicError::invalid_input(
                "raw block requires an object with a string type",
            ));
        }
        Ok(Self(value))
    }
    /// The original discriminator.
    pub fn block_type(&self) -> &str {
        self.0["type"].as_str().expect("validated discriminator")
    }
    /// The complete immutable payload.
    pub fn as_value(&self) -> &Value {
        &self.0
    }
    /// Consume the wrapper without changing any payload values.
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl Serialize for RawContentBlock {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for RawContentBlock {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(Value::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

fn deserialize_unknown<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
    known: &[&str],
) -> Result<RawContentBlock, D::Error> {
    let raw = RawContentBlock::deserialize(deserializer)?;
    if known.contains(&raw.block_type()) {
        return Err(serde::de::Error::custom(
            "malformed recognized protocol block",
        ));
    }
    Ok(raw)
}

fn serialize_object<S: serde::Serializer>(
    serializer: S,
    tag: Option<&str>,
    fields: Vec<(&str, Value)>,
    extra: &ExtraFields,
    reserved: &[&str],
) -> Result<S::Ok, S::Error> {
    let mut object = serde_json::Map::new();
    if let Some(tag) = tag {
        object.insert("type".into(), Value::String(tag.into()));
    }
    for (key, value) in fields {
        object.insert(key.into(), value);
    }
    for (key, value) in extra {
        if reserved.contains(&key.as_str()) || (tag.is_some() && key == "type") {
            return Err(serde::ser::Error::custom(format!(
                "extra field collides with reserved field {key}"
            )));
        }
        object.insert(key.clone(), value.clone());
    }
    object.serialize(serializer)
}

macro_rules! serialize_variant {
    ($serializer:expr, $tag:expr, $extra:expr; $($required:ident),* ; $($optional:ident),*) => {{
        #[allow(unused_mut)]
        let mut fields = vec![$((stringify!($required), serde_json::to_value($required).map_err(serde::ser::Error::custom)?)),*];
        $(if let Some(value) = $optional { fields.push((stringify!($optional), serde_json::to_value(value).map_err(serde::ser::Error::custom)?)); })*
        serialize_object($serializer, Some($tag), fields, $extra, &[$(stringify!($required),)* $(stringify!($optional),)*])
    }};
}

fn deserialize_object_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("tool input must be an object"))
    }
}

/// Cache control for prompt caching.
///
/// Attach to a content block, tool, or system block to mark a cache breakpoint,
/// or set [`crate::models::message::MessageRequest::cache_control`] to auto-cache
/// the last cacheable block.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CacheControl {
    /// Type of cache control (always `"ephemeral"`).
    #[serde(rename = "type")]
    pub cache_type: String,
    /// Time-to-live for the cache entry: `"5m"` (default) or `"1h"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl CacheControl {
    /// Create ephemeral cache control with the default 5-minute TTL.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
            extra: ExtraFields::new(),
        }
    }

    /// Create ephemeral cache control with a 1-hour TTL.
    pub fn ephemeral_1h() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
            extra: ExtraFields::new(),
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
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
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
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
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
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
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
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
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
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
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_textcitation")] RawContentBlock),
}

/// Citation settings for a document input block.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DocumentCitations {
    /// Whether citations are enabled for this document.
    pub enabled: bool,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl DocumentCitations {
    /// Enable citations for this document.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            extra: ExtraFields::new(),
        }
    }

    /// Disable citations for this document.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            extra: ExtraFields::new(),
        }
    }
}

/// Image source types.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageSource {
    /// Base64 encoded image.
    Base64 {
        media_type: String,
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Publicly accessible image URL.
    Url {
        url: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Previously uploaded file reference.
    File {
        file_id: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_imagesource")] RawContentBlock),
}

impl ImageSource {
    /// Create a base64 image source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
            extra: ExtraFields::new(),
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
        Self::Url {
            url: url.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create a file-id image source.
    pub fn file(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: file_id.into(),
            extra: ExtraFields::new(),
        }
    }
}

/// Document source types.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DocumentSource {
    /// Base64 encoded document bytes.
    Base64 {
        media_type: String,
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Publicly accessible document URL.
    Url {
        url: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Previously uploaded file reference.
    File {
        file_id: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Inline text document source.
    Text {
        media_type: String,
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Inline content-based document source.
    Content {
        #[serde(deserialize_with = "deserialize_document_content")]
        content: Vec<serde_json::Value>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_documentsource")] RawContentBlock),
}

impl DocumentSource {
    /// Create a base64 document source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
            extra: ExtraFields::new(),
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
        Self::Url {
            url: url.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create a file-id document source.
    pub fn file(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: file_id.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create an inline text document source.
    pub fn text(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Text {
            media_type: media_type.into(),
            data: data.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create an inline content document source.
    pub fn content(content: Vec<serde_json::Value>) -> Self {
        Self::Content {
            content,
            extra: ExtraFields::new(),
        }
    }
}

/// Tool result content representation.
#[derive(Debug, Clone, PartialEq, Serialize)]
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ContentBlock {
    /// Text content.
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<Vec<TextCitation>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Image content.
    Image {
        source: ImageSource,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Document content.
    Document {
        source: DocumentSource,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        citations: Option<DocumentCitations>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Client tool use content.
    ToolUse {
        id: String,
        name: String,
        #[serde(deserialize_with = "deserialize_object_value")]
        input: serde_json::Value,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Server tool use content.
    ServerToolUse {
        id: String,
        name: String,
        #[serde(default, deserialize_with = "deserialize_optional_object_value")]
        input: Option<serde_json::Value>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Client tool result content.
    ToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<ToolResultContent>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Built-in web-search tool result.
    WebSearchToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Built-in web-fetch tool result.
    WebFetchToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Code execution tool result.
    CodeExecutionToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Bash code execution tool result.
    BashCodeExecutionToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Text editor code execution tool result.
    TextEditorCodeExecutionToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// MCP tool result.
    McpToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Tool-search tool result.
    ToolSearchToolResult {
        tool_use_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Thinking content.
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Redacted thinking payload.
    RedactedThinking {
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Refusal-fallback marker emitted when a server-side fallback model takes
    /// over a turn (Claude Fable 5 refusal fallbacks).
    Fallback {
        #[serde(deserialize_with = "deserialize_fallback_boundary")]
        from: serde_json::Value,
        #[serde(deserialize_with = "deserialize_fallback_boundary")]
        to: serde_json::Value,
        /// Typed refusal trigger, retaining any additional fields.
        #[serde(deserialize_with = "deserialize_fallback_trigger")]
        trigger: Value,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Search results supplied as prompt content.
    SearchResult {
        source: String,
        title: String,
        #[serde(deserialize_with = "deserialize_search_content")]
        content: Vec<ContentBlock>,
        #[serde(default)]
        citations: Option<DocumentCitations>,
        #[serde(default)]
        cache_control: Option<CacheControl>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// A file uploaded into the code-execution container.
    ContainerUpload {
        file_id: String,
        #[serde(default)]
        cache_control: Option<CacheControl>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// A tool invocation made against an explicitly configured MCP server.
    McpToolUse {
        id: String,
        name: String,
        server_name: String,
        #[serde(deserialize_with = "deserialize_object_value")]
        input: Value,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// A compaction snapshot, including signed opaque state for exact replay.
    Compaction {
        #[serde(default)]
        content: Option<String>,
        #[serde(default)]
        encrypted_content: Option<String>,
        #[serde(default)]
        signature: Option<String>,
        #[serde(default)]
        tool_changes: Option<Vec<Value>>,
        #[serde(default)]
        cache_control: Option<CacheControl>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_contentblock")] RawContentBlock),
}

impl ContentBlock {
    /// Create a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            citations: None,
            cache_control: None,
            extra: ExtraFields::new(),
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
            extra: ExtraFields::new(),
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
        Self::Image {
            source,
            extra: ExtraFields::new(),
        }
    }

    /// Create a document content block.
    pub fn document(source: DocumentSource) -> Self {
        Self::Document {
            source,
            title: None,
            context: None,
            citations: None,
            extra: ExtraFields::new(),
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
            extra: ExtraFields::new(),
        }
    }

    /// Create a server tool use content block.
    pub fn server_tool_use(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self::ServerToolUse {
            id: id.into(),
            name: name.into(),
            input: None,
            extra: ExtraFields::new(),
        }
    }

    /// Create a text tool result content block.
    pub fn tool_result(tool_use_id: impl Into<String>, content: Option<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: content.map(ToolResultContent::Text),
            is_error: Some(false),
            extra: ExtraFields::new(),
        }
    }

    /// Create a JSON tool result content block.
    pub fn tool_result_json(tool_use_id: impl Into<String>, content: serde_json::Value) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Json(content)),
            is_error: Some(false),
            extra: ExtraFields::new(),
        }
    }

    /// Create an error tool result content block.
    pub fn tool_error(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(ToolResultContent::Text(content.into())),
            is_error: Some(true),
            extra: ExtraFields::new(),
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
            Self::Image { source, .. } => Some(source),
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
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Usage {
    /// Number of input tokens.
    #[serde(default)]
    pub input_tokens: u32,
    /// Number of output tokens.
    #[serde(default)]
    pub output_tokens: u32,
    /// Input tokens written into cache. Missing/null normalize to zero.
    #[serde(default, deserialize_with = "deserialize_nullable_counter")]
    pub cache_creation_input_tokens: u32,
    /// Input tokens read from cache. Missing/null normalize to zero.
    #[serde(default, deserialize_with = "deserialize_nullable_counter")]
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
    /// Outcome of a fallback-credit redemption, when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_credit: Option<serde_json::Value>,
    /// Inference speed used for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    /// Breakdown of output tokens by reasoning category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// Iteration-level usage details, when returned by the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<Vec<serde_json::Value>>,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

/// Output token breakdown returned by newer reasoning models.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct OutputTokensDetails {
    /// Number of tokens used for model thinking.
    #[serde(default)]
    pub thinking_tokens: u32,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

/// Cache-creation usage breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct CacheCreationUsage {
    /// Input tokens cached with 5-minute TTL.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
    /// Input tokens cached with 1-hour TTL.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

/// Built-in server-tool usage stats.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct ServerToolUsage {
    /// Number of web-search requests made by the model.
    #[serde(default)]
    pub web_search_requests: u32,
    /// Number of web-fetch requests made by the model.
    #[serde(default)]
    pub web_fetch_requests: u32,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
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
            fallback_credit: None,
            speed: None,
            output_tokens_details: None,
            iterations: None,
            extra: ExtraFields::new(),
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
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Tool {
    /// Tool type. Omitted or `"custom"` for client tools; a versioned identifier for server
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
    /// Whether this is a client tool. An omitted type and the explicit `custom`
    /// type both represent a client tool; server type tags require server execution.
    pub fn is_client(&self) -> bool {
        self.tool_type
            .as_deref()
            .is_none_or(|kind| kind == "custom")
    }

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

/// Tool-selection policy encoded as a tagged API object.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ToolChoice {
    /// Let the model decide whether to invoke a tool.
    #[default]
    Auto,
    /// Require some tool.
    Any,
    /// Require the named tool.
    Tool { name: String },
    /// Disable tool use.
    None,
    /// Automatic selection with an explicitly supplied parallel flag.
    AutoWithOptions { disable_parallel_tool_use: bool },
    /// Required selection with an explicitly supplied parallel flag.
    AnyWithOptions { disable_parallel_tool_use: bool },
    /// Named selection with an explicitly supplied parallel flag.
    ToolWithOptions {
        name: String,
        disable_parallel_tool_use: bool,
    },
}

impl ToolChoice {
    /// Disable tool use.
    pub fn none() -> Self {
        Self::None
    }
    /// The canonical API discriminator, independent of optional controls.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Auto | Self::AutoWithOptions { .. } => "auto",
            Self::Any | Self::AnyWithOptions { .. } => "any",
            Self::Tool { .. } | Self::ToolWithOptions { .. } => "tool",
            Self::None => "none",
        }
    }
    /// The forced tool's name, if any.
    pub fn forced_tool_name(&self) -> Option<&str> {
        match self {
            Self::Tool { name } | Self::ToolWithOptions { name, .. } => Some(name),
            _ => None,
        }
    }
    /// Whether parallel tool use was explicitly disabled or enabled.
    pub fn disable_parallel_tool_use(&self) -> Option<bool> {
        match self {
            Self::AutoWithOptions {
                disable_parallel_tool_use,
            }
            | Self::AnyWithOptions {
                disable_parallel_tool_use,
            }
            | Self::ToolWithOptions {
                disable_parallel_tool_use,
                ..
            } => Some(*disable_parallel_tool_use),
            _ => None,
        }
    }
    /// Set the parallel control; `none` does not accept this field.
    pub fn with_disable_parallel_tool_use(self, disabled: bool) -> crate::Result<Self> {
        Ok(match self {
            Self::Auto | Self::AutoWithOptions { .. } => Self::AutoWithOptions {
                disable_parallel_tool_use: disabled,
            },
            Self::Any | Self::AnyWithOptions { .. } => Self::AnyWithOptions {
                disable_parallel_tool_use: disabled,
            },
            Self::Tool { name } | Self::ToolWithOptions { name, .. } => Self::ToolWithOptions {
                name,
                disable_parallel_tool_use: disabled,
            },
            Self::None => {
                return Err(crate::AnthropicError::invalid_input(
                    "none tool choice cannot specify parallel tool use",
                ))
            }
        })
    }
}

impl Serialize for ToolChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serde_json::Map::new();
        object.insert("type".into(), self.kind().into());
        if let Some(name) = self.forced_tool_name() {
            if name.is_empty() {
                return Err(serde::ser::Error::custom(
                    "forced tool name cannot be empty",
                ));
            }
            object.insert("name".into(), name.into());
        }
        if let Some(disabled) = self.disable_parallel_tool_use() {
            object.insert("disable_parallel_tool_use".into(), disabled.into());
        }
        object.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ToolChoice {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        // Accept the old SDK's null/name-only encodings when reading persisted requests.
        // A historic null cannot distinguish Auto from Any; Auto is the deterministic fallback.
        if value.is_null() {
            return Ok(Self::Auto);
        }
        let object = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("tool choice must be an object"))?;
        if object
            .keys()
            .any(|key| !["type", "name", "disable_parallel_tool_use"].contains(&key.as_str()))
        {
            return Err(serde::de::Error::custom("unexpected tool-choice field"));
        }
        let kind = match object.get("type") {
            Some(Value::String(kind)) => kind.as_str(),
            None if object.len() == 1 && object.contains_key("name") => "tool",
            _ => {
                return Err(serde::de::Error::custom(
                    "tool choice requires a string type",
                ))
            }
        };
        let name = object.get("name");
        let choice = match kind {
            "auto" if name.is_none() => Self::Auto,
            "any" if name.is_none() => Self::Any,
            "none" if name.is_none() => Self::None,
            "tool" => Self::Tool {
                name: name
                    .and_then(Value::as_str)
                    .filter(|name| !name.is_empty())
                    .ok_or_else(|| {
                        serde::de::Error::custom("tool choice requires a nonempty name")
                    })?
                    .to_owned(),
            },
            _ => {
                return Err(serde::de::Error::custom(
                    "invalid tool choice type or fields",
                ))
            }
        };
        match object.get("disable_parallel_tool_use") {
            None => Ok(choice),
            Some(Value::Bool(disabled)) => choice
                .with_disable_parallel_tool_use(*disabled)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "parallel tool-use flag must be a boolean",
            )),
        }
    }
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
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
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
    /// Model context window was exhausted.
    ModelContextWindowExceeded,
    /// An unfamiliar API reason, preserved exactly.
    Unknown(String),
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
    /// Opaque token used to redeem fallback credit on a retry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_credit_token: Option<String>,
    /// Whether the refusal token can be redeemed with assistant-prefill form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_has_prefill_claim: Option<bool>,
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
            ..
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
            ..
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

        let ImageSource::Base64 {
            media_type, data, ..
        } = image_source
        else {
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
            "to": {"model": "claude-opus-4-8"},
            "trigger": {"type":"refusal"}
        }))
        .unwrap();
        assert!(matches!(block, ContentBlock::Fallback { .. }));
    }
}

fn deserialize_unknown_textcitation<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(
        deserializer,
        &[
            "char_location",
            "page_location",
            "content_block_location",
            "search_result_location",
            "web_search_result_location",
        ],
    )
}

impl Serialize for TextCitation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::CharLocation {
                cited_text,
                document_index,
                file_id,
                document_title,
                start_char_index,
                end_char_index,
                extra,
            } => {
                serialize_variant!(serializer, "char_location", extra; cited_text, document_index, start_char_index, end_char_index; file_id, document_title)
            }
            Self::PageLocation {
                cited_text,
                document_index,
                file_id,
                document_title,
                start_page_number,
                end_page_number,
                extra,
            } => {
                serialize_variant!(serializer, "page_location", extra; cited_text, document_index, start_page_number, end_page_number; file_id, document_title)
            }
            Self::ContentBlockLocation {
                cited_text,
                document_index,
                file_id,
                document_title,
                start_block_index,
                end_block_index,
                extra,
            } => {
                serialize_variant!(serializer, "content_block_location", extra; cited_text, document_index, start_block_index, end_block_index; file_id, document_title)
            }
            Self::SearchResultLocation {
                search_result_index,
                source,
                title,
                cited_text,
                start_block_index,
                end_block_index,
                extra,
            } => {
                serialize_variant!(serializer, "search_result_location", extra; search_result_index, source, title; cited_text, start_block_index, end_block_index)
            }
            Self::WebSearchResultLocation {
                cited_text,
                title,
                url,
                encrypted_index,
                extra,
            } => {
                serialize_variant!(serializer, "web_search_result_location", extra; ; cited_text, title, url, encrypted_index)
            }
            Self::Unknown(raw) => {
                if [
                    "char_location",
                    "page_location",
                    "content_block_location",
                    "search_result_location",
                    "web_search_result_location",
                ]
                .contains(&raw.block_type())
                {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}

fn deserialize_unknown_imagesource<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(deserializer, &["base64", "url", "file"])
}

impl Serialize for ImageSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Base64 {
                media_type,
                data,
                extra,
            } => {
                serialize_variant!(serializer, "base64", extra; media_type, data; )
            }
            Self::Url { url, extra } => {
                serialize_variant!(serializer, "url", extra; url; )
            }
            Self::File { file_id, extra } => {
                serialize_variant!(serializer, "file", extra; file_id; )
            }
            Self::Unknown(raw) => {
                if ["base64", "url", "file"].contains(&raw.block_type()) {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}

fn deserialize_unknown_documentsource<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(deserializer, &["base64", "url", "file", "text", "content"])
}

impl Serialize for DocumentSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Base64 {
                media_type,
                data,
                extra,
            } => {
                serialize_variant!(serializer, "base64", extra; media_type, data; )
            }
            Self::Url { url, extra } => {
                serialize_variant!(serializer, "url", extra; url; )
            }
            Self::File { file_id, extra } => {
                serialize_variant!(serializer, "file", extra; file_id; )
            }
            Self::Text {
                media_type,
                data,
                extra,
            } => {
                serialize_variant!(serializer, "text", extra; media_type, data; )
            }
            Self::Content { content, extra } => {
                if !document_content_is_valid(content) {
                    return Err(serde::ser::Error::custom(
                        "inline document requires text or image content blocks",
                    ));
                }
                serialize_variant!(serializer, "content", extra; content; )
            }
            Self::Unknown(raw) => {
                if ["base64", "url", "file", "text", "content"].contains(&raw.block_type()) {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}

fn deserialize_unknown_contentblock<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(
        deserializer,
        &[
            "text",
            "image",
            "document",
            "tool_use",
            "server_tool_use",
            "tool_result",
            "web_search_tool_result",
            "web_fetch_tool_result",
            "code_execution_tool_result",
            "bash_code_execution_tool_result",
            "text_editor_code_execution_tool_result",
            "mcp_tool_result",
            "tool_search_tool_result",
            "thinking",
            "redacted_thinking",
            "fallback",
            "search_result",
            "container_upload",
            "mcp_tool_use",
            "compaction",
        ],
    )
}

impl Serialize for ContentBlock {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text {
                text,
                citations,
                cache_control,
                extra,
            } => {
                serialize_variant!(serializer, "text", extra; text; citations, cache_control)
            }
            Self::Image { source, extra } => {
                serialize_variant!(serializer, "image", extra; source; )
            }
            Self::Document {
                source,
                title,
                context,
                citations,
                extra,
            } => {
                serialize_variant!(serializer, "document", extra; source; title, context, citations)
            }
            Self::ToolUse {
                id,
                name,
                input,
                extra,
            } => {
                if !input.is_object() {
                    return Err(serde::ser::Error::custom("tool input must be an object"));
                }
                serialize_variant!(serializer, "tool_use", extra; id, name, input; )
            }
            Self::ServerToolUse {
                id,
                name,
                input,
                extra,
            } => {
                serialize_variant!(serializer, "server_tool_use", extra; id, name; input)
            }
            Self::ToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "tool_result", extra; tool_use_id; content, is_error)
            }
            Self::WebSearchToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "web_search_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::WebFetchToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "web_fetch_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::CodeExecutionToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "code_execution_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::BashCodeExecutionToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "bash_code_execution_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::TextEditorCodeExecutionToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "text_editor_code_execution_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::McpToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "mcp_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::ToolSearchToolResult {
                tool_use_id,
                content,
                is_error,
                extra,
            } => {
                serialize_variant!(serializer, "tool_search_tool_result", extra; tool_use_id; content, is_error)
            }
            Self::Thinking {
                thinking,
                signature,
                extra,
            } => {
                serialize_variant!(serializer, "thinking", extra; thinking; signature)
            }
            Self::RedactedThinking { data, extra } => {
                serialize_variant!(serializer, "redacted_thinking", extra; data; )
            }
            Self::Fallback {
                from,
                to,
                trigger,
                extra,
            } => {
                if !is_fallback_boundary(from)
                    || !is_fallback_boundary(to)
                    || !is_fallback_trigger(trigger)
                {
                    return Err(serde::ser::Error::custom(
                        "fallback boundary requires an object with a model string",
                    ));
                }
                serialize_variant!(serializer, "fallback", extra; from, to, trigger; )
            }
            Self::SearchResult {
                source,
                title,
                content,
                citations,
                cache_control,
                extra,
            } => {
                if content.iter().any(|block| {
                    !matches!(block, ContentBlock::Text { .. } | ContentBlock::Unknown(_))
                }) {
                    return Err(serde::ser::Error::custom(
                        "search result content must contain text blocks",
                    ));
                }
                serialize_variant!(serializer, "search_result", extra; source, title, content; citations, cache_control)
            }
            Self::ContainerUpload {
                file_id,
                cache_control,
                extra,
            } => {
                serialize_variant!(serializer, "container_upload", extra; file_id; cache_control)
            }
            Self::McpToolUse {
                id,
                name,
                server_name,
                input,
                extra,
            } => {
                if !input.is_object() {
                    return Err(serde::ser::Error::custom("tool input must be an object"));
                }
                serialize_variant!(serializer, "mcp_tool_use", extra; id, name, server_name, input; )
            }
            Self::Compaction {
                content,
                encrypted_content,
                signature,
                tool_changes,
                cache_control,
                extra,
            } => {
                serialize_variant!(serializer, "compaction", extra; content; encrypted_content, signature, tool_changes, cache_control)
            }
            Self::Unknown(raw) => {
                if [
                    "text",
                    "image",
                    "document",
                    "tool_use",
                    "server_tool_use",
                    "tool_result",
                    "web_search_tool_result",
                    "web_fetch_tool_result",
                    "code_execution_tool_result",
                    "bash_code_execution_tool_result",
                    "text_editor_code_execution_tool_result",
                    "mcp_tool_result",
                    "tool_search_tool_result",
                    "thinking",
                    "redacted_thinking",
                    "fallback",
                    "search_result",
                    "container_upload",
                    "mcp_tool_use",
                    "compaction",
                ]
                .contains(&raw.block_type())
                {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}

impl StopReason {
    /// The exact API wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::MaxTokens => "max_tokens",
            Self::EndTurn => "end_turn",
            Self::StopSequence => "stop_sequence",
            Self::ToolUse => "tool_use",
            Self::PauseTurn => "pause_turn",
            Self::Refusal => "refusal",
            Self::ModelContextWindowExceeded => "model_context_window_exceeded",
            Self::Unknown(value) => value,
        }
    }
}
impl Serialize for StopReason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for StopReason {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "max_tokens" => Self::MaxTokens,
            "end_turn" => Self::EndTurn,
            "stop_sequence" => Self::StopSequence,
            "tool_use" => Self::ToolUse,
            "pause_turn" => Self::PauseTurn,
            "refusal" => Self::Refusal,
            "model_context_window_exceeded" => Self::ModelContextWindowExceeded,
            value => Self::Unknown(value.to_owned()),
        })
    }
}

impl Serialize for CacheControl {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![(
            "type",
            serde_json::to_value(&self.cache_type).map_err(serde::ser::Error::custom)?,
        )];
        if let Some(value) = &self.ttl {
            fields.push((
                "ttl",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        serialize_object(serializer, None, fields, &self.extra, &["type", "ttl"])
    }
}

impl Serialize for DocumentCitations {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![(
            "enabled",
            serde_json::to_value(self.enabled).map_err(serde::ser::Error::custom)?,
        )];
        serialize_object(serializer, None, fields, &self.extra, &["enabled"])
    }
}

impl Serialize for Usage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![
            (
                "input_tokens",
                serde_json::to_value(self.input_tokens).map_err(serde::ser::Error::custom)?,
            ),
            (
                "output_tokens",
                serde_json::to_value(self.output_tokens).map_err(serde::ser::Error::custom)?,
            ),
            (
                "cache_creation_input_tokens",
                serde_json::to_value(self.cache_creation_input_tokens)
                    .map_err(serde::ser::Error::custom)?,
            ),
            (
                "cache_read_input_tokens",
                serde_json::to_value(self.cache_read_input_tokens)
                    .map_err(serde::ser::Error::custom)?,
            ),
        ];
        if let Some(value) = &self.cache_creation {
            fields.push((
                "cache_creation",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.server_tool_use {
            fields.push((
                "server_tool_use",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.inference_geo {
            fields.push((
                "inference_geo",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.service_tier {
            fields.push((
                "service_tier",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.fallback_credit {
            fields.push((
                "fallback_credit",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.speed {
            fields.push((
                "speed",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.output_tokens_details {
            fields.push((
                "output_tokens_details",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        if let Some(value) = &self.iterations {
            fields.push((
                "iterations",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &[
                "input_tokens",
                "output_tokens",
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
                "cache_creation",
                "server_tool_use",
                "inference_geo",
                "service_tier",
                "fallback_credit",
                "speed",
                "output_tokens_details",
                "iterations",
            ],
        )
    }
}

impl Serialize for OutputTokensDetails {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![(
            "thinking_tokens",
            serde_json::to_value(self.thinking_tokens).map_err(serde::ser::Error::custom)?,
        )];
        serialize_object(serializer, None, fields, &self.extra, &["thinking_tokens"])
    }
}

impl Serialize for CacheCreationUsage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![
            (
                "ephemeral_5m_input_tokens",
                serde_json::to_value(self.ephemeral_5m_input_tokens)
                    .map_err(serde::ser::Error::custom)?,
            ),
            (
                "ephemeral_1h_input_tokens",
                serde_json::to_value(self.ephemeral_1h_input_tokens)
                    .map_err(serde::ser::Error::custom)?,
            ),
        ];
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &["ephemeral_5m_input_tokens", "ephemeral_1h_input_tokens"],
        )
    }
}

impl Serialize for ServerToolUsage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![
            (
                "web_search_requests",
                serde_json::to_value(self.web_search_requests)
                    .map_err(serde::ser::Error::custom)?,
            ),
            (
                "web_fetch_requests",
                serde_json::to_value(self.web_fetch_requests).map_err(serde::ser::Error::custom)?,
            ),
        ];
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &["web_search_requests", "web_fetch_requests"],
        )
    }
}

fn deserialize_nullable_counter<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u32, D::Error> {
    Ok(Option::<u32>::deserialize(deserializer)?.unwrap_or_default())
}

fn deserialize_optional_object_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    if value.as_ref().is_none_or(Value::is_object) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("tool input must be an object"))
    }
}

fn deserialize_search_content<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ContentBlock>, D::Error> {
    let blocks = Vec::<ContentBlock>::deserialize(deserializer)?;
    if blocks
        .iter()
        .all(|block| matches!(block, ContentBlock::Text { .. } | ContentBlock::Unknown(_)))
    {
        Ok(blocks)
    } else {
        Err(serde::de::Error::custom(
            "search result content must contain text blocks",
        ))
    }
}

impl<'de> Deserialize<'de> for ToolResultContent {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        if let Value::String(text) = value {
            return Ok(Self::Text(text));
        }
        // Once an array advertises typed content, malformed known blocks are an
        // error rather than falling through to arbitrary JSON.
        if value.as_array().is_some_and(|items| {
            items.is_empty() || items.iter().any(|item| item.get("type").is_some())
        }) {
            return serde_json::from_value(value)
                .map(Self::Blocks)
                .map_err(serde::de::Error::custom);
        }
        Ok(Self::Json(value))
    }
}

/// Policy for unfamiliar content encountered while replaying a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReplayUnknownPolicy {
    /// Fail locally until the caller decides how to handle the new payload.
    Reject,
    /// Send the complete validated payload unchanged. Server support is unknown.
    Preserve,
}

impl ContentBlock {
    /// Parse a complete raw object, checking recognized block schemas.
    pub fn raw(value: Value) -> crate::Result<Self> {
        serde_json::from_value(value).map_err(|error| {
            crate::AnthropicError::invalid_input(format!("invalid content block: {error}"))
        })
    }

    /// Create a prompt search result containing text blocks.
    pub fn search_result(
        source: impl Into<String>,
        title: impl Into<String>,
        content: impl IntoIterator<Item = String>,
    ) -> Self {
        Self::SearchResult {
            source: source.into(),
            title: title.into(),
            content: content.into_iter().map(Self::text).collect(),
            citations: None,
            cache_control: None,
            extra: ExtraFields::new(),
        }
    }

    /// Reference an uploaded file in a code-execution container.
    pub fn container_upload(file_id: impl Into<String>) -> Self {
        Self::ContainerUpload {
            file_id: file_id.into(),
            cache_control: None,
            extra: ExtraFields::new(),
        }
    }

    /// Create an MCP tool invocation; its input must be a JSON object.
    pub fn mcp_tool_use(
        id: impl Into<String>,
        name: impl Into<String>,
        server_name: impl Into<String>,
        input: Value,
    ) -> crate::Result<Self> {
        if !input.is_object() {
            return Err(crate::AnthropicError::invalid_input(
                "MCP tool input must be an object",
            ));
        }
        Ok(Self::McpToolUse {
            id: id.into(),
            name: name.into(),
            server_name: server_name.into(),
            input,
            extra: ExtraFields::new(),
        })
    }

    /// Create a compaction snapshot. Keep signatures and encrypted state from
    /// a response intact when replaying an existing block instead of this constructor.
    pub fn compaction(content: Option<String>) -> crate::Result<Self> {
        if content.as_deref() == Some("") {
            return Err(crate::AnthropicError::invalid_input(
                "compaction content cannot be empty",
            ));
        }
        Ok(Self::Compaction {
            content,
            encrypted_content: None,
            signature: None,
            tool_changes: None,
            cache_control: None,
            extra: ExtraFields::new(),
        })
    }

    /// Validate a known block's request role and return an unchanged replay copy.
    ///
    /// This checks the payload and supported role combinations, including nested
    /// raw objects. It does not enable beta APIs or guarantee account/model access.
    /// Signed thinking and compaction state are preserved without alteration.
    pub fn checked_replay(&self, role: &Role, unknown: ReplayUnknownPolicy) -> crate::Result<Self> {
        let value = serde_json::to_value(self).map_err(|error| {
            crate::AnthropicError::invalid_input(format!("content cannot be replayed: {error}"))
        })?;
        let block: Self = serde_json::from_value(value).map_err(|error| {
            crate::AnthropicError::invalid_input(format!("content cannot be replayed: {error}"))
        })?;
        block.validate_replay_role(role, unknown)?;
        Ok(block)
    }

    /// Validate role and explicit unknown handling without modifying the block.
    pub fn validate_replay_role(
        &self,
        role: &Role,
        unknown: ReplayUnknownPolicy,
    ) -> crate::Result<()> {
        let allowed = match (role, self) {
            (_, Self::Unknown(_)) => unknown == ReplayUnknownPolicy::Preserve,
            (Role::System, Self::Text { .. }) => true,
            (
                Role::User,
                Self::Text { .. }
                | Self::Image { .. }
                | Self::Document { .. }
                | Self::SearchResult { .. }
                | Self::ToolResult { .. }
                | Self::ContainerUpload { .. },
            ) => true,
            (
                Role::Assistant,
                Self::Text { .. }
                | Self::ToolUse { .. }
                | Self::ServerToolUse { .. }
                | Self::WebSearchToolResult { .. }
                | Self::WebFetchToolResult { .. }
                | Self::CodeExecutionToolResult { .. }
                | Self::BashCodeExecutionToolResult { .. }
                | Self::TextEditorCodeExecutionToolResult { .. }
                | Self::McpToolUse { .. }
                | Self::McpToolResult { .. }
                | Self::ToolSearchToolResult { .. }
                | Self::Thinking { .. }
                | Self::RedactedThinking { .. }
                | Self::Fallback { .. }
                | Self::Compaction { .. }
                | Self::ContainerUpload { .. },
            ) => true,
            _ => false,
        };
        if !allowed {
            return Err(crate::AnthropicError::invalid_input(
                "content block is unsupported for this replay role or unknown policy",
            ));
        }
        match self {
            Self::Text {
                citations: Some(citations),
                ..
            } => {
                if unknown == ReplayUnknownPolicy::Reject
                    && citations
                        .iter()
                        .any(|citation| matches!(citation, TextCitation::Unknown(_)))
                {
                    return Err(crate::AnthropicError::invalid_input(
                        "unfamiliar citation requires preserve policy",
                    ));
                }
            }
            Self::Image {
                source: ImageSource::Unknown(_),
                ..
            }
            | Self::Document {
                source: DocumentSource::Unknown(_),
                ..
            } if unknown == ReplayUnknownPolicy::Reject => {
                return Err(crate::AnthropicError::invalid_input(
                    "unfamiliar source requires preserve policy",
                ));
            }
            Self::Document {
                source: DocumentSource::Content { content, .. },
                ..
            } => {
                for value in content {
                    Self::raw(value.clone())?.validate_replay_role(&Role::User, unknown)?;
                }
            }
            Self::ToolResult {
                content: Some(ToolResultContent::Blocks(blocks)),
                ..
            } => {
                for block in blocks {
                    if !matches!(
                        block,
                        Self::Text { .. }
                            | Self::Image { .. }
                            | Self::Document { .. }
                            | Self::SearchResult { .. }
                            | Self::Unknown(_)
                    ) {
                        return Err(crate::AnthropicError::invalid_input(
                            "unsupported nested tool-result block",
                        ));
                    }
                    block.validate_replay_role(&Role::User, unknown)?;
                }
            }
            Self::ToolResult {
                content: Some(ToolResultContent::Json(_)),
                ..
            } => {
                return Err(crate::AnthropicError::invalid_input("tool-result replay requires text or content blocks; serialize arbitrary JSON into text"));
            }
            Self::SearchResult {
                content: blocks, ..
            } => {
                for block in blocks {
                    block.validate_replay_role(&Role::User, unknown)?;
                }
            }
            Self::Compaction { content, .. } if content.as_deref() == Some("") => {
                return Err(crate::AnthropicError::invalid_input(
                    "compaction replay content cannot be empty",
                ));
            }
            Self::Thinking { signature, .. } if signature.as_deref().is_none_or(str::is_empty) => {
                return Err(crate::AnthropicError::invalid_input(
                    "thinking replay requires its original signature",
                ));
            }
            _ => {}
        }
        Ok(())
    }
}

impl Serialize for Tool {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = vec![("name", Value::String(self.name.clone()))];
        if let Some(value) = &self.tool_type {
            fields.push(("type", Value::String(value.clone())));
        }
        if let Some(value) = &self.description {
            fields.push(("description", Value::String(value.clone())));
        }
        if let Some(value) = &self.input_schema {
            fields.push(("input_schema", value.clone()));
        }
        if let Some(value) = self.strict {
            fields.push(("strict", Value::Bool(value)));
        }
        if let Some(value) = &self.cache_control {
            fields.push((
                "cache_control",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &[
                "type",
                "name",
                "description",
                "input_schema",
                "strict",
                "cache_control",
            ],
        )
    }
}

fn is_fallback_boundary(value: &Value) -> bool {
    value.is_object() && value.get("model").is_some_and(Value::is_string)
}
fn deserialize_fallback_boundary<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if is_fallback_boundary(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "fallback boundary requires an object with a model string",
        ))
    }
}

fn is_fallback_trigger(value: &Value) -> bool {
    value.is_object()
        && value.get("type").and_then(Value::as_str) == Some("refusal")
        && value
            .get("category")
            .is_none_or(|value| value.is_null() || value.is_string())
}
fn deserialize_fallback_trigger<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if is_fallback_trigger(&value) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom(
            "fallback trigger requires refusal type and nullable string category",
        ))
    }
}

fn document_content_is_valid(content: &[Value]) -> bool {
    content.iter().all(|value| {
        matches!(
            ContentBlock::raw(value.clone()),
            Ok(ContentBlock::Text { .. } | ContentBlock::Image { .. } | ContentBlock::Unknown(_))
        )
    })
}
fn deserialize_document_content<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Value>, D::Error> {
    let content = Vec::<Value>::deserialize(deserializer)?;
    if document_content_is_valid(&content) {
        Ok(content)
    } else {
        Err(serde::de::Error::custom(
            "inline document requires text or image content blocks",
        ))
    }
}
