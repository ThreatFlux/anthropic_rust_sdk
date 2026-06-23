//! Unit tests for the Managed Agents request builders (beta:
//! managed-agents-2026-04-01).
//!
//! Split from `managed_agents_test.rs` to keep each file within the per-file
//! size limit. These exercise the `*CreateRequest` / `*UpdateRequest` builders
//! and `SendEvent` constructors across the environment, memory, vault,
//! deployment, and session models: build via the fluent API, serialize, and
//! assert the wire shape (and round-trip where it matters).

use std::collections::HashMap;
use threatflux::models::managed_agents::{
    CredentialCreateRequest, CredentialKind, CredentialUpdateRequest, DeploymentCreateRequest,
    DeploymentSchedule, DeploymentUpdateRequest, EnvironmentConfig, EnvironmentCreateRequest,
    EnvironmentUpdateRequest, MemoryCreateRequest, MemoryRedactRequest, MemoryStoreCreateRequest,
    MemoryStoreUpdateRequest, MemoryUpdateRequest, NetworkingConfig, SendEvent, SessionAgentRef,
    SessionResourceSpec, SessionResourceUpdateRequest, SessionUpdateRequest, VaultCreateRequest,
    VaultUpdateRequest,
};

#[cfg(test)]
mod environment_request_tests {
    use super::*;

    #[test]
    fn environment_create_request_builder_with_metadata() {
        let req = EnvironmentCreateRequest::new(
            "sandbox",
            EnvironmentConfig::Cloud {
                networking: NetworkingConfig::Unrestricted {},
            },
        )
        .metadata("team", "secops")
        .metadata("tier", "prod");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "sandbox");
        assert_eq!(value["config"]["type"], "cloud");
        assert_eq!(value["metadata"]["team"], "secops");
        assert_eq!(value["metadata"]["tier"], "prod");
    }

    #[test]
    fn environment_update_request_builder_round_trip() {
        let req =
            EnvironmentUpdateRequest::new()
                .name("renamed")
                .config(EnvironmentConfig::Cloud {
                    networking: NetworkingConfig::Limited {
                        allow_package_managers: true,
                        allow_mcp_servers: false,
                        allowed_hosts: vec!["github.com".to_string()],
                    },
                });
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "renamed");
        assert_eq!(value["config"]["type"], "cloud");
        assert_eq!(value["config"]["networking"]["type"], "limited");

        let round: EnvironmentUpdateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }

    #[test]
    fn environment_update_request_default_is_empty() {
        let value = serde_json::to_value(EnvironmentUpdateRequest::new()).unwrap();
        assert!(value.get("name").is_none());
        assert!(value.get("config").is_none());
        assert!(value.get("metadata").is_none());
    }
}

#[cfg(test)]
mod memory_request_tests {
    use super::*;

    #[test]
    fn memory_store_create_request_builder() {
        let req = MemoryStoreCreateRequest::new("notes").description("scratch space");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "notes");
        assert_eq!(value["description"], "scratch space");
        // empty metadata is skipped
        assert!(value.get("metadata").is_none());
    }

    #[test]
    fn memory_store_update_request_round_trip() {
        let mut metadata = HashMap::new();
        metadata.insert("k".to_string(), "v".to_string());
        let req = MemoryStoreUpdateRequest {
            name: Some("renamed".to_string()),
            description: Some("new desc".to_string()),
            metadata: Some(metadata),
        };
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "renamed");
        assert_eq!(value["metadata"]["k"], "v");
        let round: MemoryStoreUpdateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }

    #[test]
    fn memory_create_request_shape() {
        let req = MemoryCreateRequest::new("remember this");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["content"], "remember this");
        assert!(value.get("metadata").is_none());
    }

    #[test]
    fn memory_update_request_default_is_empty() {
        let value = serde_json::to_value(MemoryUpdateRequest::new()).unwrap();
        assert!(value.get("content").is_none());
        assert!(value.get("metadata").is_none());
    }

    #[test]
    fn memory_redact_request_with_reason() {
        let req = MemoryRedactRequest::new().reason("pii");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["reason"], "pii");
        // empty redact request emits no reason
        let empty = serde_json::to_value(MemoryRedactRequest::new()).unwrap();
        assert!(empty.get("reason").is_none());
    }
}

#[cfg(test)]
mod vault_request_tests {
    use super::*;

    #[test]
    fn vault_create_request_with_metadata() {
        let req = VaultCreateRequest::new("secrets").metadata("owner", "secops");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "secrets");
        assert_eq!(value["metadata"]["owner"], "secops");
    }

    #[test]
    fn vault_update_request_builder() {
        let req = VaultUpdateRequest::new().name("renamed");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "renamed");
        let round: VaultUpdateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }

    #[test]
    fn credential_create_request_carries_secret() {
        let req = CredentialCreateRequest::new(
            "github",
            CredentialKind::StaticBearer {
                token: Some("sk-secret".to_string()),
            },
        );
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "github");
        assert_eq!(value["kind"]["type"], "static_bearer");
        assert_eq!(value["kind"]["token"], "sk-secret");
    }

    #[test]
    fn credential_update_request_optional_kind() {
        let req = CredentialUpdateRequest {
            name: Some("renamed".to_string()),
            kind: Some(CredentialKind::EnvironmentVariable {
                name: "API_KEY".to_string(),
                value: Some("v".to_string()),
            }),
        };
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "renamed");
        assert_eq!(value["kind"]["type"], "environment_variable");

        // default update emits nothing
        let empty = serde_json::to_value(CredentialUpdateRequest::new()).unwrap();
        assert!(empty.get("name").is_none());
        assert!(empty.get("kind").is_none());
    }
}

#[cfg(test)]
mod deployment_request_tests {
    use super::*;

    #[test]
    fn deployment_schedule_timezone_builder() {
        let schedule = DeploymentSchedule::cron("0 0 * * *").timezone("America/New_York");
        let value = serde_json::to_value(&schedule).unwrap();
        assert_eq!(value["cron"], "0 0 * * *");
        assert_eq!(value["timezone"], "America/New_York");
    }

    #[test]
    fn deployment_create_request_add_resource() {
        let req = DeploymentCreateRequest::new("nightly", "agent_1")
            .schedule(DeploymentSchedule::cron("0 0 * * *"))
            .add_resource(SessionResourceSpec::MemoryStore {
                memory_store_id: "mem_1".to_string(),
                access: Some("read".to_string()),
                instructions: None,
            });
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "nightly");
        assert_eq!(value["agent"], "agent_1");
        assert_eq!(value["resources"][0]["type"], "memory_store");
        assert_eq!(value["resources"][0]["memory_store_id"], "mem_1");
    }

    #[test]
    fn deployment_update_request_builder_round_trip() {
        let req = DeploymentUpdateRequest::new()
            .name("renamed")
            .schedule(DeploymentSchedule::cron("0 12 * * *"));
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "renamed");
        assert_eq!(value["schedule"]["cron"], "0 12 * * *");
        let round: DeploymentUpdateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }
}

#[cfg(test)]
mod session_request_tests {
    use super::*;

    #[test]
    fn session_update_request_builder() {
        let req = SessionUpdateRequest::new().title("renamed");
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["title"], "renamed");
        // empty update emits no fields
        let empty = serde_json::to_value(SessionUpdateRequest::new()).unwrap();
        assert!(empty.get("title").is_none());
        assert!(empty.get("metadata").is_none());
    }

    #[test]
    fn session_resource_update_request_with_extra() {
        let mut req = SessionResourceUpdateRequest::new();
        req.mount_path = Some("/data".to_string());
        req.access = Some("read_write".to_string());
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["mount_path"], "/data");
        assert_eq!(value["access"], "read_write");
        let round: SessionResourceUpdateRequest = serde_json::from_value(value).unwrap();
        assert_eq!(round, req);
    }

    #[test]
    fn session_agent_ref_from_owned_string() {
        // exercises the `From<String>` impl (distinct from `From<&str>`)
        let owned: SessionAgentRef = String::from("agent_owned").into();
        assert_eq!(owned.id(), "agent_owned");
    }
}

#[cfg(test)]
mod send_event_tests {
    use super::*;

    #[test]
    fn send_event_interrupt_shape() {
        let value = serde_json::to_value(SendEvent::interrupt()).unwrap();
        assert_eq!(value["type"], "user.interrupt");
    }

    #[test]
    fn send_event_confirm_tool_shape() {
        let value = serde_json::to_value(SendEvent::confirm_tool("tu_1", true)).unwrap();
        assert_eq!(value["type"], "user.tool_confirmation");
        assert_eq!(value["tool_use_id"], "tu_1");
        assert_eq!(value["approve"], true);
    }

    #[test]
    fn send_event_system_shape() {
        let value = serde_json::to_value(SendEvent::system("be terse")).unwrap();
        assert_eq!(value["type"], "system.message");
        assert_eq!(value["content"], "be terse");
    }

    #[test]
    fn send_event_custom_tool_result_round_trip() {
        let event = SendEvent::UserCustomToolResult {
            tool_use_id: "tu_1".to_string(),
            content: serde_json::json!({"ok": true}),
        };
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["type"], "user.custom_tool_result");
        assert_eq!(value["content"]["ok"], true);
        let round: SendEvent = serde_json::from_value(value).unwrap();
        assert_eq!(round, event);
    }

    #[test]
    fn send_event_define_outcome_round_trip() {
        let event = SendEvent::UserDefineOutcome {
            outcome: serde_json::json!({"status": "done"}),
        };
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["type"], "user.define_outcome");
        assert_eq!(value["outcome"]["status"], "done");
        let round: SendEvent = serde_json::from_value(value).unwrap();
        assert_eq!(round, event);
    }
}
