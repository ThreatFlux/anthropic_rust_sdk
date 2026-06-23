//! Configuration for the Anthropic API client

use crate::error::{AnthropicError, Result};
use std::time::Duration;
use url::Url;

/// Default model to use when none is specified.
///
/// A current, active, balanced model. Override per-request with
/// [`crate::models::message::MessageRequest::model`] — e.g. `claude-opus-4-8`
/// or `claude-fable-5` for the most capable, `claude-haiku-4-5` for the cheapest.
pub const DEFAULT_MODEL: &str = models::SONNET_4_6;

/// Available Claude models.
///
/// Model ids are passed as plain strings, so any id can be used; these constants
/// track the current catalog. Retired ids are kept (deprecated) for source
/// compatibility but will return `404` from the API.
pub mod models {
    // --- Current models ---------------------------------------------------

    /// Claude Fable 5 — most capable widely released model. Always-on thinking;
    /// requires 30-day data retention.
    pub const FABLE_5: &str = "claude-fable-5";
    /// Claude Mythos 5 — same capabilities as Fable 5 (Project Glasswing only).
    pub const MYTHOS_5: &str = "claude-mythos-5";
    /// Claude Opus 4.8 — most capable Opus-tier model.
    pub const OPUS_4_8: &str = "claude-opus-4-8";
    /// Claude Opus 4.7 — previous-generation Opus.
    pub const OPUS_4_7: &str = "claude-opus-4-7";
    /// Claude Opus 4.6.
    pub const OPUS_4_6: &str = "claude-opus-4-6";
    /// Claude Sonnet 4.6 — best balance of speed and intelligence.
    pub const SONNET_4_6: &str = "claude-sonnet-4-6";
    /// Claude Haiku 4.5 — fastest, most cost-effective.
    pub const HAIKU_4_5: &str = "claude-haiku-4-5";

    // --- Legacy (still active) -------------------------------------------

    /// Claude Opus 4.5.
    pub const OPUS_4_5: &str = "claude-opus-4-5";
    /// Claude Sonnet 4.5.
    pub const SONNET_4_5: &str = "claude-sonnet-4-5";
    /// Claude Opus 4.1 — deprecated (retires 2026-08-05); prefer [`OPUS_4_8`].
    pub const OPUS_4_1: &str = "claude-opus-4-1";

    // --- Retired (return 404; kept for source compatibility) -------------

    /// Retired — use [`OPUS_4_8`].
    #[deprecated(note = "retired by Anthropic; use OPUS_4_8")]
    pub const OPUS_4: &str = "claude-opus-4-20250514";
    /// Retired — use [`SONNET_4_6`].
    #[deprecated(note = "retired by Anthropic; use SONNET_4_6")]
    pub const SONNET_4: &str = "claude-sonnet-4-20250514";
    /// Retired — use [`SONNET_4_6`].
    #[deprecated(note = "retired by Anthropic; use SONNET_4_6")]
    pub const SONNET_3_7: &str = "claude-3-7-sonnet-20250219";
    /// Retired — use [`HAIKU_4_5`].
    #[deprecated(note = "retired by Anthropic; use HAIKU_4_5")]
    pub const HAIKU_3_5: &str = "claude-3-5-haiku-20241022";
    /// Retired — use [`SONNET_4_6`].
    #[deprecated(note = "retired by Anthropic; use SONNET_4_6")]
    pub const SONNET_3_5: &str = "claude-3-5-sonnet-20241022";
    /// Retired — use [`OPUS_4_8`].
    #[deprecated(note = "retired by Anthropic; use OPUS_4_8")]
    pub const OPUS_3: &str = "claude-3-opus-20240229";

    /// Models that support thinking (adaptive and/or extended).
    pub fn supports_thinking(model: &str) -> bool {
        supports_adaptive_thinking(model) || matches!(model, OPUS_4_5 | SONNET_4_5 | OPUS_4_1)
    }

    /// Models that support adaptive thinking (`thinking: {type: "adaptive"}`).
    pub fn supports_adaptive_thinking(model: &str) -> bool {
        matches!(
            model,
            FABLE_5 | MYTHOS_5 | OPUS_4_8 | OPUS_4_7 | OPUS_4_6 | SONNET_4_6
        )
    }

    /// Models that support the `effort` output parameter.
    pub fn supports_effort(model: &str) -> bool {
        matches!(
            model,
            FABLE_5 | MYTHOS_5 | OPUS_4_8 | OPUS_4_7 | OPUS_4_6 | OPUS_4_5 | SONNET_4_6
        )
    }

    /// Models that support the `xhigh` effort level.
    pub fn supports_xhigh_effort(model: &str) -> bool {
        matches!(model, FABLE_5 | MYTHOS_5 | OPUS_4_8 | OPUS_4_7)
    }

    /// Check if a model supports a 1M-token context window.
    pub fn supports_1m_context(model: &str) -> bool {
        matches!(
            model,
            FABLE_5 | MYTHOS_5 | OPUS_4_8 | OPUS_4_7 | OPUS_4_6 | SONNET_4_6 | SONNET_4_5
        )
    }

    /// Get maximum extended-thinking tokens for a model.
    ///
    /// Returns `None` for adaptive-thinking models, where `budget_tokens` is
    /// removed/deprecated — use [`supports_adaptive_thinking`] and the `effort`
    /// parameter instead.
    pub fn max_thinking_tokens(model: &str) -> Option<u32> {
        if supports_adaptive_thinking(model) {
            return None;
        }
        None
    }

    /// Get all current (non-retired) models.
    pub fn all_models() -> &'static [&'static str] {
        &[
            FABLE_5, MYTHOS_5, OPUS_4_8, OPUS_4_7, OPUS_4_6, SONNET_4_6, HAIKU_4_5, OPUS_4_5,
            SONNET_4_5, OPUS_4_1,
        ]
    }

    /// Check if a model is a current (non-retired) catalog model.
    pub fn is_valid_model(model: &str) -> bool {
        !model.is_empty() && all_models().contains(&model)
    }
}

/// Configuration for the Anthropic API client
#[derive(Debug, Clone)]
pub struct Config {
    /// API key for authentication
    pub api_key: String,
    /// Admin API key for admin operations (optional)
    pub admin_key: Option<String>,
    /// Base URL for the API
    pub base_url: Url,
    /// Request timeout duration
    pub timeout: Duration,
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// User agent string
    pub user_agent: String,
    /// Default model to use
    pub default_model: String,
    /// Enable rate limiting
    pub enable_rate_limiting: bool,
    /// Rate limit: requests per second
    pub rate_limit_rps: u32,
}

impl Config {
    /// Create a new configuration with the given API key
    pub fn new(api_key: impl Into<String>) -> Result<Self> {
        let api_key = api_key.into();
        if api_key.is_empty() {
            return Err(AnthropicError::config("API key cannot be empty"));
        }

        Ok(Self {
            api_key,
            admin_key: None,
            base_url: Self::default_base_url()?,
            timeout: Duration::from_secs(60),
            max_retries: 3,
            user_agent: Self::default_user_agent(),
            default_model: DEFAULT_MODEL.to_string(),
            enable_rate_limiting: true,
            rate_limit_rps: 50,
        })
    }

    /// Create configuration from environment variables
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok(); // Ignore errors if .env file doesn't exist

        let api_key = std::env::var("ANTHROPIC_API_KEY").map_err(|_| {
            AnthropicError::config("ANTHROPIC_API_KEY environment variable not set")
        })?;

        let admin_key = std::env::var("ANTHROPIC_ADMIN_KEY").ok();

        let base_url = match std::env::var("ANTHROPIC_BASE_URL") {
            Ok(url_str) => Url::parse(&url_str)
                .map_err(|e| AnthropicError::config(format!("Invalid base URL: {}", e)))?,
            Err(_) => Self::default_base_url()?,
        };

        let timeout = std::env::var("ANTHROPIC_TIMEOUT")
            .ok()
            .and_then(|t| t.parse().ok())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(60));

        let max_retries = std::env::var("ANTHROPIC_MAX_RETRIES")
            .ok()
            .and_then(|r| r.parse().ok())
            .unwrap_or(3);

        let default_model =
            std::env::var("ANTHROPIC_DEFAULT_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());

        let enable_rate_limiting = std::env::var("ANTHROPIC_ENABLE_RATE_LIMITING")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let rate_limit_rps = std::env::var("ANTHROPIC_RATE_LIMIT_RPS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(50);

        Ok(Self {
            api_key,
            admin_key,
            base_url,
            timeout,
            max_retries,
            user_agent: Self::default_user_agent(),
            default_model,
            enable_rate_limiting,
            rate_limit_rps,
        })
    }

    /// Set the admin API key
    pub fn with_admin_key(mut self, admin_key: impl Into<String>) -> Self {
        self.admin_key = Some(admin_key.into());
        self
    }

    /// Set the base URL
    pub fn with_base_url(mut self, base_url: Url) -> Self {
        self.base_url = base_url;
        self
    }

    /// Set the request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set the user agent string
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = user_agent.into();
        self
    }

    /// Set the default model
    pub fn with_default_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Enable or disable rate limiting
    pub fn with_rate_limiting(mut self, enabled: bool) -> Self {
        self.enable_rate_limiting = enabled;
        self
    }

    /// Set the rate limit in requests per second
    pub fn with_rate_limit_rps(mut self, rps: u32) -> Self {
        self.rate_limit_rps = rps;
        self
    }

    /// Get the default base URL
    fn default_base_url() -> Result<Url> {
        Url::parse("https://api.anthropic.com")
            .map_err(|e| AnthropicError::config(format!("Invalid default base URL: {}", e)))
    }

    /// Get the default user agent string
    fn default_user_agent() -> String {
        format!(
            "{}/{} (Rust {})",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION"),
            std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unknown".to_string())
        )
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.api_key.is_empty() {
            return Err(AnthropicError::config("API key cannot be empty"));
        }

        if self.timeout.as_secs() == 0 {
            return Err(AnthropicError::config("Timeout must be greater than 0"));
        }

        if self.default_model.is_empty() {
            return Err(AnthropicError::config("Default model cannot be empty"));
        }

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_key: "sk-ant-api03-placeholder".to_string(), // Placeholder key for default config
            admin_key: None,
            base_url: Url::parse("https://api.anthropic.com").unwrap(),
            timeout: Duration::from_secs(60),
            max_retries: 3,
            user_agent: Self::default_user_agent(),
            default_model: DEFAULT_MODEL.to_string(),
            enable_rate_limiting: true,
            rate_limit_rps: 50,
        }
    }
}
