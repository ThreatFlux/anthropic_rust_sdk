//! Integration tests for Admin API
//!
//! Tests Admin API operations with mocked responses.

use chrono::TimeZone;
use serde_json::json;
use threatflux_anthropic_sdk::models::admin::{
    ApiKeyCreateRequest, InviteCreateRequest, InviteCreateRole, MessageUsageReportParams,
    UserUpdateRequest, UserUpdateRole, WorkspaceCreateRequest, WorkspaceUpdateRequest,
};
use threatflux_anthropic_sdk::{Client, Config};
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

use crate::common::fixtures;

#[cfg(test)]
mod admin_api_tests {
    use super::*;

    async fn setup_test_admin_client(mock_server: &MockServer) -> Client {
        let config = Config::new("admin-test-key")
            .unwrap()
            .with_admin_key("admin-test-key")
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    #[tokio::test]
    async fn test_get_organization() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/organizations/me"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_organization()))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.organization().get(None).await;

        assert!(response.is_ok());
        let org = response.unwrap();
        assert_eq!(org.name, "Test Organization");
        assert_eq!(org.display_name, Some("Test Org".to_string()));
    }

    #[tokio::test]
    async fn test_list_workspaces() {
        let mock_server = MockServer::start().await;

        let workspaces_response = json!({
            "data": [fixtures::test_workspace()],
            "has_more": false,
            "first_id": "ws_test123",
            "last_id": "ws_test123"
        });

        Mock::given(method("GET"))
            .and(path("/v1/organizations/workspaces"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&workspaces_response))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.workspaces().list(None, None).await;

        assert!(response.is_ok());
        let workspaces = response.unwrap();
        assert!(!workspaces.has_more);
        assert!(!workspaces.data.is_empty());
        assert_eq!(workspaces.data[0].name, "Test Workspace");
    }

    #[tokio::test]
    async fn test_get_workspace() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/organizations/workspaces/ws_test123"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(fixtures::test_workspace()))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.workspaces().get("ws_test123", None).await;

        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(workspace.id, "ws_test123");
        assert_eq!(
            workspace.status,
            Some(threatflux_anthropic_sdk::models::admin::WorkspaceStatus::Active)
        );
    }

    #[tokio::test]
    async fn test_create_workspace() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/organizations/workspaces"))
            .and(header("x-api-key", "admin-test-key"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
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

        let request = WorkspaceCreateRequest::new("new-workspace").display_name("New Workspace");
        let response = admin.workspaces().create(request, None).await;

        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(workspace.name, "new-workspace");
        assert_eq!(workspace.display_name, Some("New Workspace".to_string()));
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

        Mock::given(method("POST"))
            .and(path("/v1/organizations/workspaces/ws_test123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&updated_workspace))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let request = WorkspaceUpdateRequest::new().display_name("Updated Workspace");
        let response = admin.workspaces().update("ws_test123", request, None).await;

        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(
            workspace.display_name,
            Some("Updated Workspace".to_string())
        );
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
            .and(path("/v1/organizations/workspaces/ws_test123/archive"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&archived_workspace))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.workspaces().archive("ws_test123", None).await;

        assert!(response.is_ok());
        let workspace = response.unwrap();
        assert_eq!(
            workspace.status,
            Some(threatflux_anthropic_sdk::models::admin::WorkspaceStatus::Archived)
        );
        assert!(workspace.archived_at.is_some());
    }

    #[tokio::test]
    async fn test_list_api_keys() {
        let mock_server = MockServer::start().await;

        let api_keys_response = json!({
            "data": [
                {
                    "id": "key_123",
                    "type": "api_key",
                    "name": "Production Key",
                    "partial_key_hint": "sk-ant-...abc123",
                    "status": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "last_used_at": "2024-01-01T12:00:00Z"
                }
            ],
            "has_more": false,
            "first_id": "key_123",
            "last_id": "key_123"
        });

        Mock::given(method("GET"))
            .and(path("/v1/organizations/api_keys"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&api_keys_response))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.api_keys().list(None, None, None).await;

        assert!(response.is_ok());
        let keys = response.unwrap();
        assert!(!keys.data.is_empty());
        assert_eq!(keys.data[0].name, "Production Key");
        assert_eq!(
            keys.data[0].partial_key_hint,
            Some("sk-ant-...abc123".to_string())
        );
    }

    #[tokio::test]
    async fn test_get_api_key() {
        let mock_server = MockServer::start().await;

        let api_key_response = json!({
            "id": "key_123",
            "type": "api_key",
            "name": "Production Key",
            "partial_key_hint": "sk-ant-...abc123",
            "status": "active",
            "created_at": "2024-01-01T00:00:00Z",
            "last_used_at": "2024-01-01T12:00:00Z"
        });

        Mock::given(method("GET"))
            .and(path("/v1/organizations/api_keys/key_123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&api_key_response))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.api_keys().get("key_123", None, None).await;

        assert!(response.is_ok());
        let key = response.unwrap();
        assert_eq!(key.id, "key_123");
        assert_eq!(key.name, "Production Key");
    }

    #[tokio::test]
    async fn test_create_api_key_unsupported() {
        // The current Admin API does not support programmatic API-key creation,
        // so the SDK returns an invalid-input error without hitting the network.
        let mock_server = MockServer::start().await;
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let request = ApiKeyCreateRequest::new("New API Key");
        let response = admin.api_keys().create(request, None, None).await;

        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_delete_api_key_unsupported() {
        // Deleting Admin API keys via API is not supported by the current Admin API.
        let mock_server = MockServer::start().await;
        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.api_keys().delete("key_123", None, None).await;
        assert!(response.is_err());
    }

    #[tokio::test]
    async fn test_get_usage_report() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/organizations/usage_report/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "starting_at": "2024-01-01T00:00:00Z",
                        "ending_at": "2024-01-02T00:00:00Z",
                        "request_count": 10,
                        "input_tokens": 1000,
                        "output_tokens": 500
                    }
                ],
                "has_more": false,
                "next_page": null
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let start = chrono::Utc::now();
        let response = admin
            .usage()
            .get_message_usage_report(MessageUsageReportParams::new(start), None)
            .await;

        assert!(response.is_ok());
        let report = response.unwrap();
        assert_eq!(report.data.len(), 1);
        let bucket = &report.data[0];
        assert_eq!(bucket.input_tokens, Some(1000));
        assert_eq!(bucket.output_tokens, Some(500));
        let total_tokens = bucket.input_tokens.unwrap_or(0) + bucket.output_tokens.unwrap_or(0);
        assert_eq!(total_tokens, 1500);
    }

    #[tokio::test]
    async fn test_get_usage_report_with_date_range() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/organizations/usage_report/messages"))
            .and(query_param("bucket_width", "1d"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [
                    {
                        "starting_at": "2024-01-01T00:00:00Z",
                        "ending_at": "2024-01-31T00:00:00Z",
                        "request_count": 50,
                        "input_tokens": 5000,
                        "output_tokens": 2500
                    }
                ],
                "has_more": false,
                "next_page": null
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let start = chrono::Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let end = chrono::Utc.with_ymd_and_hms(2024, 1, 31, 0, 0, 0).unwrap();

        let params = MessageUsageReportParams::new(start)
            .ending_at(end)
            .bucket_width("1d");
        let response = admin.usage().get_message_usage_report(params, None).await;

        assert!(response.is_ok());
        let report = response.unwrap();
        assert_eq!(report.data.len(), 1);
        assert_eq!(report.data[0].input_tokens, Some(5000));
        assert_eq!(report.data[0].output_tokens, Some(2500));
    }

    #[tokio::test]
    async fn test_list_members() {
        let mock_server = MockServer::start().await;

        let users_response = json!({
            "data": [
                {
                    "id": "member_123",
                    "type": "user",
                    "email": "user@example.com",
                    "role": "admin",
                    "added_at": "2024-01-01T00:00:00Z"
                }
            ],
            "has_more": false
        });

        Mock::given(method("GET"))
            .and(path("/v1/organizations/users"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&users_response))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.users().list_users(None, None).await;

        assert!(response.is_ok());
        let users = response.unwrap();
        assert!(!users.data.is_empty());
        assert_eq!(users.data[0].email, "user@example.com");
        assert_eq!(
            users.data[0].role,
            threatflux_anthropic_sdk::models::admin::UserRole::Admin
        );
    }

    #[tokio::test]
    async fn test_invite_member() {
        let mock_server = MockServer::start().await;

        let invite_response = json!({
            "id": "invite_new123",
            "type": "invite",
            "email": "newuser@example.com",
            "role": "developer",
            "status": "pending",
            "expires_at": "2024-02-01T00:00:00Z",
            "invited_at": "2024-01-01T00:00:00Z"
        });

        Mock::given(method("POST"))
            .and(path("/v1/organizations/invites"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&invite_response))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let request = InviteCreateRequest::new("newuser@example.com", InviteCreateRole::Developer);
        let response = admin.users().create_invite(request, None).await;

        assert!(response.is_ok());
        let invite = response.unwrap();
        assert_eq!(invite.email, "newuser@example.com");
        assert_eq!(
            invite.status,
            threatflux_anthropic_sdk::models::admin::InviteStatus::Pending
        );
    }

    #[tokio::test]
    async fn test_update_member_role() {
        let mock_server = MockServer::start().await;

        let updated_user = json!({
            "id": "member_123",
            "type": "user",
            "email": "user@example.com",
            "role": "developer",
            "added_at": "2024-01-01T00:00:00Z"
        });

        Mock::given(method("POST"))
            .and(path("/v1/organizations/users/member_123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&updated_user))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let request = UserUpdateRequest::new(UserUpdateRole::Developer);
        let response = admin.users().update_user("member_123", request, None).await;

        assert!(response.is_ok());
        let user = response.unwrap();
        assert_eq!(
            user.role,
            threatflux_anthropic_sdk::models::admin::UserRole::Developer
        );
    }

    #[tokio::test]
    async fn test_remove_member() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/v1/organizations/users/member_123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "member_123",
                "type": "user_deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.users().delete_user("member_123", None).await;
        assert!(response.is_ok());
        assert_eq!(response.unwrap().id, "member_123");
    }

    #[tokio::test]
    async fn test_admin_authorization_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/v1/organizations/me"))
            .respond_with(ResponseTemplate::new(403).set_body_json(json!({
                "type": "error",
                "error": {
                    "type": "permission_error",
                    "message": "Insufficient permissions for admin operations"
                }
            })))
            .mount(&mock_server)
            .await;

        // Use a regular API key as the admin key to simulate insufficient permissions.
        let config = Config::new("regular-test-key")
            .unwrap()
            .with_admin_key("regular-test-key")
            .with_base_url(mock_server.uri().parse().unwrap());
        let client = Client::new(config);

        let admin_result = client.admin();
        assert!(admin_result.is_ok()); // Admin client creation should succeed

        let admin = admin_result.unwrap();
        let response = admin.organization().get(None).await;

        assert!(response.is_err());
        if let Err(threatflux_anthropic_sdk::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 403);
        } else {
            panic!("Expected 403 error");
        }
    }

    #[tokio::test]
    async fn test_workspace_status_filtering() {
        let mock_server = MockServer::start().await;

        let workspaces_with_different_statuses = json!({
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
            .and(path("/v1/organizations/workspaces"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(&workspaces_with_different_statuses),
            )
            .mount(&mock_server)
            .await;

        let client = setup_test_admin_client(&mock_server).await;
        let admin = client.admin().unwrap();

        let response = admin.workspaces().list(None, None).await;
        assert!(response.is_ok());

        let workspaces = response.unwrap();
        assert_eq!(workspaces.data.len(), 2);
        assert_eq!(
            workspaces.data[0].status,
            Some(threatflux_anthropic_sdk::models::admin::WorkspaceStatus::Active)
        );
        assert_eq!(
            workspaces.data[1].status,
            Some(threatflux_anthropic_sdk::models::admin::WorkspaceStatus::Archived)
        );
    }
}
