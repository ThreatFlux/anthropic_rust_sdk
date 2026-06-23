# Managed Agents — Implementation Plan (threatflux Anthropic Rust SDK)

**Status:** Plan only — no code written yet.
**Target beta:** `managed-agents-2026-04-01` (already declared as `beta_headers::MANAGED_AGENTS` in `src/client.rs`).
**Scope:** Add full Managed Agents surface (Agents, Environments, Sessions + Events/Resources/Threads, Vaults + Credentials, Memory Stores, Deployments) while reconciling the existing Skills and Files APIs.

This plan is written to mirror the SDK's existing conventions exactly:

- **API module pattern** (`src/api/<name>.rs`): a `#[derive(Clone)] pub struct XApi { client: Client }` with `pub fn new(client: Client) -> Self`, and `async fn` methods returning `Result<T>` that delegate to `self.client.request(HttpMethod::_, path, body, options)`. See `src/api/message_batches.rs`.
- **Client wiring**: a `client.x()` accessor returning `XApi::new(self.clone())` (see `Client::messages()`, `Client::message_batches()` in `src/client.rs`).
- **Models** (`src/models/<name>.rs`): `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` structs; `#[serde(rename = "type")] pub object_type: String`; `#[serde(skip_serializing_if = "Option::is_none")]` on optional fields; `#[serde(flatten, default) pub extra: HashMap<String, serde_json::Value>]` forward-compat escape hatch (as in `src/models/skill.rs`); tagged enums via `#[serde(tag = "type", rename_all = "snake_case")]` (as in `MessageBatchResult`).
- **Pagination**: reuse `types::Pagination` / `types::PaginatedResponse<T>` and `api::utils::build_paginated_path` (cursor-style `after`/`before`/`limit`, `has_more`, `first_id`/`last_id`). NOTE the Skills API instead uses `page`-token pagination; Managed Agents list endpoints use the cursor style, so `PaginatedResponse<T>` is the right base.
- **Beta header**: piggyback on `RequestOptions::beta_features` + `Client::build_headers`. Each Managed-Agents API module force-injects the beta header (mirroring `SkillsApi::with_skills_beta`).
- **Streaming**: reuse the `EventParser` / channel-pump architecture in `src/streaming/message_stream.rs`, generalized to a new `SessionEventStream`.

---

## 1. Module Layout

### 1.1 New API modules — `src/api/managed_agents/`

We namespace under a single sub-module directory (mirrors `src/api/admin/`) so the new surface is grouped and the `api` root stays readable.

```
src/api/managed_agents/
├── mod.rs            // pub mod re-exports + shared beta-injection helper
├── agents.rs         // AgentsApi          -> /v1/agents
├── environments.rs   // EnvironmentsApi     -> /v1/environments
├── sessions.rs       // SessionsApi         -> /v1/sessions  (+ events, resources, threads sub-paths)
├── session_events.rs // SessionEventsApi    -> /v1/sessions/{id}/events (+ /stream)
├── session_resources.rs // SessionResourcesApi -> /v1/sessions/{id}/resources
├── session_threads.rs   // SessionThreadsApi -> /v1/sessions/{id}/threads (+ per-thread events)
├── vaults.rs         // VaultsApi + CredentialsApi -> /v1/vaults, /v1/vaults/{id}/credentials
├── memory_stores.rs  // MemoryStoresApi     -> /v1/memory_stores (+ memories, memory_versions)
└── deployments.rs    // DeploymentsApi      -> /v1/deployments (+ deployment_runs)
```

`src/api/managed_agents/mod.rs`:

```rust
//! Managed Agents API (beta: managed-agents-2026-04-01)
pub mod agents;
pub mod environments;
pub mod sessions;
pub mod session_events;
pub mod session_resources;
pub mod session_threads;
pub mod vaults;
pub mod memory_stores;
pub mod deployments;

pub use agents::AgentsApi;
pub use environments::EnvironmentsApi;
pub use sessions::SessionsApi;
pub use session_events::SessionEventsApi;
pub use session_resources::SessionResourcesApi;
pub use session_threads::SessionThreadsApi;
pub use vaults::{VaultsApi, CredentialsApi};
pub use memory_stores::MemoryStoresApi;
pub use deployments::DeploymentsApi;

use crate::types::RequestOptions;

/// Force the managed-agents beta header onto a request (mirrors SkillsApi::with_skills_beta).
pub(crate) fn with_managed_agents_beta(options: Option<RequestOptions>) -> Option<RequestOptions> {
    Some(
        options
            .unwrap_or_default()
            .with_beta_feature(crate::client::beta_headers::MANAGED_AGENTS),
    )
}
```

Register in `src/api/mod.rs`:

```rust
pub mod managed_agents;
pub use managed_agents::{
    AgentsApi, CredentialsApi, DeploymentsApi, EnvironmentsApi, MemoryStoresApi,
    SessionEventsApi, SessionResourcesApi, SessionThreadsApi, SessionsApi, VaultsApi,
};
```

### 1.2 New model modules — `src/models/`

```
src/models/managed_agents/
├── mod.rs            // re-exports
├── agent.rs          // Agent, AgentModel, AgentTool, AgentToolset, McpServer, Multiagent, AgentCreateRequest, AgentUpdateRequest, AgentListResponse
├── environment.rs    // Environment, EnvironmentConfig, NetworkingConfig, Environment{Create,Update}Request, EnvironmentListResponse
├── session.rs        // Session, SessionStatus, SessionStopReason, SessionAgentRef, SessionResourceSpec, Session{Create,Update}Request, SessionListResponse
├── session_event.rs  // SessionEvent (big tagged enum), SessionEventEnvelope, SendEvent, SessionEventListResponse
├── vault.rs          // Vault, Credential, CredentialKind, Vault/Credential requests + list responses
├── memory.rs         // MemoryStore, Memory, MemoryVersion, requests + list responses
└── deployment.rs     // Deployment, DeploymentRun, DeploymentSchedule, requests + list responses
```

Wire into `src/models/mod.rs` with `pub mod managed_agents;` and selective `pub use` of the top-level types. Re-export the most-used types from `src/lib.rs` (`Agent`, `Session`, `SessionEvent`, `Environment`, `Vault`, `Credential`, `MemoryStore`, `Deployment`, and their `*CreateRequest`/`*ListResponse`).

> **Decision:** put the new models under a `managed_agents/` sub-dir rather than flat files, because there are ~8 model files; this keeps `src/models/` from ballooning and matches how `admin` is its own grouping at the API layer. (Models for admin happen to be flat in `admin.rs`; we deviate intentionally given the size.)

### 1.3 Client accessors — `src/client.rs`

Add accessor methods alongside the existing ones (all infallible — no admin key gating, these use the standard API key):

```rust
/// Access the Managed Agents — Agents API (beta).
pub fn agents(&self) -> AgentsApi { AgentsApi::new(self.clone()) }
/// Access the Managed Agents — Environments API (beta).
pub fn environments(&self) -> EnvironmentsApi { EnvironmentsApi::new(self.clone()) }
/// Access the Managed Agents — Sessions API (beta).
pub fn sessions(&self) -> SessionsApi { SessionsApi::new(self.clone()) }
/// Access the Managed Agents — Vaults API (beta).
pub fn vaults(&self) -> VaultsApi { VaultsApi::new(self.clone()) }
/// Access the Managed Agents — Memory Stores API (beta).
pub fn memory_stores(&self) -> MemoryStoresApi { MemoryStoresApi::new(self.clone()) }
/// Access the Managed Agents — Deployments API (beta).
pub fn deployments(&self) -> DeploymentsApi { DeploymentsApi::new(self.clone()) }
```

Session sub-resources are reached through the `SessionsApi` to avoid passing `session_id` into a client accessor:

```rust
impl SessionsApi {
    pub fn events(&self, session_id: &str) -> SessionEventsApi { ... }
    pub fn resources(&self, session_id: &str) -> SessionResourcesApi { ... }
    pub fn threads(&self, session_id: &str) -> SessionThreadsApi { ... }
}
// likewise VaultsApi::credentials(vault_id), MemoryStoresApi::memories(store_id),
//          DeploymentsApi::runs(deployment_id)
```

Each sub-API stores `client: Client` plus the parent id, so it can build the nested path. This keeps the `XApi { client }` shape while threading the parent id ergonomically.

---

## 2. Typed Model Structs (illustrative)

All structs derive `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`, use `#[serde(rename = "type")] pub object_type: String` for the discriminator, `#[serde(skip_serializing_if = "Option::is_none")]` on options, and carry a `#[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>` forward-compat field. Timestamps are `chrono::DateTime<Utc>` (matching `batch.rs`).

### 2.1 Agent — `src/models/managed_agents/agent.rs`

```rust
/// A versioned managed agent. Create once; reference from sessions by id (+ optional version).
pub struct Agent {
    #[serde(rename = "type")] pub object_type: String,   // "agent"
    pub id: String,
    pub version: String,                                 // each update mints a new version
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String>,
    pub model: AgentModel,
    #[serde(skip_serializing_if = "Option::is_none")] pub system: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub tools: Vec<AgentTool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub mcp_servers: Vec<McpServer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub skills: Vec<AgentSkillRef>,
    #[serde(skip_serializing_if = "Option::is_none")] pub multiagent: Option<Multiagent>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")] pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub created_at: Option<DateTime<Utc>>,
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

/// Model spec — a bare id string OR {id, speed}. Untagged so both shapes deserialize.
#[serde(untagged)]
pub enum AgentModel {
    Id(String),
    Spec { id: String, #[serde(skip_serializing_if = "Option::is_none")] speed: Option<String> },
}

/// Tools attachable to an agent.
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTool {
    #[serde(rename = "agent_toolset_20260401")]
    AgentToolset { /* built-in toolset flags */ #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
    McpToolset { name: String, #[serde(default)] allowed_tools: Vec<String> },
    Custom { name: String, description: String, input_schema: serde_json::Value },
}

/// MCP server reference: {type: "url", name, url}.
pub struct McpServer {
    #[serde(rename = "type")] pub server_type: String,   // "url"
    pub name: String,
    pub url: String,
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

/// Multiagent coordinator config: {type: "coordinator", agents: [...]}.
pub struct Multiagent {
    #[serde(rename = "type")] pub kind: String,          // "coordinator"
    pub agents: Vec<MultiagentMember>,                   // sub-agent id/version refs
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

/// Create request — model/system/tools live here (NOT on sessions).
pub struct AgentCreateRequest {
    pub name: String,
    pub model: AgentModel,
    #[serde(skip_serializing_if = "Option::is_none")] pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub tools: Vec<AgentTool>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub mcp_servers: Vec<McpServer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub skills: Vec<AgentSkillRef>,
    #[serde(skip_serializing_if = "Option::is_none")] pub multiagent: Option<Multiagent>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")] pub metadata: HashMap<String, String>,
}
// + builder methods (.name(), .model(), .system(), .add_tool(), .add_mcp_server(), .multiagent())
// AgentUpdateRequest mirrors create with all-Option fields (each update => new version).
pub type AgentListResponse = PaginatedResponse<Agent>;
```

### 2.2 Environment — `src/models/managed_agents/environment.rs`

```rust
pub struct Environment {
    #[serde(rename = "type")] pub object_type: String,   // "environment"
    pub id: String,
    pub name: String,
    pub config: EnvironmentConfig,
    #[serde(skip_serializing_if = "Option::is_none")] pub created_at: Option<DateTime<Utc>>,
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

#[serde(tag = "type", rename_all = "snake_case")]
pub enum EnvironmentConfig {
    Cloud { networking: NetworkingConfig },
    SelfHosted { networking: NetworkingConfig, #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
}

#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkingConfig {
    Unrestricted {},
    Limited {
        #[serde(default)] allow_package_managers: bool,
        #[serde(default)] allow_mcp_servers: bool,
        #[serde(default, skip_serializing_if = "Vec::is_empty")] allowed_hosts: Vec<String>,
    },
}
// EnvironmentCreateRequest / EnvironmentUpdateRequest + EnvironmentListResponse = PaginatedResponse<Environment>
```

### 2.3 Session — `src/models/managed_agents/session.rs`

```rust
pub struct Session {
    #[serde(rename = "type")] pub object_type: String,   // "session"
    pub id: String,
    pub agent: SessionAgentRef,
    #[serde(skip_serializing_if = "Option::is_none")] pub environment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub title: Option<String>,
    pub status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")] pub stop_reason: Option<SessionStopReason>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub resources: Vec<SessionResourceSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub vault_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")] pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub created_at: Option<DateTime<Utc>>,
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

/// Sessions take an agent id string OR {type:"agent", id, version} — NEVER inline model/system/tools.
#[serde(untagged)]
pub enum SessionAgentRef {
    Id(String),
    Ref { #[serde(rename = "type")] kind: String /* "agent" */, id: String,
          #[serde(skip_serializing_if = "Option::is_none")] version: Option<String> },
}

#[serde(rename_all = "snake_case")]
pub enum SessionStatus { Rescheduling, Running, Idle, Terminated }

/// Why the session went idle.
#[serde(rename_all = "snake_case")]
pub enum SessionStopReason { EndTurn, AwaitingInput, ToolConfirmation, OutcomeDefined, /* ... */
    #[serde(other)] Other }

#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionResourceSpec {
    File { file_id: String, #[serde(skip_serializing_if = "Option::is_none")] mount_path: Option<String> },
    GithubRepository {
        url: String,
        #[serde(skip_serializing_if = "Option::is_none")] authorization_token: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")] mount_path: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")] checkout: Option<String>,
    },
    MemoryStore {
        memory_store_id: String,
        #[serde(skip_serializing_if = "Option::is_none")] access: Option<String>,   // read | read_write
        #[serde(skip_serializing_if = "Option::is_none")] instructions: Option<String>,
    },
}

pub struct SessionCreateRequest {
    pub agent: SessionAgentRef,
    #[serde(skip_serializing_if = "Option::is_none")] pub environment_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub resources: Vec<SessionResourceSpec>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub vault_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")] pub metadata: HashMap<String, String>,
}
pub type SessionListResponse = PaginatedResponse<Session>;
```

### 2.4 Vault & Credential — `src/models/managed_agents/vault.rs`

```rust
pub struct Vault {
    #[serde(rename = "type")] pub object_type: String,   // "vault"
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub created_at: Option<DateTime<Utc>>,
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

pub struct Credential {
    #[serde(rename = "type")] pub object_type: String,   // "credential"
    pub id: String,
    pub name: String,
    pub kind: CredentialKind,            // tagged below; secret material is write-only
    #[serde(skip_serializing_if = "Option::is_none")] pub created_at: Option<DateTime<Utc>>,
    #[serde(flatten, default)] pub extra: HashMap<String, serde_json::Value>,
}

/// Secret payload — write-only on create; reads return metadata only.
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialKind {
    McpOauth { /* oauth config */ #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
    StaticBearer { #[serde(skip_serializing_if = "Option::is_none")] token: Option<String> },
    EnvironmentVariable { name: String, #[serde(skip_serializing_if = "Option::is_none")] value: Option<String> },
}
pub type VaultListResponse = PaginatedResponse<Vault>;
pub type CredentialListResponse = PaginatedResponse<Credential>;
```

### 2.5 SessionEvent enum — see §5.

---

## 3. Client Method Signatures Per Resource

Every method follows the `message_batches.rs` shape: build `path`, optional `serde_json::to_value(request)?` body, delegate to `self.client.request(...)` with `with_managed_agents_beta(options)`. Archive/delete return `()` (parse-and-discard like `MessageBatchesApi::delete`).

### 3.1 AgentsApi — `/v1/agents`

```rust
async fn create(&self, request: AgentCreateRequest, options: Option<RequestOptions>) -> Result<Agent>;
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<AgentListResponse>;
async fn get(&self, agent_id: &str, options: Option<RequestOptions>) -> Result<Agent>;
async fn get_version(&self, agent_id: &str, version: &str, options: Option<RequestOptions>) -> Result<Agent>;
async fn update(&self, agent_id: &str, request: AgentUpdateRequest, options: Option<RequestOptions>) -> Result<Agent>; // mints new version
async fn archive(&self, agent_id: &str, options: Option<RequestOptions>) -> Result<Agent>; // POST /v1/agents/{id}/archive
```

### 3.2 EnvironmentsApi — `/v1/environments`

```rust
async fn create(&self, request: EnvironmentCreateRequest, options: Option<RequestOptions>) -> Result<Environment>;
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<EnvironmentListResponse>;
async fn get(&self, environment_id: &str, options: Option<RequestOptions>) -> Result<Environment>;
async fn update(&self, environment_id: &str, request: EnvironmentUpdateRequest, options: Option<RequestOptions>) -> Result<Environment>;
async fn delete(&self, environment_id: &str, options: Option<RequestOptions>) -> Result<()>;
async fn archive(&self, environment_id: &str, options: Option<RequestOptions>) -> Result<Environment>;
```

### 3.3 SessionsApi — `/v1/sessions`

```rust
async fn create(&self, request: SessionCreateRequest, options: Option<RequestOptions>) -> Result<Session>;
async fn get(&self, session_id: &str, options: Option<RequestOptions>) -> Result<Session>;
async fn update(&self, session_id: &str, request: SessionUpdateRequest, options: Option<RequestOptions>) -> Result<Session>;
async fn delete(&self, session_id: &str, options: Option<RequestOptions>) -> Result<()>;
async fn archive(&self, session_id: &str, options: Option<RequestOptions>) -> Result<Session>;
// sub-resource accessors:
fn events(&self, session_id: &str) -> SessionEventsApi;
fn resources(&self, session_id: &str) -> SessionResourcesApi;
fn threads(&self, session_id: &str) -> SessionThreadsApi;
// convenience: poll get() until status==Idle/Terminated (mirrors MessageBatchesApi::wait_for_completion)
async fn wait_until_idle(&self, session_id: &str, poll: Duration, max_wait: Duration) -> Result<Session>;
```

### 3.4 SessionEventsApi — `/v1/sessions/{id}/events`

```rust
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<SessionEventListResponse>;
async fn send(&self, event: SendEvent, options: Option<RequestOptions>) -> Result<SessionEvent>;   // POST .../events
async fn stream(&self, options: Option<RequestOptions>) -> Result<SessionEventStream>;              // GET .../events/stream (SSE)
// ergonomic send helpers built on send():
async fn send_user_message(&self, text: impl Into<String>, options: Option<RequestOptions>) -> Result<SessionEvent>;
async fn interrupt(&self, options: Option<RequestOptions>) -> Result<SessionEvent>;
async fn confirm_tool(&self, tool_use_id: &str, approve: bool, options: Option<RequestOptions>) -> Result<SessionEvent>;
async fn custom_tool_result(&self, tool_use_id: &str, content: serde_json::Value, options: Option<RequestOptions>) -> Result<SessionEvent>;
async fn define_outcome(&self, outcome: serde_json::Value, options: Option<RequestOptions>) -> Result<SessionEvent>;
async fn system_message(&self, text: impl Into<String>, options: Option<RequestOptions>) -> Result<SessionEvent>;
```

### 3.5 SessionResourcesApi — `/v1/sessions/{id}/resources`

```rust
async fn add(&self, resource: SessionResourceSpec, options: Option<RequestOptions>) -> Result<SessionResource>;
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<SessionResourceListResponse>;
async fn get(&self, resource_id: &str, options: Option<RequestOptions>) -> Result<SessionResource>;
async fn update(&self, resource_id: &str, request: SessionResourceUpdateRequest, options: Option<RequestOptions>) -> Result<SessionResource>;
async fn delete(&self, resource_id: &str, options: Option<RequestOptions>) -> Result<()>;
```

### 3.6 SessionThreadsApi — `/v1/sessions/{id}/threads` (multiagent)

```rust
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<SessionThreadListResponse>;
async fn get(&self, thread_id: &str, options: Option<RequestOptions>) -> Result<SessionThread>;
async fn archive(&self, thread_id: &str, options: Option<RequestOptions>) -> Result<SessionThread>;
async fn list_events(&self, thread_id: &str, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<SessionEventListResponse>;
async fn stream_events(&self, thread_id: &str, options: Option<RequestOptions>) -> Result<SessionEventStream>;
```

### 3.7 VaultsApi / CredentialsApi — `/v1/vaults`, `/v1/vaults/{id}/credentials`

```rust
// VaultsApi
async fn create(&self, request: VaultCreateRequest, options: Option<RequestOptions>) -> Result<Vault>;
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<VaultListResponse>;
async fn get(&self, vault_id: &str, options: Option<RequestOptions>) -> Result<Vault>;
async fn update(&self, vault_id: &str, request: VaultUpdateRequest, options: Option<RequestOptions>) -> Result<Vault>;
async fn delete(&self, vault_id: &str, options: Option<RequestOptions>) -> Result<()>;
fn credentials(&self, vault_id: &str) -> CredentialsApi;
// CredentialsApi
async fn create(&self, request: CredentialCreateRequest, options: Option<RequestOptions>) -> Result<Credential>;
async fn list(&self, pagination: Option<Pagination>, options: Option<RequestOptions>) -> Result<CredentialListResponse>;
async fn get(&self, credential_id: &str, options: Option<RequestOptions>) -> Result<Credential>;
async fn update(&self, credential_id: &str, request: CredentialUpdateRequest, options: Option<RequestOptions>) -> Result<Credential>;
async fn delete(&self, credential_id: &str, options: Option<RequestOptions>) -> Result<()>;
```

### 3.8 MemoryStoresApi — `/v1/memory_stores` (+ memories, memory_versions)

```rust
// MemoryStoresApi
async fn create/list/get/update/delete(...);
fn memories(&self, store_id: &str) -> MemoriesApi;
// MemoriesApi -> /v1/memory_stores/{id}/memories
async fn create/list/get/update/delete(...);
async fn redact(&self, memory_id: &str, request: MemoryRedactRequest, options) -> Result<Memory>; // POST .../redact
async fn list_versions(&self, memory_id: &str, pagination, options) -> Result<MemoryVersionListResponse>;
async fn get_version(&self, memory_id: &str, version_id: &str, options) -> Result<MemoryVersion>;
```

### 3.9 DeploymentsApi — `/v1/deployments` (+ deployment_runs)

```rust
async fn create/list/get/update/delete(...);   // schedule carries a cron expression
fn runs(&self, deployment_id: &str) -> DeploymentRunsApi;
// DeploymentRunsApi -> /v1/deployments/{id}/runs (or /v1/deployment_runs?deployment_id=)
async fn list(&self, pagination, options) -> Result<DeploymentRunListResponse>;
async fn get(&self, run_id: &str, options) -> Result<DeploymentRun>;
async fn trigger(&self, options) -> Result<DeploymentRun>;   // POST .../runs (manual run)
```

---

## 4. Beta-Header Handling

`beta_headers::MANAGED_AGENTS = "managed-agents-2026-04-01"` already exists in `src/client.rs`. We do **not** need to touch `build_headers` because it already appends `options.beta_features` to the `anthropic-beta` header. Two changes:

1. **Force-inject in every Managed-Agents module** via `managed_agents::with_managed_agents_beta(options)` (mirrors `SkillsApi::with_skills_beta`). This guarantees the header is present even when the caller passes `None`, and de-dups naturally because `beta_features` is a `Vec` joined with `,` (acceptable; the API tolerates a repeated value, but we can `dedup` in the helper to be safe).

2. **Add an ergonomic `RequestOptions` toggle** in `src/types.rs` to match the existing pattern (`with_skills_api`, `with_mcp_client`, etc.):

```rust
/// Enable the Managed Agents beta feature.
pub fn with_managed_agents(self) -> Self {
    self.with_beta_feature(crate::client::beta_headers::MANAGED_AGENTS)
}
```

**Sub-betas / interaction with Files and Skills:**

- **Files for session outputs**: the Files API gains a `scope_id` parameter (session outputs are scoped files). Sessions that produce file outputs require both `files-api-2025-04-14` and `managed-agents-2026-04-01`. When a Managed-Agents call also needs Files, the caller (or our convenience methods) sets `options.with_files_api()` in addition — both headers get comma-joined automatically. `FilesApi::list`/`get` should accept an optional `scope_id` query param (new `FileListParams { scope_id, purpose }`) so session-scoped outputs are listable. This is an additive, backward-compatible change to `src/api/files.rs` / `src/models/file.rs`.
- **Skills**: Agents reference skills by id/version (`AgentSkillRef { skill_id, version }`). Creating those skills still goes through the existing `SkillsApi` (which force-injects `skills-2025-10-02`). When an agent references skills, only the agent-create call needs the managed-agents header; no new skills sub-beta. The existing `SkillsApi` is **extended, not duplicated** — we add nothing there beyond what already exists. The reconciliation is purely at the model layer (`AgentSkillRef` points at `skill.id` / `skill_version.id`).

---

## 5. Event-Streaming Design

### 5.1 `SessionEvent` model — `src/models/managed_agents/session_event.rs`

The heterogeneous SSE event set is modeled as **one internally-tagged enum**, mirroring how `MessageBatchResult` uses `#[serde(tag = "type", rename_all = "snake_case")]`. Every event carries the common envelope fields `id`, `type`, `processed_at`; we model these by giving each variant the shared fields, OR — cleaner — wrap the enum in an envelope struct and flatten:

```rust
/// Common fields present on every session event.
pub struct SessionEventMeta {
    pub id: String,
    pub processed_at: DateTime<Utc>,
}

/// All session events. The wire `type` selects the variant; dotted names map via rename.
#[serde(tag = "type")]
pub enum SessionEvent {
    // ---- received (agent-originated) ----
    #[serde(rename = "agent.message")]        AgentMessage { #[serde(flatten)] meta: SessionEventMeta, content: Vec<ContentBlock>, /* ... */ },
    #[serde(rename = "agent.thinking")]       AgentThinking { #[serde(flatten)] meta: SessionEventMeta, thinking: String },
    #[serde(rename = "agent.tool_use")]       AgentToolUse { #[serde(flatten)] meta: SessionEventMeta, tool_use_id: String, name: String, input: serde_json::Value },
    #[serde(rename = "agent.tool_result")]    AgentToolResult { #[serde(flatten)] meta: SessionEventMeta, tool_use_id: String, content: serde_json::Value },
    #[serde(rename = "agent.mcp_tool_use")]   AgentMcpToolUse { #[serde(flatten)] meta: SessionEventMeta, /* ... */ },
    #[serde(rename = "agent.custom_tool_use")] AgentCustomToolUse { #[serde(flatten)] meta: SessionEventMeta, /* ... */ },
    #[serde(rename = "session.status_running")]     StatusRunning { #[serde(flatten)] meta: SessionEventMeta },
    #[serde(rename = "session.status_idle")]        StatusIdle { #[serde(flatten)] meta: SessionEventMeta, #[serde(skip_serializing_if="Option::is_none")] stop_reason: Option<SessionStopReason> },
    #[serde(rename = "session.status_rescheduled")] StatusRescheduled { #[serde(flatten)] meta: SessionEventMeta },
    #[serde(rename = "session.status_terminated")]  StatusTerminated { #[serde(flatten)] meta: SessionEventMeta },
    #[serde(rename = "session.error")]        SessionError { #[serde(flatten)] meta: SessionEventMeta, error: serde_json::Value },
    #[serde(rename = "span.model_request_start")] SpanModelRequestStart { #[serde(flatten)] meta: SessionEventMeta, #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
    #[serde(rename = "span.model_request_end")]   SpanModelRequestEnd { #[serde(flatten)] meta: SessionEventMeta, #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
    #[serde(rename = "span.outcome_evaluation_start")] SpanOutcomeEvalStart { #[serde(flatten)] meta: SessionEventMeta, #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
    #[serde(rename = "span.outcome_evaluation_end")]   SpanOutcomeEvalEnd { #[serde(flatten)] meta: SessionEventMeta, #[serde(flatten, default)] extra: HashMap<String, serde_json::Value> },
    #[serde(rename = "session.thread_started")]   ThreadStarted { #[serde(flatten)] meta: SessionEventMeta, thread_id: String, /* ... */ },
    #[serde(rename = "session.thread_completed")] ThreadCompleted { #[serde(flatten)] meta: SessionEventMeta, thread_id: String, /* ... */ },

    // ---- sent (client-originated) — reused for send() request bodies & echoed events ----
    #[serde(rename = "user.message")]           UserMessage { content: Vec<ContentBlock> },
    #[serde(rename = "user.interrupt")]         UserInterrupt {},
    #[serde(rename = "user.tool_confirmation")] UserToolConfirmation { tool_use_id: String, approve: bool },
    #[serde(rename = "user.custom_tool_result")] UserCustomToolResult { tool_use_id: String, content: serde_json::Value },
    #[serde(rename = "user.define_outcome")]    UserDefineOutcome { outcome: serde_json::Value },
    #[serde(rename = "system.message")]         SystemMessage { content: String },

    /// Forward-compat catch-all for event types added after this SDK release.
    #[serde(other)]
    Unknown,
}
```

> **Design notes:**
> - Dotted wire names (`agent.message`, `session.status_idle`) can't be produced by `rename_all`, so each variant gets an explicit `#[serde(rename = "...")]` — same technique the SDK already uses for aliases.
> - `#[serde(other)] Unknown` guarantees the stream never hard-fails on an unrecognized event type (the message `EventParser` instead `warn!`s and drops unknowns; here we keep the event but as `Unknown` so the consumer can still observe ordering). For richer forward-compat we may instead model `Unknown { #[serde(flatten)] raw: serde_json::Value }` via a custom deserializer, but `#[serde(other)]` is the minimal/idiomatic first cut.
> - For `send()`, we define a separate lean `SendEvent` enum (the `user.*` / `system.*` subset) so the API surface doesn't let callers "send" an agent event. The `SessionEvent` enum above is the read model.

### 5.2 `SessionEventStream` — `src/streaming/session_event_stream.rs`

New stream type that reuses the **exact channel-pump pattern** from `MessageStream` (`src/streaming/message_stream.rs`): spawn a task that reads `response.bytes_stream()`, splits on newlines, feeds an `EventParser`, and forwards `Result<SessionEvent>` over an `mpsc::channel(100)`. Implements `futures::Stream<Item = Result<SessionEvent>>` + `FusedStream`.

Two implementation options for parsing:

- **Option A (preferred):** generalize `EventParser` to be generic over the output type, or add a sibling `parse_session_line` that deserializes the SSE `data:` payload straight into `SessionEvent` (the enum is internally tagged on `type`, and the SSE `event:` line also carries the type — so we can ignore `event:` and just `serde_json::from_str::<SessionEvent>(data)`). This is *simpler* than the message parser because there's no multi-line content-block reassembly — each event is a complete JSON object.
- **Option B:** copy `EventParser`'s SSE field-accumulation logic into a small `SseLineBuffer` shared by both streams, then `serde_json::from_str` the assembled `data`.

We will factor the SSE framing (the `data:`/`event:`/blank-line accumulation in `message_stream.rs`) into a reusable `streaming::sse::SseFrameReader`, and have both `MessageStream` and `SessionEventStream` consume frames from it. This avoids duplicating the byte-buffer/newline logic.

```rust
pub struct SessionEventStream { receiver: mpsc::Receiver<Result<SessionEvent>>, _handle: JoinHandle<()> }
impl SessionEventStream { pub async fn new(response: reqwest::Response) -> Result<Self> { /* same as MessageStream::new */ } }
impl Stream for SessionEventStream { type Item = Result<SessionEvent>; /* poll_recv */ }
```

`SessionEventsApi::stream()` / `SessionThreadsApi::stream_events()` call `self.client.request_stream(HttpMethod::Get, path, None, with_managed_agents_beta(options))` then `SessionEventStream::new(response).await` — identical to how `MessagesApi::create_stream` consumes `request_stream`.

Register in `src/streaming/mod.rs`: `pub mod session_event_stream; pub use session_event_stream::SessionEventStream;` and re-export from `src/lib.rs`.

---

## 6. Phased Delivery

Each phase is independently shippable (compiles, tests green, usable on its own). PR sizes assume the SDK's existing review cadence.

### Phase 0 — Core: Agents + Sessions + Events (the MVP loop)

Enables: create agent → create session → drive it via events (send + stream). This is the minimum that makes Managed Agents usable.

- `src/models/managed_agents/{mod,agent,session,session_event}.rs`
- `src/api/managed_agents/{mod,agents,sessions,session_events}.rs`
- `src/streaming/session_event_stream.rs` + SSE framing refactor (`streaming::sse`)
- `Client::agents()`, `Client::sessions()`; `SessionsApi::events()`
- `RequestOptions::with_managed_agents()` in `src/types.rs`
- lib/mod re-exports
- Tests: wiremock CRUD for agents+sessions; SSE stream test for session events; serde round-trip tests for `SessionEvent` variants.

**~3 PRs**: (1) models + serde tests, (2) agents/sessions API + client wiring, (3) session events + streaming.

### Phase 1 — Environments + Session Resources + Vaults/Credentials

Enables: sandbox/networking config, mounting files/repos/memory into sessions, secret injection for MCP/OAuth.

- `src/models/managed_agents/{environment,vault}.rs` + `session_resource` types
- `src/api/managed_agents/{environments,session_resources,vaults}.rs`
- `Client::environments()`, `Client::vaults()`; `SessionsApi::resources()`, `VaultsApi::credentials()`
- Files API additive change: `scope_id` query param + `FileListParams`
- Tests: wiremock CRUD for each; credential write-only round-trip; scope_id filter on files list.

**~2–3 PRs**: (1) environments + files scope_id, (2) session resources, (3) vaults + credentials.

### Phase 2 — Memory Stores + Deployments + Multiagent Threads

Enables: persistent memory with versioning/redaction, cron-scheduled sessions, observing per-thread events in coordinator multiagent runs.

- `src/models/managed_agents/{memory,deployment}.rs`; thread types in `session_event`/new `session_thread`
- `src/api/managed_agents/{memory_stores,deployments,session_threads}.rs`
- `Client::memory_stores()`, `Client::deployments()`; `SessionsApi::threads()`, `MemoryStoresApi::memories()`, `DeploymentsApi::runs()`
- Tests: wiremock CRUD + memory redaction + memory_versions; deployment run trigger; thread events stream.

**~3 PRs**: (1) memory stores + memories + versions, (2) deployments + runs, (3) multiagent threads + per-thread event streaming.

---

## 7. Testing Strategy

Mirror the existing `tests/` split (`tests/unit/*_test.rs`, `tests/integration/*_test.rs`, `tests/real_api/*`, shared helpers in `tests/common/mod.rs`).

1. **Serde round-trip unit tests** (`tests/unit/managed_agents_test.rs`, plus `#[cfg(test)]` mods inside each model file like `message.rs` does):
   - `AgentModel` untagged: both `"claude-..."` and `{"id":...,"speed":...}` deserialize.
   - `SessionAgentRef` untagged: bare id string vs `{type:agent,id,version}`.
   - `EnvironmentConfig` / `NetworkingConfig` tagged-enum round-trips.
   - `SessionResourceSpec` all three variants.
   - `SessionEvent`: one test per wire `type` (parse the documented JSON shape → assert the right variant) + an `Unknown` test feeding a bogus `type` to prove forward-compat doesn't panic.
   - `CredentialKind` write-only: serialize includes secret, deserialize from metadata-only response works.

2. **Mock-server integration tests** (`tests/integration/managed_agents_test.rs`) using `wiremock` exactly like `tests/common/mod.rs::mock_server`:
   - Add fixtures (`test_agent()`, `test_session()`, `test_environment()`, `test_vault()`, ...) and `mock_*` helpers to `tests/common/mod.rs`.
   - Assert each method hits the right `method`+`path` and that the `anthropic-beta` header contains `managed-agents-2026-04-01` (use `wiremock::matchers::header_regex` / a contains-matcher, since the header is comma-joined).
   - Cover archive (POST `/archive`) vs delete (DELETE) distinctly.
   - Pagination: assert `?limit=&after=` query construction via `build_paginated_path`.

3. **SSE stream tests** (`tests/unit/session_stream_test.rs`): mirror `mock_message_stream` in `tests/common/mod.rs`. Build a `text/event-stream` body of concatenated `event:`/`data:` frames covering `agent.message`, `agent.tool_use`, `session.status_idle` (with `stop_reason`), `session.error`, and a `session.thread_*` event; drive `SessionEventStream` to completion and assert the decoded variant sequence. Include a malformed-JSON frame test to assert the stream yields `Err` and terminates cleanly (matching `MessageStream` error behavior).

4. **Feature-gated real-API tests** (`tests/real_api/managed_agents_test.rs`, gated by the existing `real_api_tests` feature + `RUN_REAL_API_TESTS=true` per `tests/common/mod.rs::env`): a single end-to-end smoke — create environment → create agent → create session → send `user.message` → stream until `session.status_idle` → archive everything. Gated because it consumes real compute and requires the beta to be enabled on the key.

5. **Doc tests**: `rust,no_run` examples on the headline methods (`agents().create`, `sessions().create`, `sessions().events(id).stream`) matching the doc-comment style in `message_batches.rs`. Add an `examples/managed_agent_session.rs` (gated on env key) like the other `examples/*`.

---

## 8. Risks, Unknowns, Open Questions

### Risks / unknowns

- **Exact wire shapes are inferred.** The field lists here come from the prompt's API summary, not a fetched OpenAPI spec. The `#[serde(flatten) extra]` + `#[serde(other)] Unknown` pattern is the mitigation: unknown fields/events are preserved/tolerated rather than causing deserialize failures. Field names (`processed_at`, `stop_reason`, `mount_path`, `authorization_token`, `checkout`, `access`, `instructions`) should be validated against a live response or the official spec before GA.
- **Event envelope shape.** Whether `id`/`type`/`processed_at` sit at the top level alongside per-type fields (assumed here via `#[serde(flatten) meta]`) vs nested under a `data` object changes the enum layout. Needs one real SSE capture to confirm; the `SseFrameReader` refactor isolates the blast radius.
- **Sent vs received event symmetry.** I modeled `SendEvent` (lean, client-only) separately from the read-side `SessionEvent`. If the API actually round-trips the same schema, we can collapse them; keeping them separate is the safer default and is easy to merge later.
- **List-pagination style.** Assumed cursor-style (`after`/`before`/`has_more`, like batches) for all Managed-Agents lists. Skills uses `page` tokens — if any Managed-Agents endpoint also uses page tokens, that resource needs a bespoke `*ListParams` instead of `types::Pagination`.
- **Files `scope_id` coupling.** Whether session output files need *only* the managed-agents beta or *also* the files beta is unconfirmed; plan assumes both and sets both headers. Worst case is a harmless extra beta value.
- **Credential secret handling.** Secrets are write-only by assumption (reads return metadata). If the API ever echoes secrets we must ensure `Debug` doesn't leak them — add manual `Debug` impls or `#[derive(Debug)]` with redaction on `CredentialKind` to be safe.
- **`#[serde(other)]` limitation.** A unit `Unknown` variant discards the payload. If consumers need the raw bytes of unknown events, upgrade to a custom `Deserialize` that captures `serde_json::Value`. Deferred to avoid over-engineering Phase 0.

### Open questions (for API owners / spec)

1. What is the canonical SSE envelope — are `id`/`type`/`processed_at` top-level or nested? Any `event:` vs `data:` `type` mismatch?
2. Do Managed-Agents list endpoints use cursor (`after`) or page-token pagination? Per-resource?
3. Are archive and delete both available on every resource, or only some (e.g. environments delete vs archive)? What status code/body does archive return?
4. Exact `stop_reason` enum values for `session.status_idle`.
5. Is `deployment_runs` a sub-path of a deployment (`/v1/deployments/{id}/runs`) or a top-level resource (`/v1/deployment_runs`)?
6. Required beta-header combo for session-output Files (managed-agents only, or + files beta)?
7. Multiagent: are sub-agent threads addressed under the parent session (`/v1/sessions/{id}/threads`) only, or independently?

### Rough effort estimate

| Phase | New files (api+models+streaming) | Test files | PRs | Relative size |
|-------|----------------------------------|-----------|-----|---------------|
| P0 (agents+sessions+events) | 4 models + 3 api + 1 streaming + SSE refactor | 3 | ~3 | L (largest — establishes patterns + streaming) |
| P1 (env+resources+vaults) | 2 models + 3 api + files patch | 2–3 | ~2–3 | M |
| P2 (memory+deploy+threads) | 2 models + 3 api | 3 | ~3 | M |

**Totals:** ~8 new model files, ~9 new API files, 1 new streaming file + 1 SSE-framing refactor, ~8 test files, ~8 PRs. Touch-points to existing code are small and additive: `src/client.rs` (6 accessors), `src/api/mod.rs` + `src/models/mod.rs` + `src/lib.rs` (re-exports), `src/types.rs` (one `with_managed_agents()`), `src/streaming/mod.rs`, and a backward-compatible `scope_id` addition to `src/api/files.rs` + `src/models/file.rs`. No breaking changes to existing public API.
