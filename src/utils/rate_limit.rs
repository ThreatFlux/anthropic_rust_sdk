//! Rate limiting utilities

use governor::{
    clock::{Clock, DefaultClock, QuantaClock},
    middleware::NoOpMiddleware,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as GovernorRateLimiter,
};
use nonzero_ext::nonzero;
use std::{
    num::NonZeroU32,
    sync::Arc,
    time::{Duration, Instant},
};

/// Rate limiter for controlling request frequency
#[derive(Clone)]
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<NotKeyed, InMemoryState, DefaultClock, NoOpMiddleware>>,
    config: RateLimitConfig,
    stats: Arc<std::sync::Mutex<RateLimitStats>>,
}

/// Rate limit configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per time window
    pub max_requests: NonZeroU32,
    /// Time window duration
    pub window: Duration,
    /// Burst allowance (requests that can be made immediately)
    pub burst: Option<NonZeroU32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: nonzero!(60u32), // 60 requests per minute
            window: Duration::from_secs(60),
            burst: Some(nonzero!(10u32)), // Allow 10 immediate requests
        }
    }
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            max_requests: NonZeroU32::new(max_requests).unwrap_or(nonzero!(1u32)),
            window,
            burst: None,
        }
    }

    /// Set burst allowance
    pub fn with_burst(mut self, burst: u32) -> Self {
        self.burst = NonZeroU32::new(burst);
        self
    }

    /// Create a quota from this configuration
    fn create_quota(&self) -> Quota {
        Quota::with_period(self.window / self.max_requests.get())
            .expect("Invalid quota configuration")
            .allow_burst(self.burst.unwrap_or(nonzero!(1u32)))
    }
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let quota = config.create_quota();
        let limiter = Arc::new(GovernorRateLimiter::direct(quota));

        Self {
            limiter,
            config,
            stats: Arc::new(std::sync::Mutex::new(RateLimitStats::default())),
        }
    }

    /// Create a rate limiter with default configuration
    pub fn with_defaults() -> Self {
        Self::new(RateLimitConfig::default())
    }

    /// Create a rate limiter for a specific rate (requests per second)
    pub fn per_second(requests: u32) -> Self {
        let config = RateLimitConfig::new(requests, Duration::from_secs(1));
        Self::new(config)
    }

    /// Create a rate limiter for a specific rate (requests per minute)
    pub fn per_minute(requests: u32) -> Self {
        let config = RateLimitConfig::new(requests, Duration::from_secs(60));
        Self::new(config)
    }

    /// Create a rate limiter for a specific rate (requests per hour)
    pub fn per_hour(requests: u32) -> Self {
        let config = RateLimitConfig::new(requests, Duration::from_secs(3600));
        Self::new(config)
    }

    /// Wait until a request can be made (respecting rate limits)
    pub async fn acquire(&self) -> Result<(), RateLimitError> {
        let start = Instant::now();
        self.limiter.until_ready().await;
        let wait_time = start.elapsed();

        // Update stats
        {
            let mut stats = self.stats.lock().unwrap();
            stats.record_wait(wait_time);
        }

        Ok(())
    }

    /// Try to acquire permission immediately (non-blocking)
    pub fn try_acquire(&self) -> Result<(), RateLimitError> {
        match self.limiter.check() {
            Ok(_) => Ok(()),
            Err(_) => Err(RateLimitError::Exceeded),
        }
    }

    /// Check how many requests can be made immediately
    pub fn available_permits(&self) -> u32 {
        // Use a more accurate implementation by checking if we can acquire permits
        match self.limiter.check() {
            Ok(_) => {
                // We can make at least one request
                // For simplicity, return 1 if available, 0 if not
                1
            }
            Err(_) => 0,
        }
    }

    /// Check if the rate limiter would allow a request
    pub fn would_allow(&self) -> bool {
        self.try_acquire().is_ok()
    }

    /// Get the time until the next request can be made
    pub fn time_until_ready(&self) -> Option<Duration> {
        match self.limiter.check() {
            Ok(_) => None, // Ready now
            Err(negative) => {
                let clock = QuantaClock::default();
                Some(negative.wait_time_from(clock.now()))
            }
        }
    }

    /// Reset the rate limiter statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock().unwrap();
        *stats = RateLimitStats::default();
    }

    /// Get configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Get current statistics
    pub fn stats(&self) -> RateLimitStats {
        self.stats.lock().unwrap().clone()
    }
}

/// Rate limit error types
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RateLimitError {
    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    Exceeded,
    /// Configuration error
    #[error("Rate limit configuration error: {0}")]
    Config(String),
}

/// Adaptive rate limiter that adjusts based on response headers
#[derive(Clone)]
pub struct AdaptiveRateLimiter {
    base_limiter: RateLimiter,
    current_limit: Arc<std::sync::RwLock<u32>>,
    last_reset: Arc<std::sync::RwLock<Instant>>,
    adaptation_factor: f32,
}

impl AdaptiveRateLimiter {
    /// Create a new adaptive rate limiter
    pub fn new(initial_config: RateLimitConfig) -> Self {
        let base_limiter = RateLimiter::new(initial_config.clone());

        Self {
            base_limiter,
            current_limit: Arc::new(std::sync::RwLock::new(initial_config.max_requests.get())),
            last_reset: Arc::new(std::sync::RwLock::new(Instant::now())),
            adaptation_factor: 0.8, // Conservative adaptation
        }
    }

    /// Update rate limit based on response headers
    pub fn update_from_headers(&self, rate_limit_info: &crate::utils::http::RateLimitInfo) {
        if let (Some(remaining), Some(limit)) = (rate_limit_info.remaining, rate_limit_info.limit) {
            {
                let mut current_limit = self.current_limit.write().unwrap();
                // If the API reports a different limit, adjust accordingly
                if limit != *current_limit {
                    *current_limit = limit;
                    tracing::info!("Adjusted rate limit to {}", limit);
                }
            }

            // Calculate usage ratio
            let usage_ratio = 1.0 - (remaining as f32 / limit as f32);

            // If we're getting close to the limit, log a warning
            if usage_ratio > self.adaptation_factor {
                tracing::warn!(
                    "Approaching rate limit: {} remaining out of {} ({}% used)",
                    remaining,
                    limit,
                    (usage_ratio * 100.0) as u32
                );
            }

            // Update reset time if provided
            if rate_limit_info.reset.is_some() {
                let mut last_reset = self.last_reset.write().unwrap();
                *last_reset = Instant::now();
            }
        }
    }

    /// Acquire with adaptive behavior
    pub async fn acquire(&self) -> Result<(), RateLimitError> {
        self.base_limiter.acquire().await
    }

    /// Try to acquire with adaptive behavior
    pub fn try_acquire(&self) -> Result<(), RateLimitError> {
        self.base_limiter.try_acquire()
    }

    /// Get the current effective rate limit
    pub fn current_limit(&self) -> u32 {
        *self.current_limit.read().unwrap()
    }

    /// Set adaptation factor (0.0 to 1.0)
    pub fn set_adaptation_factor(&mut self, factor: f32) {
        self.adaptation_factor = factor.clamp(0.0, 1.0);
    }

    /// Get current statistics
    pub fn stats(&self) -> RateLimitStats {
        self.base_limiter.stats()
    }
}

/// Rate limiting middleware for automatic request pacing
pub struct RateLimitMiddleware {
    limiter: RateLimiter,
    enabled: bool,
}

impl RateLimitMiddleware {
    /// Create new rate limiting middleware
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            limiter: RateLimiter::new(config),
            enabled: true,
        }
    }

    /// Enable or disable rate limiting
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Apply rate limiting to a request
    pub async fn apply(&self) -> Result<(), RateLimitError> {
        if self.enabled {
            self.limiter.acquire().await
        } else {
            Ok(())
        }
    }
}

/// Statistics for rate limiting
#[derive(Debug, Clone, Default)]
pub struct RateLimitStats {
    /// Total requests attempted
    pub total_requests: u64,
    /// Requests that were rate limited
    pub rate_limited_requests: u64,
    /// Total time spent waiting for rate limits
    pub total_wait_time: Duration,
    /// Maximum wait time for a single request
    pub max_wait_time: Duration,
    /// Average wait time per rate limited request
    pub avg_wait_time: Duration,
}

impl RateLimitStats {
    /// Update stats with a new wait time
    pub fn record_wait(&mut self, wait_time: Duration) {
        self.total_requests += 1;

        if wait_time > Duration::ZERO {
            self.rate_limited_requests += 1;
            self.total_wait_time += wait_time;
            self.max_wait_time = self.max_wait_time.max(wait_time);

            if self.rate_limited_requests > 0 {
                self.avg_wait_time = self.total_wait_time / self.rate_limited_requests as u32;
            }
        }
    }

    /// Get the rate limiting percentage
    pub fn rate_limit_percentage(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }

        self.rate_limited_requests as f64 / self.total_requests as f64 * 100.0
    }
}
