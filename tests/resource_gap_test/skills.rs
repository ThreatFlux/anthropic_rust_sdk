//! Current Skills and version lifecycle contracts.

use super::*;

#[tokio::test]
async fn current_skill_lifecycle_uses_name_object_source_and_version_ids() {
    let server = MockServer::start().await;
    mount_current_skills(&server).await;
    mount_current_versions(&server).await;
    let api = client(&server).skills_current();
    let created = api
        .create(
            CurrentSkillCreateRequest::new()
                .display_name("My Skill")
                .add_file(upload_file()),
            Some(
                RequestOptions::new()
                    .with_beta_feature("unrelated-2026-01-01")
                    .with_header("anthropic-workspace-id", "workspace_test"),
            ),
        )
        .await
        .unwrap();
    assert_eq!(created.source.source_type, "custom");
    assert_eq!(created.source.extra["future_source"], true);
    assert_eq!(created.latest_version_id, "skv_current");
    assert_eq!(api.get("skl_current", None).await.unwrap(), created);
    assert_eq!(api.list(None, None).await.unwrap().data, vec![created]);
    assert_current_version_lifecycle(&api).await;
    api.delete("skl_current", None).await.unwrap();
    assert_current_skill_headers(&server).await;
}

#[tokio::test]
async fn current_skill_conflicting_beta_and_unsafe_layout_fail_locally() {
    let server = MockServer::start().await;
    let api = client(&server).skills_current();
    for options in [
        RequestOptions::new().with_skills_api(),
        RequestOptions::new().with_beta_feature("skills-2025-10-02"),
        RequestOptions::new().with_header("AnThRoPiC-BeTa", "other, skills-2025-10-02"),
    ] {
        assert!(api.list(None, Some(options)).await.is_err());
    }
    for name in [
        "../SKILL.md",
        "/root/SKILL.md",
        "one/../SKILL.md",
        "one\\SKILL.md",
        "one/readme.md",
    ] {
        let request = CurrentSkillCreateRequest::new().add_file(SkillFileUpload::new(
            name,
            vec![1],
            "text/plain",
        ));
        assert!(api.create(request, None).await.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

async fn mount_current_resource(server: &MockServer, verb: &str, endpoint: &str, payload: Value) {
    Mock::given(method(verb))
        .and(path(endpoint))
        .and(header("x-api-key", "sk-ant-test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(payload))
        .expect(1)
        .mount(server)
        .await;
}
async fn mount_current_skills(server: &MockServer) {
    for (verb, endpoint, payload) in [
        ("POST", "/v1/skills", skill("skl_current")),
        ("GET", "/v1/skills/skl_current", skill("skl_current")),
        (
            "GET",
            "/v1/skills",
            json!({"data":[skill("skl_current")],"next_page":null}),
        ),
        (
            "DELETE",
            "/v1/skills/skl_current",
            json!({"id":"skl_current","type":"skill_deleted"}),
        ),
    ] {
        mount_current_resource(server, verb, endpoint, payload).await;
    }
}
async fn mount_current_versions(server: &MockServer) {
    for (verb, endpoint, payload) in [
        (
            "POST",
            "/v1/skills/skl_current/versions",
            version("skv_current"),
        ),
        (
            "GET",
            "/v1/skills/skl_current/versions/skv_current",
            version("skv_current"),
        ),
        (
            "GET",
            "/v1/skills/skl_current/versions",
            json!({"data":[version("skv_current")],"next_page":null}),
        ),
        (
            "DELETE",
            "/v1/skills/skl_current/versions/skv_current",
            json!({"id":"skv_current","type":"skill_version_deleted"}),
        ),
    ] {
        mount_current_resource(server, verb, endpoint, payload).await;
    }
}
async fn assert_current_version_lifecycle(
    api: &threatflux_anthropic_sdk::api::skills::CurrentSkillsApi,
) {
    let uploaded = api
        .create_version(
            "skl_current",
            CurrentSkillVersionCreateRequest::new().add_file(upload_file()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        api.get_version("skl_current", "skv_current", None)
            .await
            .unwrap(),
        uploaded
    );
    assert_eq!(
        api.list_versions("skl_current", None, None)
            .await
            .unwrap()
            .data,
        vec![uploaded]
    );
    api.delete_version("skl_current", "skv_current", None)
        .await
        .unwrap();
}
async fn assert_current_skill_headers(server: &MockServer) {
    let requests = server.received_requests().await.unwrap();
    let form = String::from_utf8(requests[0].body.clone()).unwrap();
    assert!(form.contains("name=\"display_name\""));
    assert!(!form.contains("name=\"display_title\""));
    assert_eq!(
        requests[0].headers["anthropic-beta"],
        "unrelated-2026-01-01"
    );
    assert_eq!(
        requests[0].headers["anthropic-workspace-id"],
        "workspace_test"
    );
    assert!(requests.iter().all(|request| request
        .headers
        .get("anthropic-beta")
        .is_none_or(|value| !value.to_str().unwrap().contains("skills-2025-10-02"))));
}
