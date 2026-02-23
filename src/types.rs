//! Common types and utilities

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP method enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Get => "GET",
            Self::Post => "POST",
            Self::Put => "PUT",
            Self::Patch => "PATCH",
            Self::Delete => "DELETE",
        }
    }
}

/// Request options for customizing API calls
#[derive(Debug, Clone, Default)]
pub struct RequestOptions {
    /// Custom headers to include in the request
    pub headers: HashMap<String, String>,
    /// Override the default timeout for this request
    pub timeout: Option<std::time::Duration>,
    /// Disable retries for this request
    pub no_retry: bool,
    /// Enable Files API beta feature
    pub enable_files_api: bool,
    /// Enable PDF support beta feature
    pub enable_pdf_support: bool,
    /// Enable prompt caching beta feature
    pub enable_prompt_caching: bool,
    /// Enable 1M context window for Sonnet 4
    pub enable_1m_context: bool,
    /// Enable extended thinking with tools
    pub enable_extended_thinking_tools: bool,
    /// Additional beta features to enable (will be comma-joined)
    pub beta_features: Vec<String>,
}

impl RequestOptions {
    /// Create new empty request options
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a custom header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set a custom timeout
    pub fn with_timeout(mut self, timeout: std::time::Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Disable retries
    pub fn no_retry(mut self) -> Self {
        self.no_retry = true;
        self
    }

    /// Enable Files API beta feature
    pub fn with_files_api(mut self) -> Self {
        self.enable_files_api = true;
        self
    }

    /// Enable PDF support beta feature
    pub fn with_pdf_support(mut self) -> Self {
        self.enable_pdf_support = true;
        self
    }

    /// Enable prompt caching beta feature
    pub fn with_prompt_caching(mut self) -> Self {
        self.enable_prompt_caching = true;
        self
    }

    /// Enable 1M context window for Sonnet 4
    pub fn with_1m_context(mut self) -> Self {
        self.enable_1m_context = true;
        self
    }

    /// Enable extended thinking with tools
    pub fn with_extended_thinking_tools(mut self) -> Self {
        self.enable_extended_thinking_tools = true;
        self
    }

    /// Add a custom beta feature
    pub fn with_beta_feature(mut self, feature: impl Into<String>) -> Self {
        self.beta_features.push(feature.into());
        self
    }

    /// Create options for Claude 4 with extended thinking
    pub fn for_claude_4_thinking(budget_tokens: u32) -> Self {
        let mut options = Self::new();
        if budget_tokens > 32000 {
            options = options.with_extended_thinking_tools();
        }
        options
    }

    /// Create options for Sonnet 4 with 1M context
    pub fn for_sonnet_4_large_context() -> Self {
        Self::new().with_1m_context()
    }
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize)]
pub struct Pagination {
    /// Number of items to return per page
    pub limit: Option<u32>,
    /// Cursor for pagination
    pub after: Option<String>,
    /// Cursor for reverse pagination
    pub before: Option<String>,
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            limit: Some(20),
            after: None,
            before: None,
        }
    }
}

impl Pagination {
    /// Create new pagination with default limit
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the limit
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the after cursor
    pub fn with_after(mut self, after: impl Into<String>) -> Self {
        self.after = Some(after.into());
        self
    }

    /// Set the before cursor
    pub fn with_before(mut self, before: impl Into<String>) -> Self {
        self.before = Some(before.into());
        self
    }
}

/// Paginated response wrapper
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaginatedResponse<T> {
    /// The data items
    pub data: Vec<T>,
    /// Whether there are more items available
    pub has_more: bool,
    /// The cursor for the first item
    pub first_id: Option<String>,
    /// The cursor for the last item
    pub last_id: Option<String>,
}

/// API error response structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiErrorResponse {
    /// The error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// The error message
    pub message: String,
}

/// File upload progress callback
pub type ProgressCallback = Box<dyn Fn(u64, u64) + Send + Sync>;

/// Model capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCapability {
    TextGeneration,
    VisionProcessing,
    FunctionCalling,
    PromptCaching,
    ExtendedThinking,
    LargeContext1M,
    HybridReasoning,
    ToolUseDuringThinking,
}

/// Request priority level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RequestPriority {
    Low,
    #[default]
    Normal,
    High,
}

/// Stream event type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamEventType {
    MessageStart,
    MessageDelta,
    MessageStop,
    ContentBlockStart,
    ContentBlockDelta,
    ContentBlockStop,
    Ping,
    Error,
}

impl std::fmt::Display for StreamEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MessageStart => write!(f, "message_start"),
            Self::MessageDelta => write!(f, "message_delta"),
            Self::MessageStop => write!(f, "message_stop"),
            Self::ContentBlockStart => write!(f, "content_block_start"),
            Self::ContentBlockDelta => write!(f, "content_block_delta"),
            Self::ContentBlockStop => write!(f, "content_block_stop"),
            Self::Ping => write!(f, "ping"),
            Self::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for StreamEventType {
    type Err = crate::error::AnthropicError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "message_start" => Ok(Self::MessageStart),
            "message_delta" => Ok(Self::MessageDelta),
            "message_stop" => Ok(Self::MessageStop),
            "content_block_start" => Ok(Self::ContentBlockStart),
            "content_block_delta" => Ok(Self::ContentBlockDelta),
            "content_block_stop" => Ok(Self::ContentBlockStop),
            "ping" => Ok(Self::Ping),
            "error" => Ok(Self::Error),
            _ => Err(crate::error::AnthropicError::invalid_input(format!(
                "Unknown stream event type: {}",
                s
            ))),
        }
    }
}
