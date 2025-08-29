//! Retry logic for handling transient failures

use crate::{
    config::Config,
    error::{AnthropicError, Result},
    types::HttpMethod,
    utils::http::{HttpClient, RateLimitInfo},
};
use backoff::{backoff::Backoff, ExponentialBackoff};
use reqwest::header::HeaderMap;
use serde::de::DeserializeOwned;
use std::{sync::Arc, time::Duration};
use url::Url;

/// Client wrapper that adds retry logic to HTTP requests
#[derive(Clone)]
pub struct RetryClient {
    http_client: HttpClient,
    config: Arc<Config>,
    stats: Arc<std::sync::Mutex<RetryStats>>,
}

impl RetryClient {
    /// Create a new retry client
    pub fn new(config: Arc<Config>) -> Self {
        let http_client = HttpClient::new(config.clone());

        Self {
            http_client,
            config,
            stats: Arc::new(std::sync::Mutex::new(RetryStats::default())),
        }
    }

    /// Make an HTTP request with retry logic
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
        let _start_time = std::time::Instant::now();
        let mut backoff = self.create_backoff();
        
        // Update total requests stat
        {
            let mut stats = self.stats.lock().unwrap();
            stats.total_requests += 1;
        }

        // Track attempt statistics

        for attempt in 0..=self.config.max_retries {
            match self
                .http_client
                .request(method, url, body.clone(), headers.clone(), timeout)
                .await
            {
                Ok(result) => {
                    if attempt == 0 {
                        let mut stats = self.stats.lock().unwrap();
                        stats.successful_first_try += 1;
                    } else {
                        let mut stats = self.stats.lock().unwrap();
                        stats.retried_requests += 1;
                        stats.total_retry_attempts += attempt as u64;
                    }
                    return Ok(result);
                }
                Err(error) => {
                    // Store error for potential return later

                    // Don't retry on final attempt
                    if attempt == self.config.max_retries {
                        let mut stats = self.stats.lock().unwrap();
                        stats.failed_requests += 1;
                        return Err(error);
                    }

                    // Check if we should retry this error
                    if !self.should_retry(&error) {
                        let mut stats = self.stats.lock().unwrap();
                        stats.failed_requests += 1;
                        return Err(error);
                    }

                    // Calculate delay
                    let delay = self.calculate_delay(&error, &mut backoff);

                    tracing::debug!(
                        "Request failed (attempt {}/{}), retrying in {:?}: {}",
                        attempt + 1,
                        self.config.max_retries + 1,
                        delay,
                        error
                    );

                    // Update retry delay stats
                    {
                        let mut stats = self.stats.lock().unwrap();
                        stats.total_retry_delay += delay;
                    }

                    tokio::time::sleep(delay).await;
                }
            }
        }

        // This should never be reached due to loop structure
        Err(AnthropicError::Unknown(anyhow::anyhow!(
            "All retry attempts failed"
        )))
    }

    /// Create exponential backoff configuration
    fn create_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: Duration::from_millis(1000),
            max_interval: Duration::from_secs(60),
            multiplier: 2.0,
            max_elapsed_time: Some(Duration::from_secs(300)), // 5 minutes total
            ..Default::default()
        }
    }

    /// Determine if an error should trigger a retry
    fn should_retry(&self, error: &AnthropicError) -> bool {
        match error {
            AnthropicError::Http(reqwest_error) => {
                // Retry on network errors (connection failed, timeout, etc.)
                reqwest_error.is_timeout()
                    || reqwest_error.is_connect()
                    || reqwest_error.is_request()
            }
            AnthropicError::Api { status, .. } => {
                // Retry on specific HTTP status codes
                HttpClient::should_retry(*status)
            }
            AnthropicError::RateLimit(_) => true,
            AnthropicError::Timeout(_) => true,
            _ => false,
        }
    }

    /// Calculate delay before next retry attempt
    fn calculate_delay(
        &self,
        error: &AnthropicError,
        backoff: &mut ExponentialBackoff,
    ) -> Duration {
        match error {
            AnthropicError::RateLimit(_) => {
                // For rate limit errors, use a longer delay
                Duration::from_secs(60)
            }
            AnthropicError::Api { status: 429, .. } => {
                // 429 Too Many Requests - use exponential backoff but start with longer delay
                backoff.next_backoff().unwrap_or(Duration::from_secs(30))
            }
            AnthropicError::Api { status, .. } if HttpClient::is_server_error(*status) => {
                // Server errors - use exponential backoff
                backoff.next_backoff().unwrap_or(Duration::from_secs(30))
            }
            _ => {
                // Default exponential backoff
                backoff.next_backoff().unwrap_or(Duration::from_secs(1))
            }
        }
    }

    /// Create a smart backoff that considers rate limit headers
    pub fn create_smart_backoff(&self, rate_limit_info: &RateLimitInfo) -> Duration {
        // If we have explicit retry-after header, use it
        if let Some(retry_after) = rate_limit_info.retry_after {
            return retry_after;
        }

        // If we have rate limit info, calculate intelligent delay
        if let Some(delay) = rate_limit_info.recommended_delay() {
            return delay;
        }

        // Fallback to default exponential backoff
        Duration::from_secs(1)
    }

    /// Get retry statistics
    pub fn stats(&self) -> RetryStats {
        self.stats.lock().unwrap().clone()
    }

    /// Reset retry statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = RetryStats::default();
    }
}

/// Retry policy configuration
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_retries: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum total time to spend retrying
    pub max_elapsed_time: Option<Duration>,
    /// Jitter to add to delays (prevents thundering herd)
    pub jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
            max_elapsed_time: Some(Duration::from_secs(300)),
            jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Create a new retry policy
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Set backoff multiplier
    pub fn with_backoff_multiplier(mut self, multiplier: f64) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Set maximum elapsed time
    pub fn with_max_elapsed_time(mut self, duration: Duration) -> Self {
        self.max_elapsed_time = Some(duration);
        self
    }

    /// Enable/disable jitter
    pub fn with_jitter(mut self, jitter: bool) -> Self {
        self.jitter = jitter;
        self
    }

    /// Create exponential backoff from this policy
    pub fn create_backoff(&self) -> ExponentialBackoff {
        ExponentialBackoff {
            initial_interval: self.initial_delay,
            max_interval: self.max_delay,
            multiplier: self.backoff_multiplier,
            max_elapsed_time: self.max_elapsed_time,
            ..Default::default()
        }
    }
}

/// Retry statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct RetryStats {
    /// Total number of requests attempted
    pub total_requests: u64,
    /// Number of requests that succeeded on first try
    pub successful_first_try: u64,
    /// Number of requests that required retries
    pub retried_requests: u64,
    /// Number of requests that failed after all retries
    pub failed_requests: u64,
    /// Total number of retry attempts made
    pub total_retry_attempts: u64,
    /// Total time spent waiting for retries
    pub total_retry_delay: Duration,
}

impl RetryStats {
    /// Get success rate (requests that eventually succeeded)
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }

        let successful = self.total_requests - self.failed_requests;
        successful as f64 / self.total_requests as f64
    }

    /// Get retry rate (requests that required retries)
    pub fn retry_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }

        self.retried_requests as f64 / self.total_requests as f64
    }

    /// Get average retries per request
    pub fn average_retries(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }

        self.total_retry_attempts as f64 / self.total_requests as f64
    }
}
