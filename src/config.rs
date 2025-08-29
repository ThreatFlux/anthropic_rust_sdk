//! Configuration for the Anthropic API client

use crate::error::{AnthropicError, Result};
use std::time::Duration;
use url::Url;

/// Default model to use when none is specified
pub const DEFAULT_MODEL: &str = "claude-3-5-haiku-20241022";

/// Available Claude models
pub mod models {
    /// Claude Opus 4.1 - Most powerful, best for complex tasks
    pub const OPUS_4_1: &str = "claude-opus-4-1-20250805";
    /// Claude Opus 4 - Previous Opus version
    pub const OPUS_4: &str = "claude-opus-4-20250514";
    /// Claude Sonnet 4 - Balanced performance, 1M context available
    pub const SONNET_4: &str = "claude-sonnet-4-20250514";
    /// Claude 3.7 Sonnet - Previous Sonnet version
    pub const SONNET_3_7: &str = "claude-3-7-sonnet-20250219";
    /// Claude 3.5 Haiku - Fastest, most cost-effective
    pub const HAIKU_3_5: &str = "claude-3-5-haiku-20241022";
    /// Claude 3.5 Sonnet - Balanced performance
    pub const SONNET_3_5: &str = "claude-3-5-sonnet-20241022";
    /// Claude 3 Opus - Maximum intelligence
    pub const OPUS_3: &str = "claude-3-opus-20240229";

    /// Check if a model supports extended thinking
    pub fn supports_thinking(model: &str) -> bool {
        if model.is_empty() {
            return false;
        }
        matches!(model, OPUS_4_1 | OPUS_4 | SONNET_4)
    }

    /// Check if a model supports 1M context window
    pub fn supports_1m_context(model: &str) -> bool {
        if model.is_empty() {
            return false;
        }
        model == SONNET_4
    }

    /// Get maximum thinking tokens for a model
    pub fn max_thinking_tokens(model: &str) -> Option<u32> {
        if model.is_empty() {
            return None;
        }
        match model {
            OPUS_4_1 | OPUS_4 => Some(64000),
            SONNET_4 => Some(32000),
            _ => None,
        }
    }

    /// Get all available models
    pub fn all_models() -> &'static [&'static str] {
        &[
            OPUS_4_1, OPUS_4, SONNET_4, SONNET_3_7, HAIKU_3_5, SONNET_3_5, OPUS_3,
        ]
    }

    /// Check if a model is valid/supported
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
        dotenv::dotenv().ok(); // Ignore errors if .env file doesn't exist

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
            "threatflux/{} (Rust {})",
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
