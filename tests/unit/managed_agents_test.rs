//! Unit tests for the Managed Agents models (beta: managed-agents-2026-04-01).
//!
//! These mirror the serde round-trip style of `models_test.rs`: parse the
//! documented wire shape, assert the decoded variant/fields, and (where it
//! matters) re-serialize and assert the JSON shape. They cover the untagged
//! enums (`AgentModel`, `SessionAgentRef`), the tagged enums (`EnvironmentConfig`,
//! `NetworkingConfig`, `SessionResourceSpec`, `CredentialKind`), the write-only
//! credential semantics, and request-body serialization.

use std::collections::HashMap;
use threatflux_anthropic_sdk::models::managed_agents::{
    Agent, AgentCreateRequest, AgentModel, AgentSkillRef, AgentTool, CredentialKind,
    EnvironmentConfig, McpServer, NetworkingConfig, SessionAgentRef, SessionCreateRequest,
    SessionResourceSpec,
};

#[cfg(test)]
mod agent_model_tests {
    use super::*;

    #[test]
    fn agent_model_bare_string_round_trip() {
        let parsed: AgentModel = serde_json::from_str("\"claude-opus-4-8\"").unwrap();
        assert!(matches!(parsed, AgentModel::Id(_)));
        assert_eq!(parsed.id(), "claude-opus-4-8");

        // Bare string must serialize back to a bare string (not an object).
        let json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, "\"claude-opus-4-8\"");

        // Full round-trip preserves equality.
        let round: AgentModel = serde_json::from_str(&json).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn agent_model_spec_object_round_trip() {
        let parsed: AgentModel =
            serde_json::from_str(r#"{"id":"claude-opus-4-8","speed":"fast"}"#).unwrap();
        match &parsed {
            AgentModel::Spec { id, speed } => {
                assert_eq!(id, "claude-opus-4-8");
                assert_eq!(speed.as_deref(), Some("fast"));
            }
            other => panic!("expected Spec, got {other:?}"),
        }

        let value = serde_json::to_value(&parsed).unwrap();
        assert!(value.is_object());
        assert_eq!(value["id"], "claude-opus-4-8");
        assert_eq!(value["speed"], "fast");

        let round: AgentModel = serde_json::from_value(value).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn agent_model_spec_object_without_speed_skips_field() {
        let model = AgentModel::Spec {
            id: "claude-sonnet-4-6".to_string(),
            speed: None,
        };
        let value = serde_json::to_value(&model).unwrap();
        assert_eq!(value["id"], "claude-sonnet-4-6");
        // `speed` is `skip_serializing_if = Option::is_none`, so it must be absent.
        assert!(value.get("speed").is_none());
    }
}

#[cfg(test)]
mod session_agent_ref_tests {
    use super::*;

    #[test]
    fn session_agent_ref_bare_string_round_trip() {
        let parsed: SessionAgentRef = serde_json::from_str("\"agent_abc\"").unwrap();
        assert!(matches!(parsed, SessionAgentRef::Id(_)));
        assert_eq!(parsed.id(), "agent_abc");
        assert_eq!(serde_json::to_string(&parsed).unwrap(), "\"agent_abc\"");
    }

    #[test]
    fn session_agent_ref_object_round_trip() {
        let parsed: SessionAgentRef =
            serde_json::from_str(r#"{"type":"agent","id":"agent_abc","version":"7"}"#).unwrap();
        match &parsed {
            SessionAgentRef::Ref { kind, id, version } => {
                assert_eq!(kind, "agent");
                assert_eq!(id, "agent_abc");
                assert_eq!(version.as_deref(), Some("7"));
            }
            other => panic!("expected Ref, got {other:?}"),
        }
        assert_eq!(parsed.id(), "agent_abc");

        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "agent");
        assert_eq!(value["id"], "agent_abc");
        assert_eq!(value["version"], "7");

        let round: SessionAgentRef = serde_json::from_value(value).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn session_agent_ref_versioned_helper() {
        let r = SessionAgentRef::versioned("agent_abc", "2");
        let value = serde_json::to_value(&r).unwrap();
        assert_eq!(value["type"], "agent");
        assert_eq!(value["version"], "2");
    }
}

#[cfg(test)]
mod environment_config_tests {
    use super::*;

    #[test]
    fn networking_config_unrestricted_round_trip() {
        let cfg = NetworkingConfig::Unrestricted {};
        let value = serde_json::to_value(&cfg).unwrap();
        assert_eq!(value["type"], "unrestricted");
        let back: NetworkingConfig = serde_json::from_value(value).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn networking_config_limited_round_trip() {
        let parsed: NetworkingConfig = serde_json::from_str(
            r#"{"type":"limited","allow_package_managers":true,"allow_mcp_servers":false,"allowed_hosts":["github.com","crates.io"]}"#,
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
                assert_eq!(allowed_hosts.len(), 2);
            }
            other => panic!("expected Limited, got {other:?}"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "limited");
        let round: NetworkingConfig = serde_json::from_value(value).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn environment_config_cloud_round_trip() {
        let cfg = EnvironmentConfig::Cloud {
            networking: NetworkingConfig::Unrestricted {},
        };
        let value = serde_json::to_value(&cfg).unwrap();
        assert_eq!(value["type"], "cloud");
        assert_eq!(value["networking"]["type"], "unrestricted");
        let back: EnvironmentConfig = serde_json::from_value(value).unwrap();
        assert_eq!(back, cfg);
    }

    #[test]
    fn environment_config_self_hosted_round_trip() {
        let parsed: EnvironmentConfig = serde_json::from_str(
            r#"{"type":"self_hosted","networking":{"type":"limited","allow_package_managers":false,"allow_mcp_servers":true}}"#,
        )
        .unwrap();
        match &parsed {
            EnvironmentConfig::SelfHosted { networking, .. } => {
                assert!(matches!(networking, NetworkingConfig::Limited { .. }));
            }
            other => panic!("expected SelfHosted, got {other:?}"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "self_hosted");
    }
}

#[cfg(test)]
mod session_resource_spec_tests {
    use super::*;

    #[test]
    fn session_resource_spec_file_round_trip() {
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
            other => panic!("expected File, got {other:?}"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "file");
        let round: SessionResourceSpec = serde_json::from_value(value).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn session_resource_spec_github_repository_round_trip() {
        let parsed: SessionResourceSpec = serde_json::from_str(
            r#"{"type":"github_repository","url":"https://github.com/a/b","authorization_token":"ghp_x","mount_path":"/repo","checkout":"main"}"#,
        )
        .unwrap();
        match &parsed {
            SessionResourceSpec::GithubRepository {
                url,
                authorization_token,
                mount_path,
                checkout,
            } => {
                assert_eq!(url, "https://github.com/a/b");
                assert_eq!(authorization_token.as_deref(), Some("ghp_x"));
                assert_eq!(mount_path.as_deref(), Some("/repo"));
                assert_eq!(checkout.as_deref(), Some("main"));
            }
            other => panic!("expected GithubRepository, got {other:?}"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "github_repository");
        let round: SessionResourceSpec = serde_json::from_value(value).unwrap();
        assert_eq!(round, parsed);
    }

    #[test]
    fn session_resource_spec_memory_store_round_trip() {
        let parsed: SessionResourceSpec = serde_json::from_str(
            r#"{"type":"memory_store","memory_store_id":"mem_1","access":"read_write","instructions":"use sparingly"}"#,
        )
        .unwrap();
        match &parsed {
            SessionResourceSpec::MemoryStore {
                memory_store_id,
                access,
                instructions,
            } => {
                assert_eq!(memory_store_id, "mem_1");
                assert_eq!(access.as_deref(), Some("read_write"));
                assert_eq!(instructions.as_deref(), Some("use sparingly"));
            }
            other => panic!("expected MemoryStore, got {other:?}"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["type"], "memory_store");
        let round: SessionResourceSpec = serde_json::from_value(value).unwrap();
        assert_eq!(round, parsed);
    }
}

#[cfg(test)]
mod credential_kind_tests {
    use super::*;

    #[test]
    fn credential_kind_static_bearer_serialize_includes_secret() {
        // On write, the secret material is serialized.
        let kind = CredentialKind::StaticBearer {
            token: Some("sk-secret".to_string()),
        };
        let value = serde_json::to_value(&kind).unwrap();
        assert_eq!(value["type"], "static_bearer");
        assert_eq!(value["token"], "sk-secret");
    }

    #[test]
    fn credential_kind_static_bearer_deserialize_metadata_only() {
        // On read, the response carries metadata only (no token). It must still
        // deserialize, yielding `None` for the write-only field.
        let from_read: CredentialKind =
            serde_json::from_str(r#"{"type":"static_bearer"}"#).unwrap();
        assert_eq!(from_read, CredentialKind::StaticBearer { token: None });

        // And the metadata-only form must not emit a `token` key on re-serialize.
        let value = serde_json::to_value(&from_read).unwrap();
        assert!(value.get("token").is_none());
    }

    #[test]
    fn credential_kind_environment_variable_round_trip() {
        let kind = CredentialKind::EnvironmentVariable {
            name: "API_KEY".to_string(),
            value: Some("v".to_string()),
        };
        let serialized = serde_json::to_value(&kind).unwrap();
        assert_eq!(serialized["type"], "environment_variable");
        assert_eq!(serialized["name"], "API_KEY");
        assert_eq!(serialized["value"], "v");

        // Metadata-only read (no value).
        let from_read: CredentialKind =
            serde_json::from_str(r#"{"type":"environment_variable","name":"API_KEY"}"#).unwrap();
        match from_read {
            CredentialKind::EnvironmentVariable { name, value } => {
                assert_eq!(name, "API_KEY");
                assert!(value.is_none());
            }
            other => panic!("expected EnvironmentVariable, got {other:?}"),
        }
    }

    #[test]
    fn credential_kind_mcp_oauth_preserves_extra() {
        let from_read: CredentialKind =
            serde_json::from_str(r#"{"type":"mcp_oauth","client_id":"abc","scopes":["read"]}"#)
                .unwrap();
        match from_read {
            CredentialKind::McpOauth { extra } => {
                assert_eq!(extra["client_id"], "abc");
            }
            other => panic!("expected McpOauth, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod request_body_tests {
    use super::*;

    #[test]
    fn agent_create_request_serializes_expected_shape() {
        let req = AgentCreateRequest::new("triage", "claude-opus-4-8")
            .system("be helpful")
            .description("triage agent")
            .add_tool(AgentTool::McpToolset {
                name: "forge".to_string(),
                allowed_tools: vec!["read".to_string()],
            })
            .add_mcp_server(McpServer::url("forge", "https://mcp.example/forge"))
            .add_skill(AgentSkillRef::new("skill_123", "1"))
            .metadata("team", "secops");

        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "triage");
        // Bare-string model serializes as a bare string.
        assert_eq!(value["model"], "claude-opus-4-8");
        assert_eq!(value["system"], "be helpful");
        assert_eq!(value["description"], "triage agent");
        assert_eq!(value["tools"][0]["type"], "mcp_toolset");
        assert_eq!(value["tools"][0]["name"], "forge");
        assert_eq!(value["mcp_servers"][0]["type"], "url");
        assert_eq!(value["mcp_servers"][0]["url"], "https://mcp.example/forge");
        assert_eq!(value["skills"][0]["skill_id"], "skill_123");
        assert_eq!(value["metadata"]["team"], "secops");

        // Round-trips back into an equal request.
        let round: AgentCreateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }

    #[test]
    fn agent_create_request_omits_empty_optionals() {
        let req = AgentCreateRequest::new("bare", "claude-sonnet-4-6");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "bare");
        // No system/description/multiagent and empty collections are skipped.
        assert!(value.get("system").is_none());
        assert!(value.get("description").is_none());
        assert!(value.get("multiagent").is_none());
        assert!(value.get("tools").is_none());
        assert!(value.get("mcp_servers").is_none());
        assert!(value.get("skills").is_none());
        assert!(value.get("metadata").is_none());
    }

    #[test]
    fn agent_create_request_with_custom_tool() {
        let req = AgentCreateRequest::new("coder", "claude-opus-4-8").add_tool(AgentTool::Custom {
            name: "lint".to_string(),
            description: "lint code".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
        });
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["tools"][0]["type"], "custom");
        assert_eq!(value["tools"][0]["name"], "lint");
        assert_eq!(value["tools"][0]["input_schema"]["type"], "object");
    }

    #[test]
    fn session_create_request_serializes_expected_shape() {
        let req = SessionCreateRequest::new(SessionAgentRef::versioned("agent_abc", "3"))
            .environment("env_1")
            .title("incident triage")
            .add_resource(SessionResourceSpec::File {
                file_id: "file_1".to_string(),
                mount_path: Some("/in".to_string()),
            })
            .add_vault("vault_1")
            .metadata("source", "slack");

        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["agent"]["type"], "agent");
        assert_eq!(value["agent"]["id"], "agent_abc");
        assert_eq!(value["agent"]["version"], "3");
        assert_eq!(value["environment_id"], "env_1");
        assert_eq!(value["title"], "incident triage");
        assert_eq!(value["resources"][0]["type"], "file");
        assert_eq!(value["resources"][0]["file_id"], "file_1");
        assert_eq!(value["vault_ids"][0], "vault_1");
        assert_eq!(value["metadata"]["source"], "slack");

        let round: SessionCreateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }

    #[test]
    fn session_create_request_bare_agent_id() {
        let req = SessionCreateRequest::new("agent_abc");
        let value = serde_json::to_value(&req).unwrap();
        // Bare agent reference serializes as a bare string.
        assert_eq!(value["agent"], "agent_abc");
        assert!(value.get("environment_id").is_none());
        assert!(value.get("title").is_none());
        assert!(value.get("resources").is_none());
        assert!(value.get("vault_ids").is_none());
    }
}

#[cfg(test)]
mod forward_compat_tests {
    use super::*;

    #[test]
    fn agent_preserves_unknown_top_level_fields() {
        // An Agent payload carrying a field this SDK does not model must still
        // deserialize, capturing the unknown field in `extra`.
        let json = r#"{
            "type": "agent",
            "id": "agent_1",
            "version": "1",
            "name": "triage",
            "model": "claude-opus-4-8",
            "future_field": {"nested": 42}
        }"#;
        let agent: Agent = serde_json::from_str(json).unwrap();
        assert_eq!(agent.id, "agent_1");
        assert_eq!(agent.model.id(), "claude-opus-4-8");
        assert_eq!(agent.extra["future_field"]["nested"], 42);
    }

    #[test]
    fn agent_skill_ref_latest_has_no_version() {
        let r = AgentSkillRef::latest("skill_9");
        let value = serde_json::to_value(&r).unwrap();
        assert_eq!(value["skill_id"], "skill_9");
        assert!(value.get("version").is_none());
        // sanity: HashMap import is exercised by the explicit-extra construction.
        let with_extra = AgentSkillRef {
            skill_id: "skill_9".to_string(),
            version: None,
            extra: HashMap::new(),
        };
        assert_eq!(with_extra, r);
    }
}
