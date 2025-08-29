//! Integration tests for Admin API
//!
//! Tests Admin API operations with mocked responses.

use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, header, query_param}};
use threatflux::{Client, Config, types::Pagination};
use serde_json::json;
use pretty_assertions::assert_eq;

mod common;
use crate::common::{fixtures, mock_server};

#[cfg(test)]
mod admin_api_tests {
    use super::*;
    
    async fn setup_test_admin_client(mock_server: &MockServer) -> Client {
        let config = Config::new("admin-test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    #[tokio::test]
    async fn test_get_organization() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/organization"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_organization()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.organization().get(None).await;
        
        assert!(response.is_ok());
        let org = response.unwrap();
        assert_eq!(org.name, "Test Organization");
        assert_eq!(org.display_name, "Test Org");
    }

    #[tokio::test]
    async fn test_list_workspaces() {
        let mock_server = MockServer::start().await;
        
        let workspaces_response = json!({
            "object": "list",
            "data": [fixtures::test_workspace()],
            "has_more": false,
            "first_id": "ws_test123",
            "last_id": "ws_test123"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/workspaces"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&workspaces_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.workspaces().list(None, None).await;
        
        assert!(response.is_ok());
        let workspaces = response.unwrap();
        assert_eq!(workspaces.object, "list");
        assert!(!workspaces.data.is_empty());
        assert_eq!(workspaces.data[0].name, "Test Workspace");
    }

    #[tokio::test]
    async fn test_get_workspace() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/workspaces/ws_test123"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_workspace()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.workspaces().get("ws_test123", None).await;
        
        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(workspace.id, "ws_test123");
        assert_eq!(workspace.status, threatflux::models::admin::WorkspaceStatus::Active);
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let mock_server = MockServer::start().await;
        
        let create_request = json!({
            "name": "new-workspace",
            "display_name": "New Workspace"
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/organization/workspaces"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(201)
                .set_body_json(&json!({
                    "id": "ws_new123",
                    "type": "workspace",
                    "name": "new-workspace",
                    "display_name": "New Workspace",
                    "status": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.workspaces()
            .create("new-workspace", "New Workspace", None).await;
        
        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(workspace.name, "new-workspace");
        assert_eq!(workspace.display_name, "New Workspace");
    }

    #[tokio::test]
    async fn test_update_workspace() {
        let mock_server = MockServer::start().await;
        
        let updated_workspace = json!({
            "id": "ws_test123",
            "type": "workspace",
            "name": "updated-workspace",
            "display_name": "Updated Workspace",
            "status": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T12:00:00Z",
            "archived_at": null
        });
        
        Mock::given(method("PATCH"))
            .and(path("/v1/organization/workspaces/ws_test123"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&updated_workspace))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.workspaces()
            .update("ws_test123", Some("Updated Workspace"), None).await;
        
        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(workspace.display_name, "Updated Workspace");
    }

    #[tokio::test]
    async fn test_archive_workspace() {
        let mock_server = MockServer::start().await;
        
        let archived_workspace = json!({
            "id": "ws_test123",
            "type": "workspace",
            "name": "test-workspace",
            "display_name": "Test Workspace",
            "status": "archived",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T12:00:00Z",
            "archived_at": "2024-01-01T12:00:00Z"
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/organization/workspaces/ws_test123/archive"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&archived_workspace))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.workspaces().archive("ws_test123", None).await;
        
        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(workspace.status, threatflux::models::admin::WorkspaceStatus::Archived);
        assert!(workspace.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_list_api_keys() {
        let mock_server = MockServer::start().await;
        
        let api_keys_response = json!({
            "object": "list",
            "data": [
                {
                    "id": "key_123",
                    "type": "api_key",
                    "name": "Production Key",
                    "partial_key": "sk-ant-...abc123",
                    "created_at": "2024-01-01T00:00:00Z",
                    "last_used_at": "2024-01-01T12:00:00Z"
                }
            ],
            "has_more": false,
            "first_id": "key_123",
            "last_id": "key_123"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/api_keys"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&api_keys_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.api_keys().list(None, None).await;
        
        assert!(response.is_ok());
        let keys = response.unwrap();
        assert!(!keys.data.is_empty());
        assert_eq!(keys.data[0].name, "Production Key");
    }

    #[tokio::test]
    async fn test_create_api_key() {
        let mock_server = MockServer::start().await;
        
        let create_response = json!({
            "id": "key_new123",
            "type": "api_key",
            "name": "New API Key",
            "key": "sk-ant-api03-abc123def456ghi789jkl012mno345pqr678stu901vwx234yz-ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890abcdefg",
            "created_at": "2024-01-01T00:00:00Z"
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/organization/api_keys"))
            .respond_with(ResponseTemplate::new(201)
                .set_body_json(&create_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.api_keys().create("New API Key", None).await;
        
        assert!(response.is_ok());
        let key_response = response.unwrap();
        assert_eq!(key_response.name, "New API Key");
        assert!(key_response.key.starts_with("sk-ant-"));
    }

    #[tokio::test]
    async fn test_delete_api_key() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("DELETE"))
            .and(path("/v1/organization/api_keys/key_123"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&json!({"deleted": true})))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.api_keys().delete("key_123", None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_get_usage_report() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/usage"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_usage_report()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.usage().get_current_billing_usage(None, None).await;
        
        assert!(response.is_ok());
        let usage = response.unwrap();
        assert_eq!(usage.input_tokens, 1000);
        assert_eq!(usage.output_tokens, 500);
        assert_eq!(usage.total_tokens(), 1500);
    }

    #[tokio::test]
    async fn test_get_usage_report_with_date_range() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/usage"))
            .and(query_param("start_date", "2024-01-01"))
            .and(query_param("end_date", "2024-01-31"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&json!({
                    "input_tokens": 5000,
                    "output_tokens": 2500
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let start_date = chrono::NaiveDate::from_ymd_opt(2024, 1, 1).unwrap();
        let end_date = chrono::NaiveDate::from_ymd_opt(2024, 1, 31).unwrap();
        
        let response = admin.usage()
            .get_current_billing_usage(Some(start_date), Some(end_date))
            .await;
        
        assert!(response.is_ok());
        let usage = response.unwrap();
        assert_eq!(usage.input_tokens, 5000);
        assert_eq!(usage.output_tokens, 2500);
    }

    #[tokio::test]
    async fn test_list_members() {
        let mock_server = MockServer::start().await;
        
        let members_response = json!({
            "object": "list",
            "data": [
                {
                    "id": "member_123",
                    "type": "organization_member",
                    "email": "user@example.com",
                    "role": "admin",
                    "status": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                }
            ],
            "has_more": false
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/members"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&members_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.members().list(None, None).await;
        
        assert!(response.is_ok());
        let members = response.unwrap();
        assert!(!members.data.is_empty());
        assert_eq!(members.data[0].email, "user@example.com");
        assert_eq!(members.data[0].role, threatflux::models::admin::MemberRole::Admin);
    }

    #[tokio::test]
    async fn test_invite_member() {
        let mock_server = MockServer::start().await;
        
        let invite_response = json!({
            "id": "member_new123",
            "type": "organization_member",
            "email": "newuser@example.com",
            "role": "member",
            "status": "pending",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/organization/members"))
            .respond_with(ResponseTemplate::new(201)
                .set_body_json(&invite_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.members()
            .invite("newuser@example.com", threatflux::models::admin::MemberRole::Member, None)
            .await;
        
        assert!(response.is_ok());
        let member = response.unwrap();
        assert_eq!(member.email, "newuser@example.com");
        assert_eq!(member.status, threatflux::models::admin::MemberStatus::Pending);
    }

    #[tokio::test]
    async fn test_update_member_role() {
        let mock_server = MockServer::start().await;
        
        let updated_member = json!({
            "id": "member_123",
            "type": "organization_member",
            "email": "user@example.com",
            "role": "admin",
            "status": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T12:00:00Z"
        });
        
        Mock::given(method("PATCH"))
            .and(path("/v1/organization/members/member_123"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&updated_member))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.members()
            .update_role("member_123", threatflux::models::admin::MemberRole::Admin, None)
            .await;
        
        assert!(response.is_ok());
        let member = response.unwrap();
        assert_eq!(member.role, threatflux::models::admin::MemberRole::Admin);
    }

    #[tokio::test]
    async fn test_remove_member() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("DELETE"))
            .and(path("/v1/organization/members/member_123"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&json!({"deleted": true})))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.members().remove("member_123", None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_admin_authorization_error() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/organization"))
            .respond_with(ResponseTemplate::new(403)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "permission_error",
                        "message": "Insufficient permissions for admin operations"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        // Use a regular (non-admin) API key
        let config = Config::new("regular-test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        let client = Client::new(config);
        
        let admin_result = client.admin();
        assert!(admin_result.is_ok()); // Client creation should succeed
        
        let admin = admin_result.unwrap();
        let response = admin.organization().get(None).await;
        
        assert!(response.is_err());
        if let Err(threatflux::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 403);
        } else {
            panic!("Expected 403 error");
        }
    }

    #[tokio::test]
    async fn test_workspace_status_filtering() {
        let mock_server = MockServer::start().await;
        
        let workspaces_with_different_statuses = json!({
            "object": "list",
            "data": [
                {
                    "id": "ws_active",
                    "type": "workspace",
                    "name": "active-workspace",
                    "display_name": "Active Workspace",
                    "status": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                    "archived_at": null
                },
                {
                    "id": "ws_archived",
                    "type": "workspace",
                    "name": "archived-workspace", 
                    "display_name": "Archived Workspace",
                    "status": "archived",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                    "archived_at": "2024-01-01T12:00:00Z"
                }
            ],
            "has_more": false
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/organization/workspaces"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&workspaces_with_different_statuses))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();
        
        let response = admin.workspaces().list(None, None).await;
        assert!(response.is_ok());
        
        let workspaces = response.unwrap();
        assert_eq!(workspaces.data.len(), 2);
        assert_eq!(workspaces.data[0].status, threatflux::models::admin::WorkspaceStatus::Active);
        assert_eq!(workspaces.data[1].status, threatflux::models::admin::WorkspaceStatus::Archived);
    }
}