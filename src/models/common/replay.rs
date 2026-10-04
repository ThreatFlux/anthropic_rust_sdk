use super::*;

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
        if !self.is_allowed_for_role(role, unknown) {
            return Err(crate::AnthropicError::invalid_input(
                "content block is unsupported for this replay role or unknown policy",
            ));
        }
        self.validate_replay_state()?;
        self.validate_nested_replay(unknown)
    }

    fn is_allowed_for_role(&self, role: &Role, unknown: ReplayUnknownPolicy) -> bool {
        match (role, self) {
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
        }
    }

    fn validate_replay_state(&self) -> crate::Result<()> {
        match self {
            Self::Compaction { content, .. } if content.as_deref() == Some("") => Err(
                crate::AnthropicError::invalid_input("compaction replay content cannot be empty"),
            ),
            Self::Thinking { signature, .. } if signature.as_deref().is_none_or(str::is_empty) => {
                Err(crate::AnthropicError::invalid_input(
                    "thinking replay requires its original signature",
                ))
            }
            _ => Ok(()),
        }
    }

    fn validate_nested_replay(&self, unknown: ReplayUnknownPolicy) -> crate::Result<()> {
        match self {
            Self::Text {
                citations: Some(citations),
                ..
            } => validate_replay_citations(citations, unknown),
            Self::Image {
                source: ImageSource::Unknown(_),
                ..
            }
            | Self::Document {
                source: DocumentSource::Unknown(_),
                ..
            } if unknown == ReplayUnknownPolicy::Reject => Err(
                crate::AnthropicError::invalid_input("unfamiliar source requires preserve policy"),
            ),
            Self::Document {
                source: DocumentSource::Content { content, .. },
                ..
            } => {
                for value in content {
                    Self::raw(value.clone())?.validate_replay_role(&Role::User, unknown)?;
                }
                Ok(())
            }
            Self::ToolResult {
                content: Some(ToolResultContent::Blocks(blocks)),
                ..
            } => validate_tool_result_blocks(blocks, unknown),
            Self::ToolResult {
                content: Some(ToolResultContent::Json(_)),
                ..
            } => Err(crate::AnthropicError::invalid_input(
                "tool-result replay requires text or content blocks; serialize arbitrary JSON into text",
            )),
            Self::SearchResult { content, .. } => validate_replay_blocks(content, unknown),
            _ => Ok(()),
        }
    }
}

fn validate_replay_citations(
    citations: &[TextCitation],
    unknown: ReplayUnknownPolicy,
) -> crate::Result<()> {
    if unknown == ReplayUnknownPolicy::Reject
        && citations
            .iter()
            .any(|citation| matches!(citation, TextCitation::Unknown(_)))
    {
        return Err(crate::AnthropicError::invalid_input(
            "unfamiliar citation requires preserve policy",
        ));
    }
    Ok(())
}

fn validate_replay_blocks(
    blocks: &[ContentBlock],
    unknown: ReplayUnknownPolicy,
) -> crate::Result<()> {
    for block in blocks {
        block.validate_replay_role(&Role::User, unknown)?;
    }
    Ok(())
}

fn validate_tool_result_blocks(
    blocks: &[ContentBlock],
    unknown: ReplayUnknownPolicy,
) -> crate::Result<()> {
    for block in blocks {
        if !matches!(
            block,
            ContentBlock::Text { .. }
                | ContentBlock::Image { .. }
                | ContentBlock::Document { .. }
                | ContentBlock::SearchResult { .. }
                | ContentBlock::Unknown(_)
        ) {
            return Err(crate::AnthropicError::invalid_input(
                "unsupported nested tool-result block",
            ));
        }
        block.validate_replay_role(&Role::User, unknown)?;
    }
    Ok(())
}
