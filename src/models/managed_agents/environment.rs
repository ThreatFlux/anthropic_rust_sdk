//! Managed Agents — Environment data models (beta: managed-agents-2026-04-01)

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Networking policy for an environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkingConfig {
    /// Unrestricted outbound networking.
    Unrestricted {},
    /// Restricted outbound networking with explicit allowances.
    Limited {
        /// Allow access to package managers.
        #[serde(default)]
        allow_package_managers: bool,
        /// Allow access to configured MCP servers.
        #[serde(default)]
        allow_mcp_servers: bool,
        /// Explicitly allowed hosts.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allowed_hosts: Vec<String>,
    },
}

/// Environment configuration (cloud-managed vs self-hosted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EnvironmentConfig {
    /// Anthropic-managed cloud environment.
    Cloud {
        /// Networking policy.
        networking: NetworkingConfig,
    },
    /// Customer self-hosted environment.
    SelfHosted {
        /// Networking policy.
        networking: NetworkingConfig,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
}

/// A managed-agents execution environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Environment {
    /// Object type (always `"environment"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique environment identifier.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// Environment configuration.
    pub config: EnvironmentConfig,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Request body for creating an environment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvironmentCreateRequest {
    /// Human-friendly name.
    pub name: String,
    /// Environment configuration.
    pub config: EnvironmentConfig,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl EnvironmentCreateRequest {
    /// Create a new environment create request.
    pub fn new(name: impl Into<String>, config: EnvironmentConfig) -> Self {
        Self {
            name: name.into(),
            config,
            metadata: HashMap::new(),
        }
    }

    /// Insert a metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Request body for updating an environment.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EnvironmentUpdateRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<EnvironmentConfig>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl EnvironmentUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the configuration.
    pub fn config(mut self, config: EnvironmentConfig) -> Self {
        self.config = Some(config);
        self
    }
}

/// Response when listing environments (cursor-style pagination).
pub type EnvironmentListResponse = PaginatedResponse<Environment>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn networking_config_tagged_unrestricted() {
        let parsed: NetworkingConfig = serde_json::from_str(r#"{"type":"unrestricted"}"#).unwrap();
        assert!(matches!(parsed, NetworkingConfig::Unrestricted {}));
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "unrestricted");
    }

    #[test]
    fn networking_config_tagged_limited() {
        let parsed: NetworkingConfig = serde_json::from_str(
            r#"{"type":"limited","allow_package_managers":true,"allowed_hosts":["github.com"]}"#,
        )
        .unwrap();
        match &parsed {
            NetworkingConfig::Limited {
                allow_package_managers,
                allow_mcp_servers,
                allowed_hosts,
            } => {
                assert!(allow_package_managers);
                assert!(!allow_mcp_servers);
                assert_eq!(allowed_hosts, &["github.com".to_string()]);
            }
            _ => panic!("expected limited"),
        }
    }

    #[test]
    fn environment_config_tagged_round_trip() {
        let cfg = EnvironmentConfig::Cloud {
            networking: NetworkingConfig::Unrestricted {},
        };
        let value = serde_json::to_value(&cfg).unwrap();
        assert_eq!(value["type"], "cloud");
        assert_eq!(value["networking"]["type"], "unrestricted");
        let back: EnvironmentConfig = serde_json::from_value(value).unwrap();
        assert_eq!(back, cfg);
    }
}
