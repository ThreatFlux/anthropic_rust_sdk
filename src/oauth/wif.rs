//! Noninteractive JWT-bearer federation exchange.
//!
//! Schema: <https://platform.claude.com/docs/en/manage-claude/wif-reference>,
//! verified 2026-10-03. No cloud metadata or external CLI is accessed.

use super::{OAuthConfig, OAuthPrincipal, OAuthToken, TokenProvider};
use crate::{
    config::Config,
    error::{AnthropicError, Result},
    types::HttpMethod,
    utils::http::HttpClient,
};
use futures::future::BoxFuture;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::io::AsyncReadExt;
use url::Url;

const MAX_SUBJECT_BYTES: usize = 16 * 1024;

/// Explicit subject-token source. A file is reread for every exchange.
#[derive(Clone)]
pub enum SubjectTokenSource {
    /// Caller-supplied OIDC JWT. It cannot rotate automatically.
    Token(String),
    /// Rotating projected OIDC JWT file.
    File(PathBuf),
}

impl fmt::Debug for SubjectTokenSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Token(_) => f.write_str("Token([REDACTED])"),
            Self::File(path) => f.debug_tuple("File").field(path).finish(),
        }
    }
}

/// Federation rule and target selected explicitly by the caller.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct FederationConfig {
    /// Organization UUID.
    pub organization_id: String,
    /// Tagged service-account ID (`svac_...`).
    pub service_account_id: String,
    /// Tagged federation-rule ID (`fdrl_...`).
    pub federation_rule_id: String,
    /// Required for rules enabled in multiple workspaces.
    pub workspace_id: Option<String>,
    /// Subject token source; no ambient discovery is performed.
    pub subject: SubjectTokenSource,
}

impl FederationConfig {
    /// Select the target identities and subject token source.
    pub fn new(
        organization_id: impl Into<String>,
        service_account_id: impl Into<String>,
        federation_rule_id: impl Into<String>,
        subject: SubjectTokenSource,
    ) -> Result<Self> {
        let result = Self {
            organization_id: organization_id.into(),
            service_account_id: service_account_id.into(),
            federation_rule_id: federation_rule_id.into(),
            workspace_id: None,
            subject,
        };
        result.validate()?;
        Ok(result)
    }

    /// Bind the exchange to one workspace.
    pub fn with_workspace(mut self, workspace_id: impl Into<String>) -> Result<Self> {
        self.workspace_id = Some(workspace_id.into());
        self.validate()?;
        Ok(self)
    }

    /// Read only the explicit WIF environment variables, rejecting ambiguous credentials.
    ///
    /// Profiles, interactive login, and implicit cloud credential discovery are not used.
    pub fn from_env() -> Result<Self> {
        for variable in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_ADMIN_KEY",
            "ANTHROPIC_AUTH_TOKEN",
        ] {
            if std::env::var_os(variable).is_some() {
                return Err(AnthropicError::config(
                    "Unset API/admin/bearer credentials when selecting explicit federation",
                ));
            }
        }
        fn required(name: &str) -> Result<String> {
            std::env::var(name).map_err(|_| AnthropicError::config(format!("{name} is required")))
        }
        let subject = match (
            std::env::var("ANTHROPIC_IDENTITY_TOKEN"),
            std::env::var_os("ANTHROPIC_IDENTITY_TOKEN_FILE"),
        ) {
            (Ok(token), None) => SubjectTokenSource::Token(token),
            (Err(_), Some(file)) => SubjectTokenSource::File(file.into()),
            _ => {
                return Err(AnthropicError::config(
                    "Select exactly one subject-token string or file",
                ))
            }
        };
        let mut config = Self::new(
            required("ANTHROPIC_ORGANIZATION_ID")?,
            required("ANTHROPIC_SERVICE_ACCOUNT_ID")?,
            required("ANTHROPIC_FEDERATION_RULE_ID")?,
            subject,
        )?;
        config.workspace_id = std::env::var("ANTHROPIC_WORKSPACE_ID").ok();
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if uuid::Uuid::parse_str(&self.organization_id).is_err()
            || !valid_id(&self.service_account_id, "svac_")
            || !valid_id(&self.federation_rule_id, "fdrl_")
            || self
                .workspace_id
                .as_deref()
                .is_some_and(|id| !valid_id(id, "wrkspc_"))
        {
            return Err(AnthropicError::invalid_input(
                "Invalid federation organization, account, rule or workspace ID",
            ));
        }
        Ok(())
    }
}

fn valid_id(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|suffix| {
        !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
    })
}

#[derive(Serialize)]
struct ExchangeRequest<'a> {
    grant_type: &'static str,
    assertion: &'a str,
    federation_rule_id: &'a str,
    organization_id: &'a str,
    service_account_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace_id: Option<&'a str>,
}

#[derive(Deserialize)]
struct ExchangeResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
    scope: String,
}

/// Exchanges a supplied OIDC JWT for a short-lived Anthropic bearer token.
///
/// Use with [`super::OAuthAdminClient::with_provider`] for shared caching/refresh.
/// Administration requires an existing `org:admin` rule created in the Console.
pub struct FederationTokenProvider {
    federation: FederationConfig,
    transport: OAuthConfig,
    http: HttpClient,
}

impl fmt::Debug for FederationTokenProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FederationTokenProvider")
            .field("federation", &self.federation)
            .field("transport", &self.transport)
            .finish_non_exhaustive()
    }
}

impl FederationTokenProvider {
    /// Configure explicit federation and trusted transport. Exchanges are sent once.
    pub fn new(federation: FederationConfig, transport: OAuthConfig) -> Result<Self> {
        federation.validate()?;
        transport.validate_origin()?;
        let config = Config::new("oauth-exchange-transport")?
            .with_base_url(transport.base_url.clone())
            .with_timeout(transport.timeout)
            .with_user_agent(transport.user_agent.clone());
        config.validate()?;
        Ok(Self {
            federation,
            transport,
            http: HttpClient::new(Arc::new(config)),
        })
    }

    async fn exchange(&self) -> Result<OAuthToken> {
        let subject = self.read_subject().await?;
        let url = Url::parse(&format!(
            "{}/v1/oauth/token",
            self.transport.base_url.as_str().trim_end_matches('/')
        ))
        .map_err(|_| AnthropicError::config("Invalid exchange URL"))?;
        let body = serde_json::to_value(ExchangeRequest {
            grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
            assertion: &subject,
            federation_rule_id: &self.federation.federation_rule_id,
            organization_id: &self.federation.organization_id,
            service_account_id: &self.federation.service_account_id,
            workspace_id: self.federation.workspace_id.as_deref(),
        })?;
        let response: ExchangeResponse = self
            .http
            .request(
                HttpMethod::Post,
                &url,
                Some(body),
                reqwest::header::HeaderMap::new(),
                self.transport.timeout,
            )
            .await
            .map_err(|error| match error.status_code() {
                Some(status) => AnthropicError::api_error(
                    status,
                    "Federation exchange rejected; check rule, JWT and permissions".to_owned(),
                    None,
                ),
                None => AnthropicError::auth("Federation exchange failed"),
            })?;
        response.into_token()
    }

    async fn read_subject(&self) -> Result<String> {
        let subject = match &self.federation.subject {
            SubjectTokenSource::Token(token) => token.clone(),
            SubjectTokenSource::File(path) => {
                let file = tokio::fs::File::open(path)
                    .await
                    .map_err(|_| AnthropicError::auth("Cannot read subject-token file"))?;
                let mut bytes = Vec::new();
                file.take((MAX_SUBJECT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .await
                    .map_err(|_| AnthropicError::auth("Cannot read subject-token file"))?;
                String::from_utf8(bytes)
                    .map_err(|_| AnthropicError::auth("Subject-token file must contain UTF-8"))?
            }
        };
        let subject = subject.trim().to_owned();
        if subject.is_empty() || subject.len() > MAX_SUBJECT_BYTES {
            return Err(AnthropicError::auth(
                "Subject token is empty or exceeds the byte limit",
            ));
        }
        Ok(subject)
    }
}

impl ExchangeResponse {
    fn into_token(self) -> Result<OAuthToken> {
        if !self.token_type.eq_ignore_ascii_case("Bearer") || self.expires_in == 0 {
            return Err(AnthropicError::auth(
                "Invalid federation token type or lifetime",
            ));
        }
        let expires_at = SystemTime::now()
            .checked_add(Duration::from_secs(self.expires_in))
            .ok_or_else(|| AnthropicError::auth("Invalid federation token lifetime"))?;
        Ok(OAuthToken::new(self.access_token)?
            .with_expiration(expires_at, self.scope)
            .with_principal(OAuthPrincipal::ServiceAccount))
    }
}

impl TokenProvider for FederationTokenProvider {
    fn token(&self) -> BoxFuture<'_, Result<OAuthToken>> {
        Box::pin(self.exchange())
    }
}
