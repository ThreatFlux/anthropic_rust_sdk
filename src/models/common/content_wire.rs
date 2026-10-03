use super::*;

const CONTENT_TYPES: &[&str] = &[
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
];

pub(super) fn deserialize_unknown_contentblock<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(deserializer, CONTENT_TYPES)
}

impl Serialize for ContentBlock {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Text { .. } | Self::Image { .. } | Self::Document { .. } => {
                self.serialize_prompt(serializer)
            }
            Self::ToolUse { .. }
            | Self::ServerToolUse { .. }
            | Self::ToolResult { .. }
            | Self::McpToolUse { .. } => self.serialize_client_tools(serializer),
            Self::WebSearchToolResult { .. } | Self::WebFetchToolResult { .. } => {
                self.serialize_web_results(serializer)
            }
            Self::CodeExecutionToolResult { .. }
            | Self::BashCodeExecutionToolResult { .. }
            | Self::TextEditorCodeExecutionToolResult { .. } => {
                self.serialize_execution_results(serializer)
            }
            Self::McpToolResult { .. } | Self::ToolSearchToolResult { .. } => {
                self.serialize_remote_results(serializer)
            }
            Self::Thinking { .. } | Self::RedactedThinking { .. } | Self::Fallback { .. } => {
                self.serialize_reasoning(serializer)
            }
            Self::SearchResult { .. } | Self::ContainerUpload { .. } | Self::Compaction { .. } => {
                self.serialize_context(serializer)
            }
            Self::Unknown(raw) => serialize_unknown_content(raw, serializer),
        }
    }
}

impl ContentBlock {
    fn serialize_prompt<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
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
            _ => unreachable!("prompt selected by serializer"),
        }
    }

    fn serialize_client_tools<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
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
            _ => unreachable!("client_tools selected by serializer"),
        }
    }

    fn serialize_web_results<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
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
            _ => unreachable!("web_results selected by serializer"),
        }
    }

    fn serialize_execution_results<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
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
            _ => unreachable!("execution_results selected by serializer"),
        }
    }

    fn serialize_remote_results<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
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
            _ => unreachable!("remote_results selected by serializer"),
        }
    }

    fn serialize_reasoning<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
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
            _ => unreachable!("reasoning selected by serializer"),
        }
    }

    fn serialize_context<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
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
            _ => unreachable!("context selected by serializer"),
        }
    }
}

fn serialize_unknown_content<S: serde::Serializer>(
    raw: &RawContentBlock,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if CONTENT_TYPES.contains(&raw.block_type()) {
        return Err(serde::ser::Error::custom(
            "recognized types must use a typed variant",
        ));
    }
    raw.serialize(serializer)
}

fn is_fallback_boundary(value: &Value) -> bool {
    value.is_object() && value.get("model").is_some_and(Value::is_string)
}
pub(super) fn deserialize_fallback_boundary<'de, D: serde::Deserializer<'de>>(
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
pub(super) fn deserialize_fallback_trigger<'de, D: serde::Deserializer<'de>>(
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
