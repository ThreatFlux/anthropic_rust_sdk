//! # Threatflux - Anthropic API Rust SDK
//!
//! A comprehensive Rust SDK for the Anthropic API, providing async support,
//! streaming capabilities, and full coverage of the Anthropic API endpoints.
//!
//! ## Features
//!
//! - **Full API Coverage**: Complete implementation of all Anthropic API endpoints
//! - **Streaming Support**: Real-time message streaming with Server-Sent Events
//! - **Async/Await**: Built on tokio for high-performance async operations
//! - **Rate Limiting**: Built-in rate limiting with configurable limits
//! - **Retry Logic**: Exponential backoff retry with configurable policies
//! - **Type Safety**: Strongly typed models for all API requests and responses
//! - **File Uploads**: Support for uploading and managing files
//! - **Batch Processing**: Process multiple messages in batches
//! - **Admin API**: Full admin functionality for organizations and workspaces
//! - **Vision Support**: Image processing capabilities with base64 encoding
//! - **Tool Calling**: Function calling support with structured responses
//!
//! ## Quick Start
//!
//! ### Basic Message
//! ```rust,no_run
//! use threatflux::{Client, Config, models::MessageRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?;
//!     
//!     let request = MessageRequest::new()
//!         .model("claude-3-5-sonnet-20241022")
//!         .max_tokens(1000)
//!         .add_user_message("Hello, Claude!");
//!     
//!     let response = client.messages().create(request, None).await?;
//!     println!("Response: {}", response.text());
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Streaming Messages
//! ```rust,no_run
//! use threatflux::{Client, models::MessageRequest};
//! use futures::StreamExt;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?;
//!     
//!     let request = MessageRequest::new()
//!         .model("claude-3-5-sonnet-20241022")
//!         .max_tokens(1000)
//!         .add_user_message("Tell me a story");
//!     
//!     let mut stream = client.messages().create_stream(request, None).await?;
//!     
//!     while let Some(event) = stream.next().await {
//!         match event? {
//!             threatflux::models::StreamEvent::ContentBlockDelta { delta, .. } => {
//!                 if let Some(text) = delta.text {
//!                     print!("{}", text);
//!                 }
//!             }
//!             threatflux::models::StreamEvent::MessageStop => break,
//!             _ => {}
//!         }
//!     }
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Configuration
//! ```rust,no_run
//! use threatflux::{Client, Config};
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config::from_env()?
//!     .with_timeout(Duration::from_secs(30))
//!     .with_max_retries(3)
//!     .with_rate_limit_rps(50);
//!     
//! let client = Client::try_new(config)?;
//! # Ok(())
//! # }
//! ```

pub mod api;
pub mod builders;
pub mod client;
pub mod config;
pub mod error;
pub mod models;
pub mod streaming;
pub mod types;
pub mod utils;

// Re-export main types for convenience
pub use client::Client;
pub use config::{Config, DEFAULT_MODEL};
pub use error::{AnthropicError, Result};

// Re-export commonly used model types
pub use models::{
    ApiKey,
    ApiKeyActor,
    ApiKeyListParams,
    BatchResult,
    ClaudeCodeCoreMetrics,
    ClaudeCodeToolMetric,
    ClaudeCodeUsageActor,
    ClaudeCodeUsageReportParams,
    ClaudeCodeUsageReportResponse,
    ClaudeCodeUsageReportRow,
    CompletionRequest,
    CompletionResponse,
    CompletionStopReason,
    // Common types
    ContentBlock,
    ContentBlockDelta,
    // File types
    File,
    FileDownload,
    FileListResponse,
    FilePurpose,
    FileStatus,
    FileUploadRequest,
    FileUploadResponse,
    ImageSource,
    Invite,
    InviteCreateRequest,
    InviteCreateRole,
    InviteDeleteResponse,
    InviteListParams,
    InviteListResponse,
    InviteStatus,
    Member,
    MemberRole,
    MemberStatus,
    // Message types
    Message,
    // Batch types
    MessageBatch,
    MessageBatchCreateRequest,
    MessageBatchListResponse,
    MessageBatchRequest,
    MessageBatchResult,
    MessageBatchResultEntry,
    MessageBatchStatus,
    MessageCostReportBucket,
    MessageCostReportParams,
    MessageCostReportResponse,
    MessageDelta,
    MessageRequest,
    MessageResponse,
    MessageUsageReportBucket,
    MessageUsageReportParams,
    MessageUsageReportResponse,
    // Model types
    Model,
    ModelFamily,
    ModelListResponse,
    ModelSize,
    // Admin types
    Organization,
    OutputConfig,
    OutputEffort,
    OutputFormat,
    Role,
    // Skills types
    Skill,
    SkillCreateRequest,
    SkillDeleteResponse,
    SkillFileUpload,
    SkillLatestVersion,
    SkillListParams,
    SkillListResponse,
    SkillVersion,
    SkillVersionCreateRequest,
    SkillVersionDeleteResponse,
    SkillVersionListParams,
    SkillVersionListResponse,
    StopReason,
    StreamEvent,
    TokenCountRequest,
    TokenCountResponse,
    Tool,
    ToolChoice,
    Usage,
    UsageReport,
    User,
    UserDeleteResponse,
    UserListParams,
    UserListResponse,
    UserRole,
    UserUpdateRequest,
    UserUpdateRole,
    Workspace,
    WorkspaceDataResidency,
    WorkspaceListParams,
    WorkspaceMember,
    WorkspaceMemberCreateRequest,
    WorkspaceMemberCreateRole,
    WorkspaceMemberDeleteResponse,
    WorkspaceMemberListParams,
    WorkspaceMemberListResponse,
    WorkspaceMemberRole,
    WorkspaceMemberUpdateRequest,
    WorkspaceStatus,
    DEFAULT_COMPLETION_MODEL,
};

// Re-export utility types
pub use types::{
    ApiErrorResponse, HttpMethod, ModelCapability, PaginatedResponse, Pagination, RequestOptions,
    RequestPriority,
};

// Re-export streaming types
pub use streaming::{EventParser, MessageStream};

// Re-export builders
pub use builders::{batch_builder::BatchBuilder, message_builder::MessageBuilder};

// API version constant
pub const API_VERSION: &str = "2023-06-01";
