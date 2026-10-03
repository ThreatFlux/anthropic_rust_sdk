//! Adversarial OAuth transport checks from an independent implementation review.

use futures::future::BoxFuture;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::{Duration, SystemTime},
};
use threatflux_anthropic_sdk::{
    error::{AnthropicError, Result},
    oauth::{
        wif::{FederationConfig, FederationTokenProvider, SubjectTokenSource},
        OAuthAdminClient, OAuthConfig, OAuthToken, TokenProvider,
    },
    types::HttpMethod,
};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

struct RotatingCredentials {
    calls: AtomicUsize,
}

#[test]
fn administration_and_federation_reject_base_urls_with_credential_or_query_material() {
    for base in [
        "http://user:password@localhost",
        "http://localhost?unexpected=query",
        "http://localhost#unexpected-fragment",
        "http://localhost/unexpected-prefix",
    ] {
        let mut config = OAuthConfig::default();
        config.base_url = base.parse().unwrap();
        assert!(OAuthAdminClient::new("oauth-static-test", config.clone()).is_err());
        let federation = FederationConfig::new(
            "00000000-0000-0000-0000-000000000000",
            "svac_test",
            "fdrl_test",
            SubjectTokenSource::Token("header.payload.signature".into()),
        )
        .unwrap();
        assert!(FederationTokenProvider::new(federation, config).is_err());
    }
}
impl TokenProvider for RotatingCredentials {
    fn token(&self) -> BoxFuture<'_, Result<OAuthToken>> {
        Box::pin(async move {
            let generation = self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(OAuthToken::new(format!("oauth-test-{generation}"))?
                .with_expiration(SystemTime::now() + Duration::from_secs(600), "org:admin"))
        })
    }
}

#[tokio::test]
async fn mutation_401_is_not_replayed_but_next_explicit_request_uses_fresh_credentials() {
    let server = MockServer::start().await;
    let provider = Arc::new(RotatingCredentials {
        calls: AtomicUsize::new(0),
    });
    let mut config = OAuthConfig::default();
    config.base_url = server.uri().parse().unwrap();
    let client = OAuthAdminClient::with_provider(provider.clone(), config).unwrap();
    Mock::given(method("POST"))
        .and(path("/v1/organizations/service_accounts"))
        .and(header("authorization", "Bearer oauth-test-0"))
        .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error":"revoked"})))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/v1/organizations/service_accounts"))
        .and(header("authorization", "Bearer oauth-test-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok":true})))
        .expect(1)
        .mount(&server)
        .await;
    let first: Result<Value> = client
        .request(
            HttpMethod::Post,
            "/organizations/service_accounts",
            Some(json!({"name":"first"})),
            None,
        )
        .await;
    assert_eq!(first.unwrap_err().status_code(), Some(401));
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        1,
        "a mutation must not be replayed automatically"
    );
    let second: Value = client
        .request(
            HttpMethod::Post,
            "/organizations/service_accounts",
            Some(json!({"name":"second"})),
            None,
        )
        .await
        .unwrap();
    assert_eq!(second["ok"], true);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn normalized_paths_cannot_escape_the_organization_api_prefix() {
    let server = MockServer::start().await;
    let mut config = OAuthConfig::default();
    config.base_url = server.uri().parse().unwrap();
    let client = OAuthAdminClient::new("oauth-static-test", config).unwrap();
    for path in [
        "/organizations/../../messages",
        "/organizations/%2e%2e/%2e%2e/messages",
    ] {
        let result: Result<Value> = client.request(HttpMethod::Get, path, None, None).await;
        assert!(
            matches!(result, Err(AnthropicError::InvalidInput(_))),
            "escaping path {path} must fail before HTTP"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn federation_rejects_subjects_above_the_official_16kib_limit_without_http() {
    let server = MockServer::start().await;
    let mut transport = OAuthConfig::default();
    transport.base_url = server.uri().parse().unwrap();
    let oversized = "x".repeat(16 * 1024 + 1);
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("projected-jwt");
    tokio::fs::write(&file, &oversized).await.unwrap();
    for subject in [
        SubjectTokenSource::Token(oversized),
        SubjectTokenSource::File(file),
    ] {
        let federation = FederationConfig::new(
            "00000000-0000-0000-0000-000000000000",
            "svac_test",
            "fdrl_test",
            subject,
        )
        .unwrap();
        let provider = FederationTokenProvider::new(federation, transport.clone()).unwrap();
        assert!(matches!(
            provider.token().await,
            Err(AnthropicError::Auth(_))
        ));
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn a_second_401_is_bounded_and_does_not_leave_the_refreshed_token_cached() {
    let server = MockServer::start().await;
    let provider = Arc::new(RotatingCredentials {
        calls: AtomicUsize::new(0),
    });
    let mut config = OAuthConfig::default();
    config.base_url = server.uri().parse().unwrap();
    let client = OAuthAdminClient::with_provider(provider.clone(), config).unwrap();
    for generation in 0..2 {
        Mock::given(method("GET"))
            .and(path("/v1/organizations/service_accounts"))
            .and(header(
                "authorization",
                format!("Bearer oauth-test-{generation}"),
            ))
            .respond_with(ResponseTemplate::new(401).set_body_json(json!({"error":"revoked"})))
            .expect(1)
            .mount(&server)
            .await;
    }
    Mock::given(method("GET"))
        .and(path("/v1/organizations/service_accounts"))
        .and(header("authorization", "Bearer oauth-test-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok":true})))
        .expect(1)
        .mount(&server)
        .await;
    let first: Result<Value> = client
        .request(
            HttpMethod::Get,
            "/organizations/service_accounts",
            None,
            None,
        )
        .await;
    assert_eq!(first.unwrap_err().status_code(), Some(401));
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        2,
        "only one refresh after the initial 401"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
    let next: Value = client
        .request(
            HttpMethod::Get,
            "/organizations/service_accounts",
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(next["ok"], true);
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        3,
        "next explicit request refreshes the rejected token"
    );
}
