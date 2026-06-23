//! Managed Agents — Vault & Credential data models (beta: managed-agents-2026-04-01)

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A vault grouping credentials available to sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Vault {
    /// Object type (always `"vault"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique vault identifier.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// The secret material backing a credential.
///
/// Secret payloads are write-only on create; reads return metadata only, so the
/// secret-bearing fields are `Option` and skip when absent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialKind {
    /// OAuth configuration for an MCP server.
    McpOauth {
        /// OAuth configuration / metadata.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// A static bearer token.
    StaticBearer {
        /// The bearer token (write-only).
        #[serde(skip_serializing_if = "Option::is_none")]
        token: Option<String>,
    },
    /// An environment variable injected into the session.
    EnvironmentVariable {
        /// Variable name.
        name: String,
        /// Variable value (write-only).
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
    },
}

/// A credential stored in a vault.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Credential {
    /// Object type (always `"credential"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique credential identifier.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// Secret material (write-only payload; reads return metadata only).
    pub kind: CredentialKind,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Request body for creating a vault.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultCreateRequest {
    /// Human-friendly name.
    pub name: String,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl VaultCreateRequest {
    /// Create a new vault create request.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            metadata: HashMap::new(),
        }
    }

    /// Insert a metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Request body for updating a vault.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct VaultUpdateRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl VaultUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Request body for creating a credential.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CredentialCreateRequest {
    /// Human-friendly name.
    pub name: String,
    /// Secret material.
    pub kind: CredentialKind,
}

impl CredentialCreateRequest {
    /// Create a new credential create request.
    pub fn new(name: impl Into<String>, kind: CredentialKind) -> Self {
        Self {
            name: name.into(),
            kind,
        }
    }
}

/// Request body for updating a credential.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct CredentialUpdateRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New secret material.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<CredentialKind>,
}

impl CredentialUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Response when listing vaults (cursor-style pagination).
pub type VaultListResponse = PaginatedResponse<Vault>;

/// Response when listing credentials (cursor-style pagination).
pub type CredentialListResponse = PaginatedResponse<Credential>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credential_kind_write_only_round_trip() {
        // serialize includes the secret
        let kind = CredentialKind::StaticBearer {
            token: Some("sk-secret".to_string()),
        };
        let value = serde_json::to_value(&kind).unwrap();
        assert_eq!(value["type"], "static_bearer");
        assert_eq!(value["token"], "sk-secret");

        // deserialize from a metadata-only response (no token) still works
        let from_read: CredentialKind =
            serde_json::from_str(r#"{"type":"static_bearer"}"#).unwrap();
        assert_eq!(from_read, CredentialKind::StaticBearer { token: None });
    }

    #[test]
    fn credential_kind_environment_variable() {
        let kind: CredentialKind =
            serde_json::from_str(r#"{"type":"environment_variable","name":"API_KEY","value":"v"}"#)
                .unwrap();
        match kind {
            CredentialKind::EnvironmentVariable { name, value } => {
                assert_eq!(name, "API_KEY");
                assert_eq!(value.as_deref(), Some("v"));
            }
            _ => panic!("expected environment_variable"),
        }
    }
}
