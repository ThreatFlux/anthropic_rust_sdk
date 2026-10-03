//! Resource lifecycle and page-token contracts.

use super::*;

#[tokio::test]
async fn service_account_lifecycle_and_null_description_match_post_contract() {
    let server = MockServer::start().await;
    mount_account_lifecycle(&server).await;
    let api = client(&server, OAuthPrincipal::ServiceAccount)
        .organization()
        .service_accounts();
    let created = api
        .create(ServiceAccountCreate::new("worker"), None)
        .await
        .unwrap();
    assert_eq!(created.resource_type, "service_account");
    assert_eq!(created.created_by_actor_id.as_deref(), Some("user_creator"));
    assert_eq!(created.extra["future_account"], json!({"enabled":true}));
    assert_eq!(
        api.get("svac_worker", None).await.unwrap().id,
        "svac_worker"
    );
    let mut update = ServiceAccountUpdate::default();
    update.description = Some(None);
    api.update("svac_worker", update, None).await.unwrap();
    assert!(api
        .archive("svac_worker", None)
        .await
        .unwrap()
        .archived_at
        .is_some());
    for request in server.received_requests().await.unwrap() {
        assert!(!request.headers.contains_key("x-api-key"));
    }
}

#[tokio::test]
async fn issuer_lifecycle_and_polling_state_are_typed() {
    let server = MockServer::start().await;
    let base = "/v1/organizations/federation_issuers";
    mount(&server, "POST", base, Some(json!({"name":"provider","issuer_url":"https://issuer.example.com","jwks":{"type":"discovery"}})), issuer("fdis_provider")).await;
    mount(
        &server,
        "GET",
        &format!("{base}/fdis_provider"),
        None,
        issuer("fdis_provider"),
    )
    .await;
    mount(
        &server,
        "POST",
        &format!("{base}/fdis_provider"),
        Some(json!({"jwks_polling_disabled":false})),
        issuer("fdis_provider"),
    )
    .await;
    mount(
        &server,
        "POST",
        &format!("{base}/fdis_provider/archive"),
        None,
        issuer("fdis_provider"),
    )
    .await;
    let api = client(&server, OAuthPrincipal::ServiceAccount)
        .organization()
        .federation_issuers();
    let created = api
        .create(
            IssuerCreate::new("provider", "https://issuer.example.com"),
            None,
        )
        .await
        .unwrap();
    assert_eq!(created.poll_status.unwrap().consecutive_failures, 2);
    match created.jwks {
        Jwks::Discovery(config) => assert_eq!(config.extra["extra_keys"], true),
        _ => panic!("wrong JWKS type"),
    }
    api.get("fdis_provider", None).await.unwrap();
    let mut update = IssuerUpdate::default();
    update.jwks_polling_disabled = Some(false);
    api.update("fdis_provider", update, None).await.unwrap();
    api.archive("fdis_provider", None).await.unwrap();
}

#[tokio::test]
async fn rule_lifecycle_sends_restrictive_match_workspace_and_typed_target() {
    let server = MockServer::start().await;
    let base = "/v1/organizations/federation_rules";
    // Synthetic Anthropic resource discriminator; this fixture contains no Google credentials.
    // nosemgrep: generic.secrets.security.detected-google-gcm-service-account.detected-google-gcm-service-account
    mount(&server, "POST", base, Some(json!({"name":"deploy","issuer_id":"fdis_provider","match":{"subject_prefix":"repo:org/repo:ref:refs/heads/main"},"target":{"type":"service_account","service_account_id":"svac_worker"},"oauth_scope":"workspace:developer","workspace_id":"wrkspc_test"})), rule("fdrl_deploy")).await;
    mount(
        &server,
        "GET",
        &format!("{base}/fdrl_deploy"),
        None,
        rule("fdrl_deploy"),
    )
    .await;
    mount(
        &server,
        "POST",
        &format!("{base}/fdrl_deploy"),
        Some(json!({"description":null,"token_lifetime_seconds":60})),
        rule("fdrl_deploy"),
    )
    .await;
    mount(
        &server,
        "POST",
        &format!("{base}/fdrl_deploy/archive"),
        None,
        rule("fdrl_deploy"),
    )
    .await;
    let api = client(&server, OAuthPrincipal::ServiceAccount)
        .organization()
        .federation_rules();
    let mut request = RuleCreate::new("deploy", "fdis_provider", "svac_worker", "wrkspc_test");
    request.r#match.subject_prefix = Some("repo:org/repo:ref:refs/heads/main".into());
    let created = api.create(request, None).await.unwrap();
    assert_eq!(created.target.extra["extra_target"], true);
    assert_eq!(created.r#match.extra["extra_match"], 7);
    assert_eq!(created.extra["future_rule"], json!({"one":1}));
    api.get("fdrl_deploy", None).await.unwrap();
    let mut update = RuleUpdate::default();
    update.description = Some(None);
    update.token_lifetime_seconds = Some(60);
    api.update("fdrl_deploy", update, None).await.unwrap();
    api.archive("fdrl_deploy", None).await.unwrap();
}

#[tokio::test]
async fn oauth_pages_preserve_rule_filter_archive_flag_and_options() {
    let server = MockServer::start().await;
    for (cursor, next, id) in [
        (None, Some("next & page"), "fdrl_one"),
        (Some("next & page"), None, "fdrl_two"),
    ] {
        let mut mock = Mock::given(method("GET"))
            .and(path("/v1/organizations/federation_rules"))
            .and(query_param("issuer_id", "fdis_filter"))
            .and(query_param("include_archived", "true"))
            .and(query_param("limit", "2"))
            .and(header("test-option", "same"));
        if let Some(cursor) = cursor {
            mock = mock.and(query_param("page", cursor));
        } else {
            mock = mock.and(|request: &wiremock::Request| {
                request.url.query_pairs().all(|(key, _)| key != "page")
            });
        }
        mock.respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":[rule(id)],"next_page":next,"future_page":3})),
        )
        .expect(1)
        .mount(&server)
        .await;
    }
    let api = client(&server, OAuthPrincipal::Unknown)
        .organization()
        .federation_rules();
    let mut params = OAuthListParams::default();
    params.limit = Some(2);
    params.include_archived = Some(true);
    params.issuer_id = Some("fdis_filter".into());
    let entries = api
        .list_all_with_limits(
            params,
            PaginationLimits::new(2, 2).unwrap(),
            Some(
                RequestOptions::new()
                    .with_header("test-option", "same")
                    .no_retry(),
            ),
        )
        .await
        .unwrap();
    assert_eq!(entries.len(), 2);
}

async fn mount_account_lifecycle(server: &MockServer) {
    let base = "/v1/organizations/service_accounts";
    mount(
        server,
        "POST",
        base,
        Some(json!({"name":"worker","organization_role":"developer"})),
        account("svac_worker"),
    )
    .await;
    mount(
        server,
        "GET",
        &format!("{base}/svac_worker"),
        None,
        account("svac_worker"),
    )
    .await;
    mount(
        server,
        "POST",
        &format!("{base}/svac_worker"),
        Some(json!({"description":null})),
        account("svac_worker"),
    )
    .await;
    let mut archived = account("svac_worker");
    archived["archived_at"] = json!("2026-10-04T00:00:00Z");
    mount(
        server,
        "POST",
        &format!("{base}/svac_worker/archive"),
        None,
        archived,
    )
    .await;
}
