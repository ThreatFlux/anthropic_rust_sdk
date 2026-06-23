//! Integration tests for the Managed Agents API — vaults & credentials, memory
//! stores, and deployments (beta: managed-agents-2026-04-01).
//!
//! Split from `managed_agents_test.rs` to keep each test file within the
//! per-file size limit. Same conventions: a `wiremock` mock server, assertions
//! on HTTP method/path and that the `anthropic-beta` header contains the beta.

use serde_json::json;
use threatflux::{
    models::managed_agents::{
        CredentialCreateRequest, DeploymentCreateRequest, MemoryStoreCreateRequest,
        VaultCreateRequest,
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
mod managed_agents_more_tests {
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
