//! Local mutation validation and actionable server failures.

use super::*;

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
