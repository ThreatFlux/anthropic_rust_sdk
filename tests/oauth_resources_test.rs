//! OAuth resource wire contracts pinned to official Python SDK 18f25547.
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};
use threatflux_anthropic_sdk::{
    oauth::resources::{
        FederationMatch, IssuerCreate, IssuerUpdate, Jwks, JwksInline, OAuthListParams,
        OrganizationRole, RuleCreate, RuleUpdate, ServiceAccountCreate, ServiceAccountUpdate,
    },
    OAuthAdminClient, OAuthConfig, OAuthPrincipal, OAuthToken, PaginationLimits, RequestOptions,
};
use wiremock::{
    matchers::{body_json, header, method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

fn client(server: &MockServer, principal: OAuthPrincipal) -> OAuthAdminClient {
    let mut config = OAuthConfig::default();
    config.base_url = server.uri().parse().unwrap();
    config.max_retries = 0;
    let token = OAuthToken::new("sk-ant-oat01-resource-tests")
        .unwrap()
        .with_expiration(SystemTime::now() + Duration::from_secs(3600), "org:admin")
        .with_principal(principal);
    OAuthAdminClient::with_token(token, config).unwrap()
}

fn account(id: &str) -> Value {
    json!({"id":id,"type":"service_account","name":"worker","organization_role":"developer","created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","created_by_actor_id":"user_creator","future_account":{"enabled":true}})
}

fn issuer(id: &str) -> Value {
    json!({"id":id,"type":"federation_issuer","name":"provider","issuer_url":"https://issuer.example.com","jwks":{"type":"discovery","extra_keys":true},"check_jti":true,"max_jwt_lifetime_seconds":3600,"created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","poll_status":{"consecutive_failures":2,"next_poll_at":null,"future_poll":7},"future_issuer":[]})
}

fn rule(id: &str) -> Value {
    json!({"id":id,"type":"federation_rule","name":"deploy","issuer_id":"fdis_provider","match":{"subject_prefix":"repo:org/repo:ref:refs/heads/main","extra_match":7},"target":{"type":"service_account","service_account_id":"svac_worker","extra_target":true},"oauth_scope":"workspace:developer","token_lifetime_seconds":3600,"applies_to_all_workspaces":false,"workspace_ids":["wrkspc_test"],"created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","future_rule":{"one":1}})
}

fn membership() -> Value {
    json!({"service_account_id":"svac_worker","workspace_id":"wrkspc_test","workspace_role":"workspace_developer","type":"service_account_workspace_member","implicit":false,"created_by_actor_id":"user_test"})
}

fn binding() -> Value {
    json!({"federation_rule_id":"fdrl_deploy","workspace_id":"wrkspc_test","type":"federation_rule_workspace","created_at":"2026-10-03T00:00:00Z","workspace_name":"Test"})
}

async fn mount(
    server: &MockServer,
    verb: &str,
    endpoint: &str,
    body: Option<Value>,
    response: Value,
) {
    let mut mock = Mock::given(method(verb)).and(path(endpoint)).and(header(
        "authorization",
        "Bearer sk-ant-oat01-resource-tests",
    ));
    if let Some(body) = body {
        mock = mock.and(body_json(body));
    }
    mock.respond_with(ResponseTemplate::new(200).set_body_json(response))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn service_account_lifecycle_and_null_description_match_post_contract() {
    let server = MockServer::start().await;
    let base = "/v1/organizations/service_accounts";
    mount(
        &server,
        "POST",
        base,
        Some(json!({"name":"worker","organization_role":"developer"})),
        account("svac_worker"),
    )
    .await;
    mount(
        &server,
        "GET",
        &format!("{base}/svac_worker"),
        None,
        account("svac_worker"),
    )
    .await;
    mount(
        &server,
        "POST",
        &format!("{base}/svac_worker"),
        Some(json!({"description":null})),
        account("svac_worker"),
    )
    .await;
    let mut archived = account("svac_worker");
    archived["archived_at"] = json!("2026-10-04T00:00:00Z");
    mount(
        &server,
        "POST",
        &format!("{base}/svac_worker/archive"),
        None,
        archived,
    )
    .await;
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

#[tokio::test]
async fn account_and_rule_workspace_subresources_use_expected_methods_and_dtos() {
    let server = MockServer::start().await;
    let accounts = "/v1/organizations/service_accounts/svac_worker/workspaces";
    let rules = "/v1/organizations/federation_rules/fdrl_deploy/workspaces";
    mount(
        &server,
        "GET",
        accounts,
        None,
        json!({"data":[membership()],"next_page":null}),
    )
    .await;
    mount(
        &server,
        "POST",
        accounts,
        Some(json!({"workspace_id":"wrkspc_test","workspace_role":"workspace_developer"})),
        membership(),
    )
    .await;
    mount(&server, "DELETE", &format!("{accounts}/wrkspc_test"), None, json!({"type":"service_account_workspace_member_deleted","service_account_id":"svac_worker","workspace_id":"wrkspc_test"})).await;
    mount(&server, "GET", rules, None, json!({"data":[binding()]})).await;
    mount(
        &server,
        "POST",
        rules,
        Some(json!({"workspace_id":"wrkspc_test"})),
        binding(),
    )
    .await;
    mount(&server, "DELETE", &format!("{rules}/wrkspc_test"), None, json!({"type":"federation_rule_workspace_deleted","federation_rule_id":"fdrl_deploy","workspace_id":"wrkspc_test"})).await;
    let organization = client(&server, OAuthPrincipal::User).organization();
    let accounts = organization.service_accounts();
    assert_eq!(
        accounts
            .workspaces("svac_worker", OAuthListParams::default(), None)
            .await
            .unwrap()
            .data
            .len(),
        1
    );
    accounts
        .add_workspace("svac_worker", "wrkspc_test", "workspace_developer", None)
        .await
        .unwrap();
    assert_eq!(
        accounts
            .remove_workspace("svac_worker", "wrkspc_test", None)
            .await
            .unwrap()
            .service_account_id,
        "svac_worker"
    );
    let rules = organization.federation_rules();
    assert_eq!(
        rules
            .workspaces("fdrl_deploy", None)
            .await
            .unwrap()
            .data
            .len(),
        1
    );
    rules
        .add_workspace("fdrl_deploy", "wrkspc_test", None)
        .await
        .unwrap();
    assert_eq!(
        rules
            .remove_workspace("fdrl_deploy", "wrkspc_test", None)
            .await
            .unwrap()
            .federation_rule_id,
        "fdrl_deploy"
    );
}

#[tokio::test]
async fn workspace_addressed_account_memberships_cover_get_list_add_update_remove() {
    let server = MockServer::start().await;
    let base = "/v1/organizations/workspaces/wrkspc_test/service_accounts";
    mount(
        &server,
        "GET",
        base,
        None,
        json!({"data":[membership()],"next_page":null}),
    )
    .await;
    mount(
        &server,
        "GET",
        &format!("{base}/svac_worker"),
        None,
        membership(),
    )
    .await;
    mount(
        &server,
        "POST",
        base,
        Some(json!({"service_account_id":"svac_worker","workspace_role":"workspace_developer"})),
        membership(),
    )
    .await;
    mount(
        &server,
        "POST",
        &format!("{base}/svac_worker"),
        Some(json!({"workspace_role":"workspace_user"})),
        membership(),
    )
    .await;
    mount(&server, "DELETE", &format!("{base}/svac_worker"), None, json!({"service_account_id":"svac_worker","workspace_id":"wrkspc_test","type":"service_account_workspace_member_deleted"})).await;
    let api = client(&server, OAuthPrincipal::User)
        .organization()
        .workspaces()
        .service_accounts("wrkspc_test")
        .unwrap();
    api.list(OAuthListParams::default(), None).await.unwrap();
    api.get("svac_worker", None).await.unwrap();
    api.add("svac_worker", "workspace_developer", None)
        .await
        .unwrap();
    api.update("svac_worker", "workspace_user", None)
        .await
        .unwrap();
    api.remove("svac_worker", None).await.unwrap();
}

#[tokio::test]
async fn known_workload_admin_promotion_and_unsupported_scopes_fail_locally() {
    let server = MockServer::start().await;
    let organization = client(&server, OAuthPrincipal::ServiceAccount).organization();
    let mut create = ServiceAccountCreate::new("admin-worker");
    create.organization_role = OrganizationRole::Admin;
    assert!(organization
        .service_accounts()
        .create(create, None)
        .await
        .is_err());
    let mut update = ServiceAccountUpdate::default();
    update.organization_role = Some(OrganizationRole::Admin);
    assert!(organization
        .service_accounts()
        .update("svac_worker", update, None)
        .await
        .is_err());
    let mut create = RuleCreate::new("rule", "fdis_provider", "svac_worker", "wrkspc_test");
    create.oauth_scope = "org:admin".into();
    assert!(organization
        .federation_rules()
        .create(create, None)
        .await
        .is_err());
    let mut update = RuleUpdate::default();
    update.oauth_scope = Some("workspace:manage_tunnels".into());
    assert!(organization
        .federation_rules()
        .update("fdrl_rule", update, None)
        .await
        .is_err());
    assert!(organization
        .service_accounts()
        .add_workspace("svac_worker", "wrkspc_test", "workspace_billing", None)
        .await
        .is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn interactive_user_can_create_admin_account() {
    let server = MockServer::start().await;
    mount(
        &server,
        "POST",
        "/v1/organizations/service_accounts",
        Some(json!({"name":"admin-worker","organization_role":"admin"})),
        account("svac_admin"),
    )
    .await;
    let mut request = ServiceAccountCreate::new("admin-worker");
    request.organization_role = OrganizationRole::Admin;
    client(&server, OAuthPrincipal::User)
        .organization()
        .service_accounts()
        .create(request, None)
        .await
        .unwrap();
}

#[tokio::test]
async fn jwks_roundtrip_reserved_keys_malformed_known_and_fetch_url_validation() {
    for value in [
        json!({"type":"future","nested":{"x":1}}),
        json!({"type":"explicit_url","url":"https://keys.example.com/jwks","extra_keys":true}),
        json!({"type":"inline","keys":[{"kty":"RSA","kid":"one","unknown":7}]}),
    ] {
        let jwks: Jwks = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(jwks).unwrap(), value);
    }
    assert!(serde_json::from_value::<Jwks>(json!({"type":"explicit_url"})).is_err());
    assert!(serde_json::from_value::<Jwks>(json!({"type":123})).is_err());
    let server = MockServer::start().await;
    let api = client(&server, OAuthPrincipal::User)
        .organization()
        .federation_issuers();
    let mut request = IssuerCreate::new("internal", "https://issuer.internal:8443");
    request.jwks = Jwks::inline(vec![HashMap::new()]);
    mount(
        &server,
        "POST",
        "/v1/organizations/federation_issuers",
        Some(serde_json::to_value(&request).unwrap()),
        issuer("fdis_internal"),
    )
    .await;
    api.create(request, None).await.unwrap();
    let mut request = IssuerCreate::new("bad", "https://issuer.example.com");
    let mut jwks: JwksInline = serde_json::from_value(json!({"type":"inline","keys":[]})).unwrap();
    jwks.extra.insert("type".into(), json!("discovery"));
    request.jwks = Jwks::Inline(jwks);
    assert!(api.create(request, None).await.is_err());
    assert!(api
        .create(
            IssuerCreate::new("bad", "https://issuer.internal:8443"),
            None
        )
        .await
        .is_err());
    let mut request = IssuerCreate::new("bad", "https://issuer.example.com");
    request.jwks = Jwks::Unknown(json!({"type":"future"}));
    assert!(api.create(request, None).await.is_err());
    let mut request = IssuerCreate::new("bad", "https://issuer.example.com");
    request.max_jwt_lifetime_seconds = Some(176_401);
    assert!(api.create(request, None).await.is_err());
    let mut update = IssuerUpdate::default();
    update.jwks_polling_disabled = Some(true);
    assert!(api.update("fdis_internal", update, None).await.is_err());
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

#[tokio::test]
async fn nonrestrictive_match_and_reserved_extensions_are_rejected_without_http() {
    let server = MockServer::start().await;
    let api = client(&server, OAuthPrincipal::User)
        .organization()
        .federation_rules();
    for matcher in [
        FederationMatch::default(),
        serde_json::from_value(json!({"audience":"aud-only"})).unwrap(),
        serde_json::from_value(json!({"subject_prefix":"*","condition":"true"})).unwrap(),
    ] {
        let mut request = RuleCreate::new("rule", "fdis_provider", "svac_worker", "wrkspc_test");
        request.r#match = matcher;
        assert!(api.create(request, None).await.is_err());
    }
    let mut request = RuleCreate::new("rule", "fdis_provider", "svac_worker", "wrkspc_test");
    request.r#match.subject_prefix = Some("subject".into());
    request
        .r#match
        .extra
        .insert("claims".into(), json!({"unsafe":"x"}));
    assert!(api.create(request, None).await.is_err());
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn archive_dependency_and_scope_errors_remain_actionable() {
    let server = MockServer::start().await;
    Mock::given(method("POST")).and(path("/v1/organizations/service_accounts/svac_worker/archive"))
        .respond_with(ResponseTemplate::new(400).set_body_json(json!({"type":"invalid_request_error","message":"Archive live referencing federation rules first"}))).expect(1).mount(&server).await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/federation_rules/fdrl_denied"))
        .respond_with(ResponseTemplate::new(403).set_body_json(
            json!({"type":"permission_error","message":"org:admin scope is required"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let organization = client(&server, OAuthPrincipal::Unknown).organization();
    let error = organization
        .service_accounts()
        .archive("svac_worker", None)
        .await
        .unwrap_err();
    assert_eq!(error.status_code(), Some(400));
    assert!(error.to_string().contains("referencing federation rules"));
    assert_eq!(
        organization
            .federation_rules()
            .get("fdrl_denied", None)
            .await
            .unwrap_err()
            .status_code(),
        Some(403)
    );
}
