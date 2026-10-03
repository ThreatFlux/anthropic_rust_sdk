use super::*;

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
pub(super) fn deserialize_optional_object_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    let value = Option::<Value>::deserialize(deserializer)?;
    if value.as_ref().is_none_or(Value::is_object) {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("tool input must be an object"))
    }
}

pub(super) fn deserialize_search_content<'de, D: serde::Deserializer<'de>>(
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
