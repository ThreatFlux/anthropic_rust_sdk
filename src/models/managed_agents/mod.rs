//! Managed Agents data models (beta: managed-agents-2026-04-01).
//!
//! These models cover the Managed Agents surface: agents, environments,
//! sessions (with events and resources), vaults/credentials, memory stores, and
//! deployments. They mirror the SDK's existing model conventions — `type`
//! discriminators via `object_type`, `#[serde(skip_serializing_if)]` on
//! options, and a forward-compatible `extra: HashMap` escape hatch.

pub mod agent;
pub mod deployment;
pub mod environment;
pub mod memory;
pub mod session;
pub mod session_event;
pub mod vault;

pub use agent::{
    Agent, AgentCreateRequest, AgentListResponse, AgentModel, AgentSkillRef, AgentTool,
    AgentUpdateRequest, McpServer, Multiagent, MultiagentMember,
};
pub use deployment::{
    Deployment, DeploymentCreateRequest, DeploymentListResponse, DeploymentRun,
    DeploymentRunListResponse, DeploymentSchedule, DeploymentUpdateRequest,
};
pub use environment::{
    Environment, EnvironmentConfig, EnvironmentCreateRequest, EnvironmentListResponse,
    EnvironmentUpdateRequest, NetworkingConfig,
};
pub use memory::{
    Memory, MemoryCreateRequest, MemoryListResponse, MemoryRedactRequest, MemoryStore,
    MemoryStoreCreateRequest, MemoryStoreListResponse, MemoryStoreUpdateRequest,
    MemoryUpdateRequest, MemoryVersion, MemoryVersionListResponse,
};
pub use session::{
    Session, SessionAgentRef, SessionCreateRequest, SessionListResponse, SessionResource,
    SessionResourceListResponse, SessionResourceSpec, SessionResourceUpdateRequest, SessionStatus,
    SessionStopReason, SessionThread, SessionThreadListResponse, SessionUpdateRequest,
};
pub use session_event::{SendEvent, SessionEvent, SessionEventListResponse, SessionEventMeta};
pub use vault::{
    Credential, CredentialCreateRequest, CredentialKind, CredentialListResponse,
    CredentialUpdateRequest, Vault, VaultCreateRequest, VaultListResponse, VaultUpdateRequest,
};
