use serde_json::json;
use threatflux::{
    models::skill::{
        SkillCreateRequest, SkillFileUpload, SkillListParams, SkillVersionCreateRequest,
        SkillVersionListParams,
    },
    Client, Config,
};
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

fn setup_client(mock_server: &MockServer) -> Client {
    let config = Config::new("test-key")
        .unwrap()
        .with_base_url(mock_server.uri().parse().unwrap());
    Client::new(config)
}

fn sample_skill_payload() -> serde_json::Value {
    json!({
        "type": "skill",
        "id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
        "display_title": "Meeting notes skill",
        "created_at": "2026-02-20T16:00:00Z",
        "updated_at": "2026-02-20T16:00:00Z",
        "latest_version": "1723500000",
        "source": "custom"
    })
}

fn sample_skill_version_payload() -> serde_json::Value {
    json!({
        "type": "skill_version",
        "id": "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
        "skill_id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
        "version": "1723500000",
        "name": "meeting-notes",
        "description": "Summarizes notes from uploaded meeting transcripts.",
        "directory": "meeting-notes",
        "created_at": "2026-02-20T16:05:00Z"
    })
}

#[tokio::test]
async fn test_list_skills_uses_beta_header_and_query_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/skills"))
        .and(query_param("limit", "5"))
        .and(query_param("page", "page_1"))
        .and(query_param("source", "custom"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [sample_skill_payload()],
            "has_more": false
        })))
        .mount(&mock_server)
        .await;

    let client = setup_client(&mock_server);
    let params = SkillListParams::new()
        .with_limit(5)
        .with_page("page_1")
        .with_source("custom");

    let response = client.skills().list(Some(params), None).await.unwrap();

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].id, "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE");
    assert_eq!(response.data[0].source.as_deref(), Some("custom"));

    let requests = mock_server.received_requests().await.unwrap();
    let beta_header = requests[0]
        .headers
        .get("anthropic-beta")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(beta_header.contains("skills-2025-10-02"));
}

#[tokio::test]
async fn test_get_and_delete_skill_parse_reference_payloads() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/skills/skl_01DP8V5D1N6V3Q6N57V8Q9W0XE"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_payload()))
        .mount(&mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path("/v1/skills/skl_01DP8V5D1N6V3Q6N57V8Q9W0XE"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "skill_deleted",
            "id": "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE"
        })))
        .mount(&mock_server)
        .await;

    let client = setup_client(&mock_server);

    let skill = client
        .skills()
        .get("skl_01DP8V5D1N6V3Q6N57V8Q9W0XE", None)
        .await
        .unwrap();
    assert_eq!(skill.object_type.as_deref(), Some("skill"));

    let deleted = client
        .skills()
        .delete("skl_01DP8V5D1N6V3Q6N57V8Q9W0XE", None)
        .await
        .unwrap();
    assert_eq!(deleted.object_type.as_deref(), Some("skill_deleted"));
}

#[tokio::test]
async fn test_create_skill_multipart_upload() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/skills"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_payload()))
        .mount(&mock_server)
        .await;

    let client = setup_client(&mock_server);
    let request = SkillCreateRequest::new()
        .display_title("Meeting notes skill")
        .add_file(SkillFileUpload::new(
            "meeting-notes/SKILL.md",
            b"# Meeting notes\nThis is a test skill.".to_vec(),
            "text/markdown",
        ));

    let skill = client.skills().create(request, None).await.unwrap();
    assert_eq!(skill.id, "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE");

    let requests = mock_server.received_requests().await.unwrap();
    let beta_header = requests[0]
        .headers
        .get("anthropic-beta")
        .unwrap()
        .to_str()
        .unwrap();
    assert!(beta_header.contains("skills-2025-10-02"));
}

#[tokio::test]
async fn test_list_and_get_skill_versions() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/v1/skills/skl_01DP8V5D1N6V3Q6N57V8Q9W0XE/versions"))
        .and(query_param("limit", "10"))
        .and(query_param("page", "page_2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [sample_skill_version_payload()],
            "has_more": false
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path(
            "/v1/skills/skl_01DP8V5D1N6V3Q6N57V8Q9W0XE/versions/skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_version_payload()))
        .mount(&mock_server)
        .await;

    let client = setup_client(&mock_server);

    let params = SkillVersionListParams::new()
        .with_limit(10)
        .with_page("page_2");
    let versions = client
        .skills()
        .list_versions("skl_01DP8V5D1N6V3Q6N57V8Q9W0XE", Some(params), None)
        .await
        .unwrap();

    assert_eq!(versions.data.len(), 1);
    assert_eq!(
        versions.data[0].object_type.as_deref(),
        Some("skill_version")
    );

    let version = client
        .skills()
        .get_version(
            "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
            "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
            None,
        )
        .await
        .unwrap();
    assert_eq!(version.version.as_deref(), Some("1723500000"));
}

#[tokio::test]
async fn test_create_and_delete_skill_version() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/skills/skl_01DP8V5D1N6V3Q6N57V8Q9W0XE/versions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_version_payload()))
        .mount(&mock_server)
        .await;

    Mock::given(method("DELETE"))
        .and(path(
            "/v1/skills/skl_01DP8V5D1N6V3Q6N57V8Q9W0XE/versions/skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "type": "skill_version_deleted",
            "id": "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE"
        })))
        .mount(&mock_server)
        .await;

    let client = setup_client(&mock_server);
    let request = SkillVersionCreateRequest::new().add_file(SkillFileUpload::new(
        "meeting-notes/SKILL.md",
        b"# Meeting notes\nVersion 2".to_vec(),
        "text/markdown",
    ));

    let version = client
        .skills()
        .create_version("skl_01DP8V5D1N6V3Q6N57V8Q9W0XE", request, None)
        .await
        .unwrap();
    assert_eq!(version.id, "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE");

    let deleted = client
        .skills()
        .delete_version(
            "skl_01DP8V5D1N6V3Q6N57V8Q9W0XE",
            "skv_01HQ8V5D1N6V3Q6N57V8Q9W0XE",
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        deleted.object_type.as_deref(),
        Some("skill_version_deleted")
    );
}
