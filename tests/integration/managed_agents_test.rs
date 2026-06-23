//! Integration tests for the Managed Agents API (beta: managed-agents-2026-04-01)
//!
//! These tests use a `wiremock` mock server. Each test asserts the client method
//! hits the right HTTP `method` + `path`, that the `anthropic-beta` request header
//! CONTAINS `managed-agents-2026-04-01` (the header is comma-joined, so a regex /
//! contains matcher is used rather than exact equality), and that the typed
//! response round-trips back into the SDK model.

use serde_json::json;
use threatflux::{
    models::managed_agents::{
        AgentCreateRequest, AgentUpdateRequest, CredentialCreateRequest, DeploymentCreateRequest,
        EnvironmentConfig, EnvironmentCreateRequest, MemoryStoreCreateRequest, NetworkingConfig,
        SessionCreateRequest, VaultCreateRequest,
    },
    types::Pagination,
    Client, Config,
};
use wiremock::{
    matchers::{header_regex, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

use crate::common::fixtures;

#[cfg(test)]
mod managed_agents_api_tests {
    use super::*;

    async fn setup_test_client(mock_server: &MockServer) -> Client {
        let config = Config::new("sk-ant-test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    /// Matcher asserting the comma-joined `anthropic-beta` header contains the
    /// managed-agents beta value.
    fn managed_agents_beta() -> impl wiremock::Match {
        header_regex("anthropic-beta", "managed-agents-2026-04-01")
    }

    // -------------------------------------------------------------------------
    // Agents
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_agent_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/agents"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_agent()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = AgentCreateRequest::new("triage", "claude-opus-4-8").system("be helpful");
        let agent = client.agents().create(request, None).await;

        assert!(agent.is_ok(), "agent create should succeed: {agent:?}");
        let agent = agent.unwrap();
        assert_eq!(agent.id, "agent_test123");
        assert_eq!(agent.model.id(), "claude-opus-4-8");
    }

    #[tokio::test]
    async fn test_agent_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/agents"))
            .and(query_param("limit", "10"))
            .and(query_param("after", "agent_cursor"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_agent()],
                "has_more": false,
                "first_id": "agent_test123",
                "last_id": "agent_test123"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let pagination = Pagination::new().with_limit(10).with_after("agent_cursor");
        let list = client.agents().list(Some(pagination), None).await;

        assert!(list.is_ok(), "agent list should succeed: {list:?}");
        let list = list.unwrap();
        assert_eq!(list.data.len(), 1);
        assert!(!list.has_more);
    }

    #[tokio::test]
    async fn test_agent_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/agents/agent_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_agent()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let agent = client.agents().get("agent_test123", None).await;
        assert!(agent.is_ok(), "agent get should succeed: {agent:?}");
    }

    #[tokio::test]
    async fn test_agent_get_version() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/agents/agent_test123/versions/2"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_agent()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let agent = client
            .agents()
            .get_version("agent_test123", "2", None)
            .await;
        assert!(agent.is_ok(), "agent get_version should succeed: {agent:?}");
    }

    #[tokio::test]
    async fn test_agent_update() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/agents/agent_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_agent()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = AgentUpdateRequest::new().name("triage-v2");
        let agent = client.agents().update("agent_test123", request, None).await;
        assert!(agent.is_ok(), "agent update should succeed: {agent:?}");
    }

    #[tokio::test]
    async fn test_agent_archive() {
        let mock_server = MockServer::start().await;

        // Archive is a POST to .../archive (distinct from DELETE).
        Mock::given(method("POST"))
            .and(path("/v1/agents/agent_test123/archive"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_agent()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let agent = client.agents().archive("agent_test123", None).await;
        assert!(agent.is_ok(), "agent archive should succeed: {agent:?}");
    }

    // -------------------------------------------------------------------------
    // Environments
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_environment_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/environments"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_environment()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = EnvironmentCreateRequest::new(
            "sandbox",
            EnvironmentConfig::Cloud {
                networking: NetworkingConfig::Unrestricted {},
            },
        );
        let env = client.environments().create(request, None).await;
        assert!(env.is_ok(), "environment create should succeed: {env:?}");
        assert_eq!(env.unwrap().id, "env_test123");
    }

    #[tokio::test]
    async fn test_environment_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/environments"))
            .and(query_param("limit", "5"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_environment()],
                "has_more": false,
                "first_id": "env_test123",
                "last_id": "env_test123"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let list = client
            .environments()
            .list(Some(Pagination::new().with_limit(5)), None)
            .await;
        assert!(list.is_ok(), "environment list should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 1);
    }

    #[tokio::test]
    async fn test_environment_delete() {
        let mock_server = MockServer::start().await;

        // Delete is a DELETE verb (distinct from POST .../archive).
        Mock::given(method("DELETE"))
            .and(path("/v1/environments/env_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let result = client.environments().delete("env_test123", None).await;
        assert!(
            result.is_ok(),
            "environment delete should succeed: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_environment_archive() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/environments/env_test123/archive"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_environment()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let env = client.environments().archive("env_test123", None).await;
        assert!(env.is_ok(), "environment archive should succeed: {env:?}");
    }

    // -------------------------------------------------------------------------
    // Sessions
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_session_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/sessions"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_session()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = SessionCreateRequest::new("agent_test123").title("triage run");
        let session = client.sessions().create(request, None).await;
        assert!(
            session.is_ok(),
            "session create should succeed: {session:?}"
        );
        let session = session.unwrap();
        assert_eq!(session.id, "session_test123");
        assert_eq!(session.agent.id(), "agent_test123");
    }

    #[tokio::test]
    async fn test_session_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_session()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let session = client.sessions().get("session_test123", None).await;
        assert!(session.is_ok(), "session get should succeed: {session:?}");
    }

    #[tokio::test]
    async fn test_session_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v1/sessions/session_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let result = client.sessions().delete("session_test123", None).await;
        assert!(result.is_ok(), "session delete should succeed: {result:?}");
    }

    #[tokio::test]
    async fn test_session_archive() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/sessions/session_test123/archive"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_session()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let session = client.sessions().archive("session_test123", None).await;
        assert!(
            session.is_ok(),
            "session archive should succeed: {session:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Session events
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_session_events_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123/events"))
            .and(query_param("limit", "20"))
            .and(query_param("after", "evt_cursor"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [{
                    "type": "agent.message",
                    "id": "evt_1",
                    "processed_at": "2026-04-01T00:00:00Z",
                    "content": [{"type": "text", "text": "hi"}]
                }],
                "has_more": false,
                "first_id": "evt_1",
                "last_id": "evt_1"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let pagination = Pagination::new().with_limit(20).with_after("evt_cursor");
        let list = client
            .sessions()
            .events("session_test123")
            .list(Some(pagination), None)
            .await;
        assert!(list.is_ok(), "session events list should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 1);
    }

    #[tokio::test]
    async fn test_session_events_send() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/sessions/session_test123/events"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "agent.message",
                "id": "evt_2",
                "processed_at": "2026-04-01T00:00:00Z",
                "content": [{"type": "text", "text": "ack"}]
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let event = client
            .sessions()
            .events("session_test123")
            .send_user_message("hello", None)
            .await;
        assert!(
            event.is_ok(),
            "session event send should succeed: {event:?}"
        );
    }

    #[tokio::test]
    async fn test_session_events_stream() {
        use futures::StreamExt;

        let mock_server = MockServer::start().await;

        let stream_body = [
            "event: agent.message\n",
            "data: {\"type\":\"agent.message\",\"id\":\"evt_1\",\"processed_at\":\"2026-04-01T00:00:00Z\",\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}\n",
            "\n",
            "event: session.status_idle\n",
            "data: {\"type\":\"session.status_idle\",\"id\":\"evt_2\",\"processed_at\":\"2026-04-01T00:00:00Z\",\"stop_reason\":\"end_turn\"}\n",
            "\n",
        ]
        .join("");

        Mock::given(method("GET"))
            .and(path("/v1/sessions/session_test123/events/stream"))
            .and(managed_agents_beta())
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(stream_body),
            )
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let stream = client
            .sessions()
            .events("session_test123")
            .stream(None)
            .await;
        assert!(
            stream.is_ok(),
            "session event stream should open: {:?}",
            stream.as_ref().err()
        );

        let events: Vec<_> = stream.unwrap().collect().await;
        assert_eq!(events.len(), 2, "should decode two events");
        assert!(events.iter().all(|e| e.is_ok()), "all events should decode");
    }

    // -------------------------------------------------------------------------
    // Vaults & credentials
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_vault_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/vaults"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_vault()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let vault = client
            .vaults()
            .create(VaultCreateRequest::new("secrets"), None)
            .await;
        assert!(vault.is_ok(), "vault create should succeed: {vault:?}");
        assert_eq!(vault.unwrap().id, "vault_test123");
    }

    #[tokio::test]
    async fn test_vault_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/vaults"))
            .and(query_param("limit", "3"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_vault()],
                "has_more": false,
                "first_id": "vault_test123",
                "last_id": "vault_test123"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let list = client
            .vaults()
            .list(Some(Pagination::new().with_limit(3)), None)
            .await;
        assert!(list.is_ok(), "vault list should succeed: {list:?}");
    }

    #[tokio::test]
    async fn test_vault_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v1/vaults/vault_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let result = client.vaults().delete("vault_test123", None).await;
        assert!(result.is_ok(), "vault delete should succeed: {result:?}");
    }

    #[tokio::test]
    async fn test_credential_create_write_only() {
        let mock_server = MockServer::start().await;

        // The create body carries the secret; the read response returns metadata only.
        Mock::given(method("POST"))
            .and(path("/v1/vaults/vault_test123/credentials"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "credential",
                "id": "cred_test123",
                "name": "github",
                "kind": {"type": "static_bearer"}
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = CredentialCreateRequest::new("github", fixtures::test_credential_kind());
        let credential = client
            .vaults()
            .credentials("vault_test123")
            .create(request, None)
            .await;
        assert!(
            credential.is_ok(),
            "credential create should succeed: {credential:?}"
        );
        assert_eq!(credential.unwrap().id, "cred_test123");
    }

    // -------------------------------------------------------------------------
    // Memory stores
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_memory_store_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/memory_stores"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory_store()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let store = client
            .memory_stores()
            .create(MemoryStoreCreateRequest::new("notes"), None)
            .await;
        assert!(
            store.is_ok(),
            "memory store create should succeed: {store:?}"
        );
        assert_eq!(store.unwrap().id, "mem_test123");
    }

    #[tokio::test]
    async fn test_memory_store_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/memory_stores"))
            .and(query_param("limit", "2"))
            .and(query_param("after", "mem_cursor"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_memory_store()],
                "has_more": true,
                "first_id": "mem_test123",
                "last_id": "mem_test123"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let pagination = Pagination::new().with_limit(2).with_after("mem_cursor");
        let list = client.memory_stores().list(Some(pagination), None).await;
        assert!(list.is_ok(), "memory store list should succeed: {list:?}");
        assert!(list.unwrap().has_more);
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v1/memory_stores/mem_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let result = client.memory_stores().delete("mem_test123", None).await;
        assert!(
            result.is_ok(),
            "memory store delete should succeed: {result:?}"
        );
    }

    // -------------------------------------------------------------------------
    // Deployments
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_deployment_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/deployments"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_deployment()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = DeploymentCreateRequest::new("nightly", "agent_test123").environment("env_1");
        let deployment = client.deployments().create(request, None).await;
        assert!(
            deployment.is_ok(),
            "deployment create should succeed: {deployment:?}"
        );
        assert_eq!(deployment.unwrap().id, "deploy_test123");
    }

    #[tokio::test]
    async fn test_deployment_list_pagination() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/deployments"))
            .and(query_param("limit", "4"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_deployment()],
                "has_more": false,
                "first_id": "deploy_test123",
                "last_id": "deploy_test123"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let list = client
            .deployments()
            .list(Some(Pagination::new().with_limit(4)), None)
            .await;
        assert!(list.is_ok(), "deployment list should succeed: {list:?}");
    }

    #[tokio::test]
    async fn test_deployment_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v1/deployments/deploy_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let result = client.deployments().delete("deploy_test123", None).await;
        assert!(
            result.is_ok(),
            "deployment delete should succeed: {result:?}"
        );
    }

    #[tokio::test]
    async fn test_deployment_run_trigger() {
        let mock_server = MockServer::start().await;

        // Manual run trigger: POST to .../runs.
        Mock::given(method("POST"))
            .and(path("/v1/deployments/deploy_test123/runs"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "type": "deployment_run",
                "id": "run_test123",
                "deployment_id": "deploy_test123",
                "session_id": "session_test123",
                "status": "running"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let run = client
            .deployments()
            .runs("deploy_test123")
            .trigger(None)
            .await;
        assert!(
            run.is_ok(),
            "deployment run trigger should succeed: {run:?}"
        );
        assert_eq!(run.unwrap().id, "run_test123");
    }
}
