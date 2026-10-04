//! Explicit OAuth administration and optional workload identity federation.
//!
//! This client never uses `ANTHROPIC_API_KEY` or `ANTHROPIC_ADMIN_KEY`. Static
//! tokens are not refreshed. Providers must obtain tokens without interactive login.

pub mod resources;
pub mod wif;

use crate::{
    config::Config,
    error::{AnthropicError, Result},
    types::{HttpMethod, RequestOptions},
    utils::{http::HttpClient, retry::RetryClient},
};
use futures::future::BoxFuture;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::de::DeserializeOwned;
use std::{
    fmt,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::Mutex;
use url::Url;

/// A bearer credential, with optional expiration and scope supplied by a provider.
#[derive(Clone)]
pub struct OAuthToken {
    access_token: String,
    /// Token expiration. A missing expiry denotes a caller-managed static token.
    pub expires_at: Option<SystemTime>,
    /// Granted OAuth scope, when known.
    pub scope: Option<String>,
    /// Known caller kind; opaque static tokens default to unknown.
    pub principal: OAuthPrincipal,
}

/// Caller identity used for documented local permission checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OAuthPrincipal {
    /// Opaque credential; the service enforces its permissions.
    Unknown,
    /// Interactive user OAuth credential.
    User,
    /// Workload service-account OAuth credential.
    ServiceAccount,
}

impl fmt::Debug for OAuthToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthToken")
            .field("access_token", &"[REDACTED]")
            .field("expires_at", &self.expires_at)
            .field("scope", &self.scope)
            .finish()
    }
}

impl OAuthToken {
    /// Construct an explicit bearer credential, rejecting empty/invalid headers and API keys.
    pub fn new(token: impl Into<String>) -> Result<Self> {
        let token = token.into();
        if token.is_empty()
            || token.bytes().any(|byte| byte.is_ascii_whitespace())
            || token.starts_with("sk-ant-api")
            || token.starts_with("sk-ant-admin")
        {
            return Err(AnthropicError::auth(
                "An OAuth bearer token is required, not an API key",
            ));
        }
        HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| AnthropicError::auth("Invalid bearer credential header"))?;
        Ok(Self {
            access_token: token,
            expires_at: None,
            scope: None,
            principal: OAuthPrincipal::Unknown,
        })
    }

    /// Read the bearer value explicitly; callers must protect it as a secret.
    pub fn expose_secret(&self) -> &str {
        &self.access_token
    }

    /// Attach an expiration and the granted scope to a provider-issued token.
    pub fn with_expiration(mut self, expires_at: SystemTime, scope: impl Into<String>) -> Self {
        self.expires_at = Some(expires_at);
        self.scope = Some(scope.into());
        self
    }

    /// State a known principal kind; this does not grant server-side permissions.
    pub fn with_principal(mut self, principal: OAuthPrincipal) -> Self {
        self.principal = principal;
        self
    }

    fn usable(&self, skew: Duration) -> bool {
        self.expires_at.is_none_or(|expiry| {
            expiry
                .duration_since(SystemTime::now())
                .is_ok_and(|remaining| remaining > skew)
        })
    }
}

/// Fetch fresh credentials without interactive login. Calls are serialized by the client.
pub trait TokenProvider: Send + Sync {
    /// Obtain a token; do not log credential material or complete exchange bodies.
    fn token(&self) -> BoxFuture<'_, Result<OAuthToken>>;
}

/// OAuth transport settings, separate from API/admin-key configuration.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct OAuthConfig {
    /// Trusted API origin. The configured bearer token is sent here.
    pub base_url: Url,
    /// Timeout per attempt.
    pub timeout: Duration,
    /// Read-only request retries; mutations are sent once.
    pub max_retries: u32,
    /// User agent.
    pub user_agent: String,
    /// Refresh provider credentials before they enter this expiration window.
    pub refresh_skew: Duration,
}

impl Default for OAuthConfig {
    fn default() -> Self {
        Self {
            base_url: Url::parse("https://api.anthropic.com").expect("static URL"),
            timeout: Duration::from_secs(60),
            max_retries: 3,
            user_agent: format!("threatflux-anthropic-sdk/{}", env!("CARGO_PKG_VERSION")),
            refresh_skew: Duration::from_secs(30),
        }
    }
}

impl OAuthConfig {
    pub(crate) fn validate_origin(&self) -> Result<()> {
        if !self.base_url.username().is_empty()
            || self.base_url.password().is_some()
            || self.base_url.query().is_some()
            || self.base_url.fragment().is_some()
            || !matches!(self.base_url.path(), "" | "/")
        {
            return Err(AnthropicError::config(
                "OAuth base URL must be an origin without embedded credentials, path, query or fragment",
            ));
        }
        Ok(())
    }
}

#[derive(Clone)]
enum Credential {
    Static(OAuthToken),
    Provider(Arc<dyn TokenProvider>),
}

struct Inner {
    config: OAuthConfig,
    credential: Credential,
    cached: Mutex<Option<OAuthToken>>,
    http: HttpClient,
    retry: RetryClient,
}

/// Bearer-only client for administration resources requiring `org:admin` OAuth.
#[derive(Clone)]
pub struct OAuthAdminClient {
    inner: Arc<Inner>,
}

impl fmt::Debug for OAuthAdminClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OAuthAdminClient")
            .field("config", &self.inner.config)
            .finish_non_exhaustive()
    }
}

impl OAuthAdminClient {
    /// Use a caller-managed token. This token will never be refreshed automatically.
    pub fn new(token: impl Into<String>, config: OAuthConfig) -> Result<Self> {
        Self::build(Credential::Static(OAuthToken::new(token)?), config)
    }

    /// Use a caller-managed token with explicit expiration/scope/principal metadata.
    pub fn with_token(token: OAuthToken, config: OAuthConfig) -> Result<Self> {
        Self::build(Credential::Static(token), config)
    }

    /// Use an asynchronous, noninteractive token provider with shared refresh.
    pub fn with_provider(provider: Arc<dyn TokenProvider>, config: OAuthConfig) -> Result<Self> {
        Self::build(Credential::Provider(provider), config)
    }

    /// Read `ANTHROPIC_AUTH_TOKEN`, rejecting conflicting API/admin credentials.
    pub fn from_env() -> Result<Self> {
        if std::env::var_os("ANTHROPIC_API_KEY").is_some()
            || std::env::var_os("ANTHROPIC_ADMIN_KEY").is_some()
        {
            return Err(AnthropicError::config(
                "Unset API/admin key variables when selecting OAuth administration",
            ));
        }
        let token = std::env::var("ANTHROPIC_AUTH_TOKEN")
            .map_err(|_| AnthropicError::config("ANTHROPIC_AUTH_TOKEN is required"))?;
        let mut config = OAuthConfig::default();
        if let Ok(base) = std::env::var("ANTHROPIC_BASE_URL") {
            config.base_url =
                Url::parse(&base).map_err(|_| AnthropicError::config("Invalid OAuth base URL"))?;
        }
        Self::new(token, config)
    }

    fn build(credential: Credential, config: OAuthConfig) -> Result<Self> {
        config.validate_origin()?;
        let transport = Config::new("oauth-transport")?
            .with_base_url(config.base_url.clone())
            .with_timeout(config.timeout)
            .with_max_retries(config.max_retries)
            .with_user_agent(config.user_agent.clone());
        transport.validate()?;
        let transport = Arc::new(transport);
        Ok(Self {
            inner: Arc::new(Inner {
                config,
                credential,
                cached: Mutex::new(None),
                http: HttpClient::new(transport.clone()),
                retry: RetryClient::new(transport),
            }),
        })
    }

    /// Resource clients for service accounts and federation configuration.
    pub fn organization(&self) -> resources::OAuthOrganizationApi {
        resources::OAuthOrganizationApi::new(self.clone())
    }

    pub(crate) async fn require_interactive_if_known(&self) -> Result<()> {
        if self.credential(None).await?.principal == OAuthPrincipal::ServiceAccount {
            return Err(AnthropicError::auth("Admin-role service account creation/promotion requires an interactive user OAuth credential"));
        }
        Ok(())
    }

    async fn credential(&self, rejected: Option<&str>) -> Result<OAuthToken> {
        match &self.inner.credential {
            Credential::Static(token) => Ok(token.clone()),
            Credential::Provider(provider) => {
                let mut cached = self.inner.cached.lock().await;
                if let Some(token) = cached.as_ref() {
                    if token.usable(self.inner.config.refresh_skew)
                        && rejected != Some(token.expose_secret())
                    {
                        return Ok(token.clone());
                    }
                }
                let token = provider.token().await?;
                if !token.usable(Duration::ZERO) {
                    return Err(AnthropicError::auth(
                        "Token provider returned an expired credential",
                    ));
                }
                *cached = Some(token.clone());
                Ok(token)
            }
        }
    }

    fn headers(&self, token: &OAuthToken, options: &RequestOptions) -> Result<HeaderMap> {
        if token
            .scope
            .as_deref()
            .is_some_and(|scope| !scope.split_whitespace().any(|part| part == "org:admin"))
        {
            return Err(AnthropicError::auth(
                "Organization administration requires org:admin OAuth scope",
            ));
        }
        let mut headers = HeaderMap::new();
        let mut auth = HeaderValue::from_str(&format!("Bearer {}", token.expose_secret()))
            .map_err(|_| AnthropicError::auth("Invalid bearer credential header"))?;
        auth.set_sensitive(true);
        headers.insert(reqwest::header::AUTHORIZATION, auth);
        headers.insert(
            "anthropic-version",
            HeaderValue::from_static(crate::API_VERSION),
        );
        headers.insert(
            reqwest::header::USER_AGENT,
            HeaderValue::from_str(&self.inner.config.user_agent)
                .map_err(|_| AnthropicError::config("Invalid OAuth user agent"))?,
        );
        if !options.beta_features.is_empty() {
            headers.insert(
                "anthropic-beta",
                HeaderValue::from_str(&options.beta_features.join(","))
                    .map_err(|_| AnthropicError::invalid_input("Invalid beta header"))?,
            );
        }
        for (name, value) in &options.headers {
            let name = HeaderName::from_bytes(name.as_bytes())
                .map_err(|_| AnthropicError::invalid_input("Invalid header name"))?;
            if name == reqwest::header::AUTHORIZATION || name == "x-api-key" {
                return Err(AnthropicError::invalid_input(
                    "Authentication headers cannot override OAuth credentials",
                ));
            }
            headers.insert(
                name,
                HeaderValue::from_str(value)
                    .map_err(|_| AnthropicError::invalid_input("Invalid header value"))?,
            );
        }
        Ok(headers)
    }

    async fn invalidate_rejected(&self, token: &OAuthToken) {
        let mut cached = self.inner.cached.lock().await;
        if cached
            .as_ref()
            .is_some_and(|cached| cached.expose_secret() == token.expose_secret())
        {
            *cached = None;
        }
    }

    /// Call an organization administration endpoint with isolated bearer authentication.
    ///
    /// Reads honor retry options and can refresh once after a 401. Mutations are
    /// sent once, with no automatic credential replay after uncertain delivery.
    pub async fn request<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        path: &str,
        body: Option<serde_json::Value>,
        options: Option<RequestOptions>,
    ) -> Result<T> {
        let url = self.organization_url(path)?;
        let options = options.unwrap_or_default();
        let timeout = options.timeout.unwrap_or(self.inner.config.timeout);
        let token = self.credential(None).await?;
        if !token.usable(Duration::ZERO) {
            return Err(AnthropicError::auth("OAuth credential has expired"));
        }
        let headers = self.headers(&token, &options)?;
        let read = method == HttpMethod::Get;
        let result = self
            .send_request(
                method,
                &url,
                body.clone(),
                headers,
                timeout,
                read && !options.no_retry,
            )
            .await;
        if !Self::is_unauthorized(&result) {
            return result;
        }
        self.invalidate_rejected(&token).await;
        if read && matches!(self.inner.credential, Credential::Provider(_)) {
            return self
                .refresh_request(method, &url, body, &options, timeout, &token)
                .await;
        }
        result
    }

    fn organization_url(&self, path: &str) -> Result<Url> {
        if !path.starts_with("/organizations/") && path != "/organizations" {
            return Err(AnthropicError::invalid_input(
                "OAuth administration paths must start with /organizations",
            ));
        }
        let url = Url::parse(&format!(
            "{}/v1{}",
            self.inner.config.base_url.as_str().trim_end_matches('/'),
            path
        ))
        .map_err(|_| AnthropicError::invalid_input("Invalid organization resource path"))?;
        if url.fragment().is_some()
            || !(url.path() == "/v1/organizations" || url.path().starts_with("/v1/organizations/"))
        {
            return Err(AnthropicError::invalid_input(
                "OAuth administration paths must remain within organization resources",
            ));
        }
        Ok(url)
    }

    fn is_unauthorized<T>(result: &Result<T>) -> bool {
        result
            .as_ref()
            .err()
            .is_some_and(|error| error.status_code() == Some(401))
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        url: &Url,
        body: Option<serde_json::Value>,
        headers: HeaderMap,
        timeout: Duration,
        retry: bool,
    ) -> Result<T> {
        if retry {
            self.inner
                .retry
                .request(method, url, body, headers, timeout)
                .await
        } else {
            self.inner
                .http
                .request(method, url, body, headers, timeout)
                .await
        }
    }

    async fn refresh_request<T: DeserializeOwned>(
        &self,
        method: HttpMethod,
        url: &Url,
        body: Option<serde_json::Value>,
        options: &RequestOptions,
        timeout: Duration,
        rejected: &OAuthToken,
    ) -> Result<T> {
        let token = self.credential(Some(rejected.expose_secret())).await?;
        let headers = self.headers(&token, options)?;
        let result = self
            .inner
            .http
            .request(method, url, body, headers, timeout)
            .await;
        if Self::is_unauthorized(&result) {
            self.invalidate_rejected(&token).await;
        }
        result
    }
}
