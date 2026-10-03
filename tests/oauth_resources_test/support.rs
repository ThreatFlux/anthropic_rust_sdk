//! Synthetic OAuth wire fixtures and mock transport.

use super::*;

pub(super) fn client(server: &MockServer, principal: OAuthPrincipal) -> OAuthAdminClient {
    let mut config = OAuthConfig::default();
    config.base_url = server.uri().parse().unwrap();
    config.max_retries = 0;
    let token = OAuthToken::new("sk-ant-oat01-resource-tests")
        .unwrap()
        .with_expiration(SystemTime::now() + Duration::from_secs(3600), "org:admin")
        .with_principal(principal);
    OAuthAdminClient::with_token(token, config).unwrap()
}

pub(super) fn account(id: &str) -> Value {
    // Synthetic Anthropic resource discriminator; this fixture contains no Google credentials.
    // nosemgrep: generic.secrets.security.detected-google-gcm-service-account.detected-google-gcm-service-account
    json!({"id":id,"type":"service_account","name":"worker","organization_role":"developer","created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","created_by_actor_id":"user_creator","future_account":{"enabled":true}})
}

pub(super) fn issuer(id: &str) -> Value {
    json!({"id":id,"type":"federation_issuer","name":"provider","issuer_url":"https://issuer.example.com","jwks":{"type":"discovery","extra_keys":true},"check_jti":true,"max_jwt_lifetime_seconds":3600,"created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","poll_status":{"consecutive_failures":2,"next_poll_at":null,"future_poll":7},"future_issuer":[]})
}

pub(super) fn rule(id: &str) -> Value {
    // Synthetic Anthropic resource discriminator; this fixture contains no Google credentials.
    // nosemgrep: generic.secrets.security.detected-google-gcm-service-account.detected-google-gcm-service-account
    json!({"id":id,"type":"federation_rule","name":"deploy","issuer_id":"fdis_provider","match":{"subject_prefix":"repo:org/repo:ref:refs/heads/main","extra_match":7},"target":{"type":"service_account","service_account_id":"svac_worker","extra_target":true},"oauth_scope":"workspace:developer","token_lifetime_seconds":3600,"applies_to_all_workspaces":false,"workspace_ids":["wrkspc_test"],"created_at":"2026-10-03T00:00:00Z","updated_at":"2026-10-03T00:00:00Z","future_rule":{"one":1}})
}

pub(super) fn membership() -> Value {
    // Synthetic Anthropic resource discriminator; this fixture contains no Google credentials.
    // nosemgrep: generic.secrets.security.detected-google-gcm-service-account.detected-google-gcm-service-account
    json!({"service_account_id":"svac_worker","workspace_id":"wrkspc_test","workspace_role":"workspace_developer","type":"service_account_workspace_member","implicit":false,"created_by_actor_id":"user_test"})
}

pub(super) fn binding() -> Value {
    json!({"federation_rule_id":"fdrl_deploy","workspace_id":"wrkspc_test","type":"federation_rule_workspace","created_at":"2026-10-03T00:00:00Z","workspace_name":"Test"})
}

pub(super) async fn mount(
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
