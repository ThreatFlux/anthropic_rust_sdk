//! HTTP client for the Anthropic API

/// Anthropic API version
pub const API_VERSION: &str = "2023-06-01";

/// Beta headers for various features
pub mod beta_headers {
    /// Files API beta header
    pub const FILES_API: &str = "files-api-2025-04-14";
    /// PDF support beta header
    pub const PDF_SUPPORT: &str = "pdfs-2024-09-25";
    /// Prompt caching beta header
    pub const PROMPT_CACHING: &str = "prompt-caching-2024-07-31";
    /// Prompt tools beta header
    pub const PROMPT_TOOLS: &str = "prompt-tools-2025-04-02";
    /// 1M context window for Sonnet 4
    pub const CONTEXT_1M: &str = "context-1m-2025-08-07";
    /// Extended thinking with tools beta header
    pub const EXTENDED_THINKING_TOOLS: &str = "extended-thinking-tools-2025-05-01";
}

use crate::{
    api::{
        admin::AdminApi, files::FilesApi, message_batches::MessageBatchesApi,
        messages::MessagesApi, models::ModelsApi,
    },
    config::Config,
    error::{AnthropicError, Result},
    types::{HttpMethod, RequestOptions},
    utils::{http::HttpClient, retry::RetryClient},
};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::de::DeserializeOwned;
use std::sync::Arc;
use url::Url;

/// Main client for the Anthropic API
#[derive(Clone)]
pub struct Client {
    config: Arc<Config>,
    http_client: HttpClient,
    retry_client: RetryClient,
}

impl Client {
    /// Helper function to create consistent config errors
    fn config_error(message: &str, error: impl std::fmt::Display) -> AnthropicError {
        AnthropicError::config(format!("{}: {}", message, error))
    }

    /// Create a new client with the given configuration, panicking on invalid config
    pub fn new(config: Config) -> Self {
        Self::try_new(config).expect("Invalid configuration")
    }

    /// Create a new client with the given configuration, returning an error on invalid config
    pub fn try_new(config: Config) -> Result<Self> {
        config.validate()?;

        let config = Arc::new(config);
        let http_client = HttpClient::new(config.clone());
        let retry_client = RetryClient::new(config.clone());

        Ok(Self {
            config,
            http_client,
            retry_client,
        })
    }

    /// Create a client from environment variables
    pub fn from_env() -> Result<Self> {
        let config = Config::from_env()?;
        Self::try_new(config)
    }

    /// Get the configuration
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Access the Messages API
    pub fn messages(&self) -> MessagesApi {
        MessagesApi::new(self.clone())
    }

    /// Access the Models API
    pub fn models(&self) -> ModelsApi {
        ModelsApi::new(self.clone())
    }

    /// Access the Message Batches API
    pub fn message_batches(&self) -> MessageBatchesApi {
        MessageBatchesApi::new(self.clone())
    }

    /// Access the Files API
    pub fn files(&self) -> FilesApi {
        FilesApi::new(self.clone())
    }

    /// Access the Admin API (requires admin key)
    pub fn admin(&self) -> Result<AdminApi> {
        if self.config.admin_key.is_none() {
            return Err(AnthropicError::auth(
                "Admin key is required for admin operations",
            ));
        }
        Ok(AdminApi::new(self.clone()))
    }

    /// Make a raw HTTP request
    pub async fn request<T>(
        &self,
        method: HttpMethod,
        path: &str,
        body: Option<serde_json::Value>,
        options: Option<RequestOptions>,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = self.build_url(path)?;
        let headers = self.build_headers(&options)?;
        let timeout = options
            .as_ref()
            .and_then(|o| o.timeout)
            .unwrap_or(self.config.timeout);

        if options.as_ref().map(|o| o.no_retry).unwrap_or(false) {
            self.http_client
                .request(method, &url, body, headers, timeout)
                .await
        } else {
            self.retry_client
                .request(method, &url, body, headers, timeout)
                .await
        }
    }

    /// Make a streaming request
    pub async fn request_stream(
        &self,
        method: HttpMethod,
        path: &str,
        body: Option<serde_json::Value>,
        options: Option<RequestOptions>,
    ) -> Result<reqwest::Response> {
        let url = self.build_url(path)?;
        let headers = self.build_headers(&options)?;
        let timeout = options
            .as_ref()
            .and_then(|o| o.timeout)
            .unwrap_or(self.config.timeout);

        self.http_client
            .request_stream(method, &url, body, headers, timeout)
            .await
    }

    /// Build the full URL for an API endpoint
    fn build_url(&self, path: &str) -> Result<Url> {
        let path = if path.starts_with('/') {
            path
        } else {
            &format!("/{}", path)
        };
        let url_str = format!("{}/v1{}", self.config.base_url, path);

        Url::parse(&url_str).map_err(|e| Self::config_error("Invalid URL", e))
    }

    /// Build HTTP headers for requests
    fn build_headers(&self, options: &Option<RequestOptions>) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();

        // Add authentication header
        let auth_value = format!("Bearer {}", self.config.api_key);
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&auth_value)
                .map_err(|e| Self::config_error("Invalid auth header", e))?,
        );

        // Add API version header
        headers.insert("anthropic-version", HeaderValue::from_static(API_VERSION));

        // Add user agent
        headers.insert(
            "User-Agent",
            HeaderValue::from_str(&self.config.user_agent)
                .map_err(|e| Self::config_error("Invalid user agent", e))?,
        );

        // Add content type for JSON requests
        headers.insert("Content-Type", HeaderValue::from_static("application/json"));

        // Add beta headers based on options
        if let Some(options) = options {
            // Collect all beta features that need to be enabled
            let mut beta_features = Vec::new();

            // Add beta headers for specific features
            if options.enable_files_api {
                beta_features.push(beta_headers::FILES_API);
            }
            if options.enable_pdf_support {
                beta_features.push(beta_headers::PDF_SUPPORT);
            }
            if options.enable_prompt_caching {
                beta_features.push(beta_headers::PROMPT_CACHING);
            }
            if options.enable_1m_context {
                beta_features.push(beta_headers::CONTEXT_1M);
            }
            if options.enable_extended_thinking_tools {
                beta_features.push(beta_headers::EXTENDED_THINKING_TOOLS);
            }

            // Add custom beta features from options
            beta_features.extend(options.beta_features.iter().map(|s| s.as_str()));

            // Set the combined beta header if any features are enabled
            if !beta_features.is_empty() {
                let beta_header_value = beta_features.join(",");
                headers.insert(
                    "anthropic-beta",
                    HeaderValue::from_str(&beta_header_value)
                        .map_err(|e| Self::config_error("Invalid beta header", e))?,
                );
            }

            // Add custom headers from options
            for (key, value) in &options.headers {
                let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                    .map_err(|e| Self::config_error("Invalid header name", e))?;
                headers.insert(
                    header_name,
                    HeaderValue::from_str(value)
                        .map_err(|e| Self::config_error("Invalid header value", e))?,
                );
            }
        }

        Ok(headers)
    }

    /// Build admin headers (includes admin key)
    pub(crate) fn build_admin_headers(
        &self,
        options: &Option<RequestOptions>,
    ) -> Result<HeaderMap> {
        let mut headers = self.build_headers(options)?;

        // Add admin auth header
        if let Some(admin_key) = &self.config.admin_key {
            let admin_auth_value = format!("Bearer {}", admin_key);
            headers.insert(
                "Authorization",
                HeaderValue::from_str(&admin_auth_value)
                    .map_err(|e| Self::config_error("Invalid admin auth header", e))?,
            );
        }

        Ok(headers)
    }
}
