//! Common data types shared across API models

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Content block types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Image content
    Image { source: ImageSource },
    /// Tool use content
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result content
    ToolResult {
        tool_use_id: String,
        content: Option<String>,
        is_error: Option<bool>,
    },
}

impl ContentBlock {
    /// Create a text content block
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create an image content block
    pub fn image(source: ImageSource) -> Self {
        Self::Image { source }
    }

    /// Create a tool use content block
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

    /// Create a tool result content block
    pub fn tool_result(tool_use_id: impl Into<String>, content: Option<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content,
            is_error: Some(false),
        }
    }

    /// Create an error tool result content block
    pub fn tool_error(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: Some(content.into()),
            is_error: Some(true),
        }
    }

    /// Get text content if this is a text block
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text } => Some(text),
            _ => None,
        }
    }
}

/// Image source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64 encoded image
    Base64 { media_type: String, data: String },
}

impl ImageSource {
    /// Create a base64 image source
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
        }
    }

    /// Create from image bytes
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let data = BASE64_STANDARD.encode(bytes);
        Self::base64(media_type, data)
    }
}

/// Usage statistics
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Number of input tokens
    pub input_tokens: u32,
    /// Number of output tokens
    pub output_tokens: u32,
}

impl Usage {
    /// Create new usage stats
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
        }
    }

    /// Get total tokens
    pub fn total_tokens(&self) -> u32 {
        self.input_tokens + self.output_tokens
    }
}

/// Tool definition for function calling
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,
}

impl Tool {
    /// Create a new tool definition
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
        }
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
    fn test_content_block_creators() {
        let text_block = ContentBlock::text("Hello");
        if let ContentBlock::Text { text } = text_block {
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
            assert_eq!(content, Some("result".to_string()));
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
            assert_eq!(content, Some("error message".to_string()));
            assert_eq!(is_error, Some(true));
        } else {
            panic!("Expected error result block");
        }
    }

    #[test]
    fn test_image_source_from_bytes() {
        let bytes = b"fake image data";
        let image_source = ImageSource::from_bytes("image/png", bytes);

        let ImageSource::Base64 { media_type, data } = image_source;
        assert_eq!(media_type, "image/png");
        // Check that data is base64 encoded
        assert!(!data.is_empty());
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::User.to_string(), "user");
        assert_eq!(Role::Assistant.to_string(), "assistant");
        assert_eq!(Role::System.to_string(), "system");
    }
}
