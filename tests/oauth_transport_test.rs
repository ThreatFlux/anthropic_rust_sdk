use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use threatflux_anthropic_sdk::oauth::wif::{
    FederationConfig, FederationTokenProvider, SubjectTokenSource,
};
use threatflux_anthropic_sdk::{
    Client, Config, HttpMethod, OAuthAdminClient, OAuthConfig, OAuthPrincipal, OAuthToken,
    RequestOptions, Result, TokenProvider,
};
use wiremock::{
    matchers::{body_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn config(server: &MockServer) -> OAuthConfig {
    let mut config = OAuthConfig::default();
    config.base_url = server.uri().parse().unwrap();
    config.max_retries = 0;
    config.refresh_skew = Duration::from_millis(1);
    config
}

struct Provider {
    calls: AtomicUsize,
    lifetime: Duration,
    scope: &'static str,
}
impl Provider {
    fn new(lifetime: Duration) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            lifetime,
            scope: "org:admin",
        }
    }
}
impl TokenProvider for Provider {
    fn token(&self) -> BoxFuture<'_, Result<OAuthToken>> {
        Box::pin(async move {
            let number = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
            tokio::task::yield_now().await;
            Ok(OAuthToken::new(format!("test-token-{number}"))?
                .with_expiration(SystemTime::now() + self.lifetime, self.scope))
        })
    }
}

#[test]
fn bearer_credentials_validate_and_redact() {
    for bad in [
        "",
        "   ",
        "a\r\nb",
        "sk-ant-api03-example",
        "sk-ant-admin-example",
    ] {
        assert!(OAuthToken::new(bad).is_err());
    }
    let token = OAuthToken::new("sensitive-test-token").unwrap();
    assert!(!format!("{token:?}").contains("sensitive-test-token"));
    let client = OAuthAdminClient::with_token(token, OAuthConfig::default()).unwrap();
    assert!(!format!("{client:?}").contains("sensitive-test-token"));
    let source = SubjectTokenSource::Token("secret-jwt".into());
    assert!(!format!("{source:?}").contains("secret-jwt"));
}

#[tokio::test]
async fn static_oauth_sends_only_bearer_and_blocks_overrides() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/v1/organizations/service_accounts"))
        .and(header("authorization", "Bearer sk-ant-oat01-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[]})))
        .expect(1)
        .mount(&server)
        .await;
    let client = OAuthAdminClient::new("sk-ant-oat01-test", config(&server)).unwrap();
    let _: Value = client
        .request(
            HttpMethod::Get,
            "/organizations/service_accounts",
            None,
            None,
        )
        .await
        .unwrap();
    for name in ["x-api-key", "X-API-KEY", "Authorization", "authorization"] {
        assert!(client
            .request::<Value>(
                HttpMethod::Get,
                "/organizations/service_accounts",
                None,
                Some(RequestOptions::new().with_header(name, "override"))
            )
            .await
            .is_err());
    }
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key("x-api-key"));
    assert!(requests[0].headers.contains_key("anthropic-version"));
}

#[tokio::test]
async fn provider_refresh_is_shared_across_clones_and_concurrent_requests() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/organizations/service_accounts"))
        .and(header("authorization", "Bearer test-token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":[]})))
        .expect(20)
        .mount(&server)
        .await;
    let provider = Arc::new(Provider::new(Duration::from_secs(3600)));
    let client = OAuthAdminClient::with_provider(provider.clone(), config(&server)).unwrap();
    let futures = (0..20).map(|_| {
        let client = client.clone();
        async move {
            client
                .request::<Value>(
                    HttpMethod::Get,
                    "/organizations/service_accounts",
                    None,
                    None,
                )
                .await
        }
    });
    for result in futures::future::join_all(futures).await {
        assert!(result.is_ok());
    }
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn read_401_refreshes_once_but_mutations_are_never_replayed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer test-token-1"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer test-token-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer test-token-2"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&server)
        .await;
    let provider = Arc::new(Provider::new(Duration::from_secs(3600)));
    let client = OAuthAdminClient::with_provider(provider.clone(), config(&server)).unwrap();
    let response = client
        .request::<Value>(
            HttpMethod::Get,
            "/organizations/service_accounts",
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(response, json!({}));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(
        client
            .request::<Value>(
                HttpMethod::Post,
                "/organizations/service_accounts",
                Some(json!({"name":"worker"})),
                None
            )
            .await
            .unwrap_err()
            .status_code(),
        Some(401)
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn expired_provider_tokens_refresh_and_bad_scope_or_expiry_fails_before_http() {
    let server = MockServer::start().await;
    Mock::given(header("authorization", "Bearer test-token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(header("authorization", "Bearer test-token-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    // Each token stays valid for a minute, but the refresh skew is longer, so the
    // cached token is already inside the refresh window when the second request
    // runs. This exercises the refresh path without depending on wall-clock
    // sleeps, which made the test flaky under slow instrumented runs (tarpaulin):
    // a 20 ms token could expire before the client accepted it.
    let provider = Arc::new(Provider::new(Duration::from_secs(60)));
    let mut config = config(&server);
    config.refresh_skew = Duration::from_secs(120);
    let client = OAuthAdminClient::with_provider(provider.clone(), config).unwrap();
    client
        .request::<Value>(
            HttpMethod::Get,
            "/organizations/service_accounts",
            None,
            None,
        )
        .await
        .unwrap();
    client
        .request::<Value>(
            HttpMethod::Get,
            "/organizations/service_accounts",
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}

#[tokio::test]
async fn federation_exchange_uses_pinned_json_and_rereads_rotating_subject_file() {
    let server = MockServer::start().await;
    let file = tempfile::NamedTempFile::new().unwrap();
    let organization = "00000000-0000-4000-8000-000000000000";
    for (subject, token) in [
        ("jwt-one", "sk-ant-oat01-one"),
        ("jwt-two", "sk-ant-oat01-two"),
    ] {
        Mock::given(method("POST")).and(path("/v1/oauth/token"))
            .and(body_json(json!({"grant_type":"urn:ietf:params:oauth:grant-type:jwt-bearer","assertion":subject,
                "federation_rule_id":"fdrl_example","organization_id":organization,
                "service_account_id":"svac_example","workspace_id":"wrkspc_example"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"access_token":token,"token_type":"Bearer","expires_in":600,"scope":"org:admin"})))
            .expect(1).mount(&server).await;
    }
    let federation = FederationConfig::new(
        organization,
        "svac_example",
        "fdrl_example",
        SubjectTokenSource::File(file.path().into()),
    )
    .unwrap()
    .with_workspace("wrkspc_example")
    .unwrap();
    let provider = FederationTokenProvider::new(federation, config(&server)).unwrap();
    for (subject, expected) in [
        ("jwt-one\n", "sk-ant-oat01-one"),
        ("jwt-two\n", "sk-ant-oat01-two"),
    ] {
        tokio::fs::write(file.path(), subject).await.unwrap();
        let token = provider.token().await.unwrap();
        assert_eq!(token.expose_secret(), expected);
        assert_eq!(token.principal, OAuthPrincipal::ServiceAccount);
        assert!(token.expires_at.unwrap() > SystemTime::now());
    }
    for request in server.received_requests().await.unwrap() {
        assert!(!request.headers.contains_key("x-api-key"));
        assert!(!request.headers.contains_key("authorization"));
    }
}

#[tokio::test]
async fn federation_errors_do_not_echo_subject_tokens_or_remote_payloads() {
    let server = MockServer::start().await;
    Mock::given(path("/v1/oauth/token"))
        .respond_with(
            ResponseTemplate::new(403).set_body_string("secret-subject-token was rejected"),
        )
        .expect(1)
        .mount(&server)
        .await;
    let federation = FederationConfig::new(
        "00000000-0000-4000-8000-000000000000",
        "svac_example",
        "fdrl_example",
        SubjectTokenSource::Token("secret-subject-token".into()),
    )
    .unwrap();
    let provider = FederationTokenProvider::new(federation, config(&server)).unwrap();
    let error = provider.token().await.unwrap_err();
    assert_eq!(error.status_code(), Some(403));
    assert!(!error.to_string().contains("secret-subject-token"));
}

#[tokio::test]
async fn generic_client_recognizes_anthropic_oauth_prefix_as_bearer() {
    let server = MockServer::start().await;
    Mock::given(header("authorization", "Bearer sk-ant-oat01-test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .mount(&server)
        .await;
    let client = Client::new(
        Config::new("sk-ant-oat01-test")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap())
            .with_max_retries(0),
    );
    client
        .request::<Value>(HttpMethod::Get, "/models", None, None)
        .await
        .unwrap();
    assert!(!server.received_requests().await.unwrap()[0]
        .headers
        .contains_key("x-api-key"));
}

#[tokio::test]
async fn invalid_provider_scope_and_expiry_fail_before_http() {
    let server = MockServer::start().await;
    for (expiry, scope) in [
        (SystemTime::now() - Duration::from_secs(1), "org:admin"),
        (
            SystemTime::now() + Duration::from_secs(60),
            "workspace:inference",
        ),
    ] {
        let token = OAuthToken::new("forbidden")
            .unwrap()
            .with_expiration(expiry, scope);
        let client = OAuthAdminClient::with_token(token, config(&server)).unwrap();
        assert!(client
            .request::<Value>(
                HttpMethod::Get,
                "/organizations/service_accounts",
                None,
                None
            )
            .await
            .is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}
