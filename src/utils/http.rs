//! HTTP client utilities

use crate::{
    config::Config,
    error::{AnthropicError, Result},
    types::{ApiErrorResponse, HttpMethod},
};
use reqwest::{header::HeaderMap, multipart::Form, Client, ClientBuilder};
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};
use url::Url;

/// HTTP client wrapper for making API requests
#[derive(Clone)]
pub struct HttpClient {
    client: Client,
    #[allow(dead_code)]
    config: Arc<Config>,
}

impl HttpClient {
    /// Create a new HTTP client
    pub fn new(config: Arc<Config>) -> Self {
        let mut builder = ClientBuilder::new()
            .timeout(config.timeout)
            .user_agent(&config.user_agent);

        // Configure TLS
        #[cfg(feature = "native-tls")]
        {
            builder = builder.use_native_tls();
        }

        #[cfg(feature = "rustls-tls")]
        {
            builder = builder.use_rustls_tls();
        }

        let client = builder.build().expect("Failed to create HTTP client");

        Self { client, config }
    }

    /// Helper method to build request with common configuration
    fn build_request_builder(
        &self,
        method: HttpMethod,
        url: &Url,
        headers: HeaderMap,
        timeout: Duration,
    ) -> reqwest::RequestBuilder {
        let request_builder = match method {
            HttpMethod::Get => self.client.get(url.clone()),
            HttpMethod::Post => self.client.post(url.clone()),
            HttpMethod::Put => self.client.put(url.clone()),
            HttpMethod::Patch => self.client.patch(url.clone()),
            HttpMethod::Delete => self.client.delete(url.clone()),
        };

        request_builder.headers(headers).timeout(timeout)
    }

    /// Make an HTTP request and parse the JSON response
    pub async fn request<T>(
        &self,
        method: HttpMethod,
        url: &Url,
        body: Option<serde_json::Value>,
        headers: HeaderMap,
        timeout: Duration,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request_builder = self.build_request_builder(method, url, headers, timeout);
        let request_builder = if let Some(body) = body {
            request_builder.json(&body)
        } else {
            request_builder
        };

        let response = request_builder
            .send()
            .await
            .map_err(AnthropicError::Http)?;
        self.handle_response(response).await
    }

    /// Make a streaming HTTP request
    pub async fn request_stream(
        &self,
        method: HttpMethod,
        url: &Url,
        body: Option<serde_json::Value>,
        headers: HeaderMap,
        timeout: Duration,
    ) -> Result<reqwest::Response> {
        let request_builder = self.build_request_builder(method, url, headers, timeout);
        let request_builder = if let Some(body) = body {
            request_builder.json(&body)
        } else {
            request_builder
        };

        request_builder
            .send()
            .await
            .map_err(AnthropicError::Http)
    }

    /// Make a multipart form request (for file uploads)
    pub async fn request_multipart<T>(
        &self,
        method: HttpMethod,
        url: &Url,
        form: Form,
        headers: HeaderMap,
        timeout: Duration,
    ) -> Result<T>
    where
        T: DeserializeOwned,
    {
        // Validate method for multipart requests
        if !matches!(
            method,
            HttpMethod::Post | HttpMethod::Put | HttpMethod::Patch
        ) {
            return Err(AnthropicError::invalid_input(
                "Multipart requests only support POST, PUT, and PATCH methods",
            ));
        }

        let request_builder = self.build_request_builder(method, url, headers, timeout);
        let request_builder = request_builder.multipart(form);

        let response = request_builder
            .send()
            .await
            .map_err(AnthropicError::Http)?;
        self.handle_response(response).await
    }

    /// Handle HTTP response and parse JSON or return errors
    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let status = response.status();

        if status.is_success() {
            let json = response.json().await?;
            Ok(json)
        } else {
            let status_code = status.as_u16();

            // Try to parse error response
            match response.text().await {
                Ok(error_text) => {
                    // Try to parse as API error response
                    if let Ok(api_error) = serde_json::from_str::<ApiErrorResponse>(&error_text) {
                        Err(AnthropicError::api_error(
                            status_code,
                            api_error.message,
                            Some(api_error.error_type),
                        ))
                    } else {
                        // Fallback to raw error text
                        Err(AnthropicError::api_error(status_code, error_text, None))
                    }
                }
                Err(_) => {
                    // Can't read response body
                    Err(AnthropicError::api_error(
                        status_code,
                        format!("HTTP {}", status_code),
                        None,
                    ))
                }
            }
        }
    }

    /// Check if a status code indicates a client error (4xx)
    pub fn is_client_error(status_code: u16) -> bool {
        (400..500).contains(&status_code)
    }

    /// Check if a status code indicates a server error (5xx)
    pub fn is_server_error(status_code: u16) -> bool {
        (500..600).contains(&status_code)
    }

    /// Check if a request should be retried based on status code
    pub fn should_retry(status_code: u16) -> bool {
        matches!(status_code, 429 | 500 | 502 | 503 | 504)
    }

    /// Get rate limit headers from response
    pub fn parse_rate_limit_headers(headers: &HeaderMap) -> RateLimitInfo {
        let remaining = headers
            .get("x-ratelimit-remaining")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        let limit = headers
            .get("x-ratelimit-limit")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        let reset = headers
            .get("x-ratelimit-reset")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok())
            .map(|timestamp| {
                chrono::DateTime::from_timestamp(timestamp, 0).unwrap_or_else(chrono::Utc::now)
            });

        let retry_after = headers
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .map(Duration::from_secs);

        RateLimitInfo {
            remaining,
            limit,
            reset,
            retry_after,
        }
    }
}

/// Rate limit information from response headers
#[derive(Debug, Clone)]
pub struct RateLimitInfo {
    /// Number of requests remaining in current window
    pub remaining: Option<u32>,
    /// Total requests allowed in current window
    pub limit: Option<u32>,
    /// When the rate limit window resets
    pub reset: Option<chrono::DateTime<chrono::Utc>>,
    /// How long to wait before retrying (from Retry-After header)
    pub retry_after: Option<Duration>,
}

impl RateLimitInfo {
    /// Check if we're approaching the rate limit
    pub fn is_approaching_limit(&self, threshold: f32) -> bool {
        match (self.remaining, self.limit) {
            (Some(remaining), Some(limit)) => {
                let usage_ratio = 1.0 - (remaining as f32 / limit as f32);
                usage_ratio >= threshold
            }
            _ => false,
        }
    }

    /// Get the recommended delay before next request
    pub fn recommended_delay(&self) -> Option<Duration> {
        if let Some(retry_after) = self.retry_after {
            return Some(retry_after);
        }

        // If we're close to the limit and have reset time, calculate delay
        if self.is_approaching_limit(0.8) {
            if let Some(reset_time) = self.reset {
                let now = chrono::Utc::now();
                if reset_time > now {
                    let delay = (reset_time - now)
                        .to_std()
                        .unwrap_or(Duration::from_secs(1));
                    return Some(delay.min(Duration::from_secs(60))); // Cap at 1 minute
                }
            }
        }

        None
    }
}
