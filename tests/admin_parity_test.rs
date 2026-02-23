use serde_json::json;
use threatflux::{types::Pagination, Client, Config};
use wiremock::{
    matchers::{header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

fn setup_admin_client(mock_server: &MockServer) -> Client {
    let config = Config::new("regular-api-key")
        .unwrap()
        .with_admin_key("admin-api-key")
        .with_base_url(mock_server.uri().parse().unwrap());
    Client::new(config)
}

#[tokio::test]
async fn test_admin_auth_uses_x_api_key_only() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/me"))
        .and(header("x-api-key", "admin-api-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "org_123",
            "name": "Example Org",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        })))
        .mount(&mock_server)
        .await;

    let client = setup_admin_client(&mock_server);
    let _ = client
        .admin()
        .unwrap()
        .organization()
        .get(None)
        .await
        .unwrap();

    let requests = mock_server.received_requests().await.unwrap();
    assert_eq!(
        requests[0]
            .headers
            .get("x-api-key")
            .unwrap()
            .to_str()
            .unwrap(),
        "admin-api-key"
    );
    assert!(!requests[0].headers.contains_key("authorization"));
}

#[tokio::test]
async fn test_users_list_uses_after_id_before_id_query_names() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/users"))
        .and(query_param("after_id", "after_user"))
        .and(query_param("before_id", "before_user"))
        .and(query_param("limit", "7"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "has_more": false
        })))
        .mount(&mock_server)
        .await;

    let client = setup_admin_client(&mock_server);
    let pagination = Pagination::new()
        .with_limit(7)
        .with_after("after_user")
        .with_before("before_user");

    let _ = client
        .admin()
        .unwrap()
        .organization()
        .list_users(Some(pagination), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_workspace_members_list_uses_after_id_before_id_query_names() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/workspaces/ws_123/members"))
        .and(query_param("after_id", "after_member"))
        .and(query_param("before_id", "before_member"))
        .and(query_param("limit", "4"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "has_more": false
        })))
        .mount(&mock_server)
        .await;

    let client = setup_admin_client(&mock_server);
    let pagination = Pagination::new()
        .with_limit(4)
        .with_after("after_member")
        .with_before("before_member");

    let _ = client
        .admin()
        .unwrap()
        .workspaces()
        .list_members("ws_123", Some(pagination), None)
        .await
        .unwrap();
}

#[tokio::test]
async fn test_invites_list_uses_after_id_before_id_query_names() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/organizations/invites"))
        .and(query_param("after_id", "after_invite"))
        .and(query_param("before_id", "before_invite"))
        .and(query_param("limit", "6"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [],
            "has_more": false
        })))
        .mount(&mock_server)
        .await;

    let client = setup_admin_client(&mock_server);
    let pagination = Pagination::new()
        .with_limit(6)
        .with_after("after_invite")
        .with_before("before_invite");

    let _ = client
        .admin()
        .unwrap()
        .organization()
        .list_invites(Some(pagination), None)
        .await
        .unwrap();
}
