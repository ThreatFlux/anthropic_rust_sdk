//! Bounded ID- and token-cursor contracts.

use super::*;

#[tokio::test]
async fn token_pages_preserve_filters_headers_and_continue_without_has_more() {
    let server = MockServer::start().await;
    mount_file_pages(&server).await;
    let files = client(&server)
        .files()
        .list_all_with_limits(
            FileListParams::new()
                .scope_id("session & one")
                .with_limit(7),
            PaginationLimits::default(),
            Some(
                RequestOptions::new()
                    .with_header("test-traversal", "preserved")
                    .with_files_api()
                    .with_timeout(Duration::from_secs(1))
                    .no_retry(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        files
            .iter()
            .map(|file| file.id.as_str())
            .collect::<Vec<_>>(),
        ["f1", "f2", "f3"]
    );
}

#[tokio::test]
async fn current_skill_and_version_token_traversal() {
    let server = MockServer::start().await;
    for (endpoint, first, second) in [
        ("/v1/skills", skill("s1"), skill("s2")),
        (
            "/v1/skills/skl_current/versions",
            version("v1"),
            version("v2"),
        ),
    ] {
        Mock::given(method("GET"))
            .and(path(endpoint))
            .and(query_param("page", "page_next"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({"data":[second],"next_page":null})),
            )
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":[first],"next_page":"page_next"})),
            )
            .expect(1)
            .mount(&server)
            .await;
    }
    let api = client(&server).skills_current();
    assert_eq!(
        api.list_all_with_limits(SkillListParams::new(), PaginationLimits::default(), None)
            .await
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        api.list_all_versions_with_limits(
            "skl_current",
            SkillVersionListParams::new(),
            PaginationLimits::default(),
            None
        )
        .await
        .unwrap()
        .len(),
        2
    );
}

#[tokio::test]
async fn model_id_pages_encode_cursors_and_preserve_options() {
    let server = MockServer::start().await;
    for (cursor, next, id, more) in [
        (None, "cursor & one", "m1", true),
        (Some("cursor & one"), "last", "m2", true),
        (Some("last"), "m3", "m3", false),
    ] {
        let mut mock = Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(query_param("limit", "5"))
            .and(header("test-options", "every-page"));
        if let Some(cursor) = cursor {
            mock = mock.and(query_param("after", cursor));
        } else {
            mock = mock.and(|request: &wiremock::Request| {
                request.url.query_pairs().all(|(key, _)| key != "after")
            });
        }
        mock.respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":[{"id":id,"type":"model","display_name":"Model","created_at":"2026-10-03T00:00:00Z"}],
            "has_more":more,"first_id":id,"last_id":next
        }))).expect(1).mount(&server).await;
    }
    let models = client(&server)
        .models()
        .list_all_with_limits(
            Pagination::new().with_limit(5),
            PaginationLimits::new(3, 3).unwrap(),
            Some(
                RequestOptions::new()
                    .with_header("test-options", "every-page")
                    .no_retry(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(
        models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        ["m1", "m2", "m3"]
    );
}

#[tokio::test]
async fn model_invalid_continuation_and_reverse_contract() {
    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/v1/models")).and(query_param("before", "start"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data":[{"id":"model","type":"model","display_name":"Model","created_at":"2026-10-03T00:00:00Z"}],
            "has_more":true,"first_id":null,"last_id":"must-not-be-used"
        }))).expect(1).mount(&server).await;
    assert!(client(&server)
        .models()
        .list_all_with_limits(
            Pagination::new().with_before("start"),
            PaginationLimits::default(),
            None
        )
        .await
        .is_err());
}

async fn mount_file_pages(server: &MockServer) {
    for (cursor, next, id) in [
        (None, Some("page one&two"), "f1"),
        (Some("page one&two"), Some("page_three"), "f2"),
        (Some("page_three"), None, "f3"),
    ] {
        let mut mock = Mock::given(method("GET"))
            .and(path("/v1/files"))
            .and(query_param("scope_id", "session & one"))
            .and(query_param("limit", "7"))
            .and(header("test-traversal", "preserved"));
        if let Some(cursor) = cursor {
            mock = mock.and(query_param("page", cursor));
        } else {
            mock = mock.and(|request: &wiremock::Request| {
                request.url.query_pairs().all(|(key, _)| key != "page")
            });
        }
        mock.respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"data":[file(id)],"next_page":next})),
        )
        .expect(1)
        .mount(server)
        .await;
    }
}
