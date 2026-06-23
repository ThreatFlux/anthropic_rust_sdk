//! Integration tests for the Managed Agents sub-resource APIs, part 2
//! (beta: managed-agents-2026-04-01).
//!
//! Continuation of `managed_agents_sub_test.rs`, split to keep each file within
//! the per-file size limit. Covers the memories sub-API, memory-store get/update,
//! the credentials sub-API + vault get/update, and deployment get/update + runs.
//! Same conventions: a `wiremock` mock server, assertions on HTTP method/path,
//! and that the `anthropic-beta` header contains the managed-agents beta value.

use serde_json::json;
use threatflux::{
    models::managed_agents::{
        CredentialUpdateRequest, DeploymentUpdateRequest, MemoryCreateRequest, MemoryRedactRequest,
        MemoryStoreUpdateRequest, MemoryUpdateRequest, VaultUpdateRequest,
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
mod managed_agents_sub2_tests {
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
    // Memories sub-API
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_memory_create() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/memory_stores/mem_test123/memories"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let memory = client
            .memory_stores()
            .memories("mem_test123")
            .create(MemoryCreateRequest::new("remember this"), None)
            .await;
        assert!(memory.is_ok(), "memory create should succeed: {memory:?}");
        assert_eq!(memory.unwrap().id, "memory_test123");
    }

    #[tokio::test]
    async fn test_memory_list_and_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/memory_stores/mem_test123/memories"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_memory()],
                "has_more": false,
                "first_id": "memory_test123",
                "last_id": "memory_test123"
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/v1/memory_stores/mem_test123/memories/memory_test123",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let memories = client.memory_stores().memories("mem_test123");

        assert!(
            memories.list(None, None).await.is_ok(),
            "list should succeed"
        );
        assert!(
            memories.get("memory_test123", None).await.is_ok(),
            "get should succeed"
        );
    }

    #[tokio::test]
    async fn test_memory_update_and_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1/memory_stores/mem_test123/memories/memory_test123",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory()))
            .mount(&mock_server)
            .await;
        Mock::given(method("DELETE"))
            .and(path(
                "/v1/memory_stores/mem_test123/memories/memory_test123",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let memories = client.memory_stores().memories("mem_test123");

        let update = MemoryUpdateRequest::new();
        assert!(
            memories
                .update("memory_test123", update, None)
                .await
                .is_ok(),
            "update should succeed"
        );
        assert!(
            memories.delete("memory_test123", None).await.is_ok(),
            "delete should succeed"
        );
    }

    #[tokio::test]
    async fn test_memory_redact() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/v1/memory_stores/mem_test123/memories/memory_test123/redact",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let request = MemoryRedactRequest::new().reason("pii");
        let memory = client
            .memory_stores()
            .memories("mem_test123")
            .redact("memory_test123", request, None)
            .await;
        assert!(memory.is_ok(), "memory redact should succeed: {memory:?}");
    }

    #[tokio::test]
    async fn test_memory_versions() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path(
                "/v1/memory_stores/mem_test123/memories/memory_test123/versions",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_memory_version()],
                "has_more": false,
                "first_id": "memver_test123",
                "last_id": "memver_test123"
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/v1/memory_stores/mem_test123/memories/memory_test123/versions/memver_test123",
            ))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory_version()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let memories = client.memory_stores().memories("mem_test123");

        let list = memories.list_versions("memory_test123", None, None).await;
        assert!(list.is_ok(), "list_versions should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 1);

        let version = memories
            .get_version("memory_test123", "memver_test123", None)
            .await;
        assert!(version.is_ok(), "get_version should succeed: {version:?}");
        assert_eq!(version.unwrap().id, "memver_test123");
    }

    #[tokio::test]
    async fn test_memory_store_get_update() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/memory_stores/mem_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory_store()))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/memory_stores/mem_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_memory_store()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        assert!(
            client
                .memory_stores()
                .get("mem_test123", None)
                .await
                .is_ok(),
            "memory store get should succeed"
        );
        let update = MemoryStoreUpdateRequest::new();
        assert!(
            client
                .memory_stores()
                .update("mem_test123", update, None)
                .await
                .is_ok(),
            "memory store update should succeed"
        );
    }

    // -------------------------------------------------------------------------
    // Credentials sub-API + vault get/update
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_credential_list_and_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/vaults/vault_test123/credentials"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_credential()],
                "has_more": false,
                "first_id": "cred_test123",
                "last_id": "cred_test123"
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/vaults/vault_test123/credentials/cred_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_credential()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let credentials = client.vaults().credentials("vault_test123");

        assert!(
            credentials.list(None, None).await.is_ok(),
            "list should succeed"
        );
        assert!(
            credentials.get("cred_test123", None).await.is_ok(),
            "get should succeed"
        );
    }

    #[tokio::test]
    async fn test_credential_update_and_delete() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/vaults/vault_test123/credentials/cred_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_credential()))
            .mount(&mock_server)
            .await;
        Mock::given(method("DELETE"))
            .and(path("/v1/vaults/vault_test123/credentials/cred_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"deleted": true})))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let credentials = client.vaults().credentials("vault_test123");

        let update = CredentialUpdateRequest::new();
        assert!(
            credentials
                .update("cred_test123", update, None)
                .await
                .is_ok(),
            "update should succeed"
        );
        assert!(
            credentials.delete("cred_test123", None).await.is_ok(),
            "delete should succeed"
        );
    }

    #[tokio::test]
    async fn test_vault_get_update() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/vaults/vault_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_vault()))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/vaults/vault_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_vault()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        assert!(
            client.vaults().get("vault_test123", None).await.is_ok(),
            "vault get should succeed"
        );
        let update = VaultUpdateRequest::new().name("renamed");
        assert!(
            client
                .vaults()
                .update("vault_test123", update, None)
                .await
                .is_ok(),
            "vault update should succeed"
        );
    }

    // -------------------------------------------------------------------------
    // Deployment get/update + runs
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_deployment_get_update() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/deployments/deploy_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_deployment()))
            .mount(&mock_server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/deployments/deploy_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_deployment()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        assert!(
            client
                .deployments()
                .get("deploy_test123", None)
                .await
                .is_ok(),
            "deployment get should succeed"
        );
        let update = DeploymentUpdateRequest::new().name("renamed");
        assert!(
            client
                .deployments()
                .update("deploy_test123", update, None)
                .await
                .is_ok(),
            "deployment update should succeed"
        );
    }

    #[tokio::test]
    async fn test_deployment_runs_list_get() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/deployments/deploy_test123/runs"))
            .and(query_param("limit", "3"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [fixtures::test_deployment_run()],
                "has_more": false,
                "first_id": "run_test123",
                "last_id": "run_test123"
            })))
            .mount(&mock_server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/deployments/deploy_test123/runs/run_test123"))
            .and(managed_agents_beta())
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_deployment_run()))
            .mount(&mock_server)
            .await;

        let client = setup_test_client(&mock_server).await;
        let runs = client.deployments().runs("deploy_test123");

        let list = runs.list(Some(Pagination::new().with_limit(3)), None).await;
        assert!(list.is_ok(), "runs list should succeed: {list:?}");
        assert_eq!(list.unwrap().data.len(), 1);

        let run = runs.get("run_test123", None).await;
        assert!(run.is_ok(), "run get should succeed: {run:?}");
        assert_eq!(run.unwrap().id, "run_test123");
    }
}
