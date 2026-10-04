use super::*;

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
