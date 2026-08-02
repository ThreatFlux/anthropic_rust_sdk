//! # Anthropic Rust SDK
//!
//! An async, typed client for the Anthropic API. This is an unofficial,
//! community-maintained SDK and is not developed, endorsed, or supported by
//! Anthropic.
//!
//! The crate provides high-level clients for Messages, Models, Message Batches,
//! Files, Skills, selected administration resources, and selected beta or
//! research-preview resources. API schemas and preview availability can change
//! independently of a crate release; consult the repository's coverage notes
//! and Anthropic's official API documentation before relying on a less common
//! surface.
//!
//! ## Operational behavior
//!
//! - Non-streaming requests retry eligible network, timeout, rate-limit, and
//!   selected server errors by default. Use [`RequestOptions::no_retry`] when a
//!   mutation must not be replayed automatically.
//! - Streaming requests are not automatically reconnected, resumed, or retried.
//! - Request timeouts apply per attempt; total retry time can be longer.
//! - Rate-limiter utilities are available in [`utils::rate_limit`], but
//!   [`Client`] does not automatically apply them in version 0.2.0.
//! - A custom base URL receives the configured credential and must be trusted.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use threatflux_anthropic_sdk::{Client, MessageBuilder, DEFAULT_MODEL};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::from_env()?;
//!     let model = std::env::var("ANTHROPIC_MODEL")
//!         .unwrap_or_else(|_| DEFAULT_MODEL.to_owned());
//!
//!     let request = MessageBuilder::new()
//!         .model(model)
//!         .max_tokens(256)
//!         .user("Explain Rust ownership in one short paragraph.")
//!         .build_validated()?;
//!
//!     let response = client.messages().create(request, None).await?;
//!     println!("{}", response.text());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Configuration
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use threatflux_anthropic_sdk::{Client, Config, Result};
//!
//! fn configured_client(api_key: &str) -> Result<Client> {
//!     let config = Config::new(api_key)?
//!         .with_timeout(Duration::from_secs(30))
//!         .with_max_retries(2);
//!     Client::try_new(config)
//! }
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
    // Managed Agents types
    Agent,
    AgentCreateRequest,
    AgentListResponse,
    AgentModel,
    AgentTool,
    AgentUpdateRequest,
    ApiKey,
    ApiKeyActor,
    ApiKeyListParams,
    BatchResult,
    // Prompt caching
    CacheControl,
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
    Credential,
    CredentialCreateRequest,
    CredentialKind,
    CredentialListResponse,
    Deployment,
    DeploymentCreateRequest,
    DeploymentListResponse,
    DeploymentRun,
    Dream,
    DreamCreateRequest,
    DreamError,
    DreamInput,
    DreamListParams,
    DreamListResponse,
    DreamModel,
    DreamModelConfig,
    DreamOutput,
    DreamStatus,
    DreamUsage,
    EnrollmentUrl,
    Environment,
    EnvironmentConfig,
    EnvironmentCreateRequest,
    EnvironmentListResponse,
    // Refusal fallbacks
    Fallback,
    FallbackCreditToken,
    Fallbacks,
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
    MemoryStore,
    MemoryStoreCreateRequest,
    MemoryStoreListResponse,
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
    OutputTokensDetails,
    Role,
    SendEvent,
    Session,
    SessionCreateRequest,
    SessionEvent,
    SessionEventListResponse,
    SessionListResponse,
    SessionStatus,
    SessionStopReason,
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
    StopDetails,
    StopReason,
    StreamEvent,
    SystemBlock,
    SystemPrompt,
    TaskBudget,
    ThinkingConfig,
    TokenCountRequest,
    TokenCountResponse,
    Tool,
    ToolChoice,
    TrustGrant,
    TrustGrantStatus,
    Tunnel,
    TunnelCertificate,
    TunnelCertificateCreateRequest,
    TunnelCertificateListResponse,
    TunnelCreateRequest,
    TunnelListParams,
    TunnelListResponse,
    TunnelRotateTokenRequest,
    TunnelToken,
    Usage,
    UsageReport,
    User,
    UserDeleteResponse,
    UserListParams,
    UserListResponse,
    UserProfile,
    UserProfileCreateRequest,
    UserProfileListParams,
    UserProfileListResponse,
    UserProfileRelationship,
    UserProfileUpdateRequest,
    UserRole,
    UserUpdateRequest,
    UserUpdateRole,
    Vault,
    VaultCreateRequest,
    VaultListResponse,
    WebhookEvent,
    WebhookEventData,
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
pub use streaming::{EventParser, MessageStream, SessionEventStream};

// Re-export builders
pub use builders::{batch_builder::BatchBuilder, message_builder::MessageBuilder};

// API version constant
pub const API_VERSION: &str = "2023-06-01";
