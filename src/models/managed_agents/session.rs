//! Managed Agents — Session data models (beta: managed-agents-2026-04-01)

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Reference to the agent a session runs.
///
/// Sessions take either a bare agent id string or an object
/// `{ type: "agent", id, version }`. They NEVER inline model/system/tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SessionAgentRef {
    /// Bare agent id string.
    Id(String),
    /// Structured agent reference.
    Ref {
        /// Discriminator (always `"agent"`).
        #[serde(rename = "type")]
        kind: String,
        /// Agent identifier.
        id: String,
        /// Optional pinned version.
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
}

impl SessionAgentRef {
    /// Returns the agent id regardless of representation.
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
            Self::Ref { id, .. } => id.as_str(),
        }
    }

    /// Create a structured reference pinned to a specific version.
    pub fn versioned(id: impl Into<String>, version: impl Into<String>) -> Self {
        Self::Ref {
            kind: "agent".to_string(),
            id: id.into(),
            version: Some(version.into()),
        }
    }
}

impl From<String> for SessionAgentRef {
    fn from(value: String) -> Self {
        Self::Id(value)
    }
}

impl From<&str> for SessionAgentRef {
    fn from(value: &str) -> Self {
        Self::Id(value.to_string())
    }
}

/// Lifecycle status of a session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Session is being (re)scheduled onto compute.
    Rescheduling,
    /// Session is actively running.
    Running,
    /// Session is idle, awaiting input or stopped.
    Idle,
    /// Session has terminated.
    Terminated,
    /// Forward-compatible catch-all for unknown statuses.
    #[serde(other)]
    Unknown,
}

/// Why a session went idle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStopReason {
    /// The agent ended its turn.
    EndTurn,
    /// The agent is awaiting client input.
    AwaitingInput,
    /// The agent is awaiting tool-use confirmation.
    ToolConfirmation,
    /// The outcome was defined / satisfied.
    OutcomeDefined,
    /// Forward-compatible catch-all for unknown stop reasons.
    #[serde(other)]
    Other,
}

/// A resource specification mountable into a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SessionResourceSpec {
    /// A file mounted into the session.
    File {
        /// File identifier.
        file_id: String,
        /// Optional mount path inside the session.
        #[serde(skip_serializing_if = "Option::is_none")]
        mount_path: Option<String>,
    },
    /// A GitHub repository cloned into the session.
    GithubRepository {
        /// Repository URL.
        url: String,
        /// Optional authorization token (write-only).
        #[serde(skip_serializing_if = "Option::is_none")]
        authorization_token: Option<String>,
        /// Optional mount path inside the session.
        #[serde(skip_serializing_if = "Option::is_none")]
        mount_path: Option<String>,
        /// Optional git ref to check out.
        #[serde(skip_serializing_if = "Option::is_none")]
        checkout: Option<String>,
    },
    /// A memory store attached to the session.
    MemoryStore {
        /// Memory store identifier.
        memory_store_id: String,
        /// Access level (`read` | `read_write`).
        #[serde(skip_serializing_if = "Option::is_none")]
        access: Option<String>,
        /// Usage instructions for the agent.
        #[serde(skip_serializing_if = "Option::is_none")]
        instructions: Option<String>,
    },
}

/// A session driving a managed agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// Object type (always `"session"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique session identifier.
    pub id: String,
    /// The agent this session runs.
    pub agent: SessionAgentRef,
    /// Environment the session executes in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    /// Optional title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Current lifecycle status.
    pub status: SessionStatus,
    /// Why the session is idle (when applicable).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<SessionStopReason>,
    /// Resources mounted into the session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<SessionResourceSpec>,
    /// Vaults available to the session.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vault_ids: Vec<String>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Whether the session has reached a resting state (idle or terminated).
    pub fn is_resting(&self) -> bool {
        matches!(self.status, SessionStatus::Idle | SessionStatus::Terminated)
    }
}

/// A materialized resource attached to a session.
///
/// The wire shape is the resource spec plus an `id`, e.g.
/// `{"type":"file","id":"res_…","file_id":"…","mount_path":"…"}` — the `type`
/// field is the spec discriminator (`file` / `github_repository` /
/// `memory_store`), so there is no separate object-type field.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionResource {
    /// Unique resource identifier.
    pub id: String,
    /// The resource specification (the `type` discriminator and its fields).
    #[serde(flatten)]
    pub spec: SessionResourceSpec,
}

/// Request body for creating a session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionCreateRequest {
    /// The agent to run.
    pub agent: SessionAgentRef,
    /// Environment to execute in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    /// Optional title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Resources to mount.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<SessionResourceSpec>,
    /// Vaults to attach.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vault_ids: Vec<String>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl SessionCreateRequest {
    /// Create a new session create request for the given agent.
    pub fn new(agent: impl Into<SessionAgentRef>) -> Self {
        Self {
            agent: agent.into(),
            environment_id: None,
            title: None,
            resources: Vec::new(),
            vault_ids: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the environment.
    pub fn environment(mut self, environment_id: impl Into<String>) -> Self {
        self.environment_id = Some(environment_id.into());
        self
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add a resource.
    pub fn add_resource(mut self, resource: SessionResourceSpec) -> Self {
        self.resources.push(resource);
        self
    }

    /// Add a vault.
    pub fn add_vault(mut self, vault_id: impl Into<String>) -> Self {
        self.vault_ids.push(vault_id.into());
        self
    }

    /// Insert a metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Request body for updating a session.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SessionUpdateRequest {
    /// New title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl SessionUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Request body for updating a session resource.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct SessionResourceUpdateRequest {
    /// New mount path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mount_path: Option<String>,
    /// New access level (for memory-store resources).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl SessionResourceUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }
}

/// A multiagent thread within a session (a sub-agent's conversation).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionThread {
    /// Object type (always `"session_thread"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique thread identifier.
    pub id: String,
    /// Parent session identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Optional sub-agent identifier this thread runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Optional thread title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Current lifecycle status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SessionStatus>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Response when listing sessions (cursor-style pagination).
pub type SessionListResponse = PaginatedResponse<Session>;

/// Response when listing session threads (cursor-style pagination).
pub type SessionThreadListResponse = PaginatedResponse<SessionThread>;

/// Response when listing session resources (cursor-style pagination).
pub type SessionResourceListResponse = PaginatedResponse<SessionResource>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_agent_ref_untagged_bare_string() {
        let parsed: SessionAgentRef = serde_json::from_str("\"agent_123\"").unwrap();
        assert!(matches!(parsed, SessionAgentRef::Id(_)));
        assert_eq!(parsed.id(), "agent_123");
        assert_eq!(serde_json::to_string(&parsed).unwrap(), "\"agent_123\"");
    }

    #[test]
    fn session_agent_ref_untagged_object() {
        let parsed: SessionAgentRef =
            serde_json::from_str(r#"{"type":"agent","id":"agent_123","version":"3"}"#).unwrap();
        match &parsed {
            SessionAgentRef::Ref { kind, id, version } => {
                assert_eq!(kind, "agent");
                assert_eq!(id, "agent_123");
                assert_eq!(version.as_deref(), Some("3"));
            }
            _ => panic!("expected ref variant"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "agent");
    }

    #[test]
    fn session_resource_spec_file() {
        let parsed: SessionResourceSpec =
            serde_json::from_str(r#"{"type":"file","file_id":"file_1","mount_path":"/in"}"#)
                .unwrap();
        match &parsed {
            SessionResourceSpec::File {
                file_id,
                mount_path,
            } => {
                assert_eq!(file_id, "file_1");
                assert_eq!(mount_path.as_deref(), Some("/in"));
            }
            _ => panic!("expected file"),
        }
    }

    #[test]
    fn session_resource_spec_github_repository() {
        let parsed: SessionResourceSpec = serde_json::from_str(
            r#"{"type":"github_repository","url":"https://github.com/a/b","checkout":"main"}"#,
        )
        .unwrap();
        match &parsed {
            SessionResourceSpec::GithubRepository { url, checkout, .. } => {
                assert_eq!(url, "https://github.com/a/b");
                assert_eq!(checkout.as_deref(), Some("main"));
            }
            _ => panic!("expected github_repository"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "github_repository");
    }

    #[test]
    fn session_resource_spec_memory_store() {
        let parsed: SessionResourceSpec = serde_json::from_str(
            r#"{"type":"memory_store","memory_store_id":"mem_1","access":"read_write"}"#,
        )
        .unwrap();
        match &parsed {
            SessionResourceSpec::MemoryStore {
                memory_store_id,
                access,
                ..
            } => {
                assert_eq!(memory_store_id, "mem_1");
                assert_eq!(access.as_deref(), Some("read_write"));
            }
            _ => panic!("expected memory_store"),
        }
    }

    #[test]
    fn session_resource_roundtrips() {
        // A materialized resource: the `type` field is the spec discriminator,
        // alongside an `id`. This must both deserialize and re-serialize with a
        // single `type` key (regression test for the object_type/flatten clash).
        let json = serde_json::json!({
            "type": "file",
            "id": "res_1",
            "file_id": "file_1",
            "mount_path": "/workspace/data.csv"
        });
        let res: SessionResource = serde_json::from_value(json).unwrap();
        assert_eq!(res.id, "res_1");
        assert!(matches!(res.spec, SessionResourceSpec::File { .. }));

        let back = serde_json::to_value(&res).unwrap();
        assert_eq!(back["type"], "file");
        assert_eq!(back["id"], "res_1");
        assert_eq!(back["file_id"], "file_1");
        assert_eq!(back["mount_path"], "/workspace/data.csv");
    }

    #[test]
    fn session_stop_reason_forward_compat() {
        let parsed: SessionStopReason = serde_json::from_str("\"some_future_reason\"").unwrap();
        assert_eq!(parsed, SessionStopReason::Other);
    }
}
