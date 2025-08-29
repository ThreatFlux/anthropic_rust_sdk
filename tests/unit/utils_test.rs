//! Unit tests for Utils modules
//!
//! Tests HTTP utils, retry logic, rate limiting, and utility functions.

use threatflux::{
    error::AnthropicError,
    utils::{http, retry, rate_limit},
    types::{RequestOptions, RequestPriority},
};
use std::time::Duration;
use tokio::time::Instant;
use pretty_assertions::assert_eq;

#[cfg(test)]
mod http_utils_tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};

    #[test]
    fn test_build_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", HeaderValue::from_static("test-key"));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        
        // Test that headers are built correctly
        assert_eq!(headers.get("x-api-key").unwrap(), "test-key");
        assert_eq!(headers.get(CONTENT_TYPE).unwrap(), "application/json");
    }

    #[test]
    fn test_user_agent_header() {
        let mut headers = HeaderMap::new();
        let user_agent = format!("threatflux/{}", env!("CARGO_PKG_VERSION"));
        headers.insert("user-agent", HeaderValue::from_str(&user_agent).unwrap());
        
        assert_eq!(headers.get("user-agent").unwrap(), user_agent);
    }

    #[test]
    fn test_request_id_generation() {
        use uuid::Uuid;
        
        let request_id = Uuid::new_v4().to_string();
        assert!(request_id.len() > 0);
        
        // Generate multiple IDs and ensure they're unique
        let id1 = Uuid::new_v4().to_string();
        let id2 = Uuid::new_v4().to_string();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_query_params_encoding() {
        use url::Url;
        
        let mut url = Url::parse("https://api.anthropic.com/v1/messages").unwrap();
        url.query_pairs_mut()
            .append_pair("limit", "20")
            .append_pair("after", "cursor_123")
            .append_pair("special", "value with spaces");
        
        let query = url.query().unwrap();
        assert!(query.contains("limit=20"));
        assert!(query.contains("after=cursor_123"));
        assert!(query.contains("special=value%20with%20spaces"));
    }

    #[test]
    fn test_content_type_detection() {
        assert_eq!(mime_guess::from_path("test.json").first_or_octet_stream(), "application/json");
        assert_eq!(mime_guess::from_path("test.txt").first_or_octet_stream(), "text/plain");
        assert_eq!(mime_guess::from_path("test.png").first_or_octet_stream(), "image/png");
        assert_eq!(mime_guess::from_path("test.jpeg").first_or_octet_stream(), "image/jpeg");
    }

    #[test]
    fn test_url_validation() {
        use url::Url;
        
        assert!(Url::parse("https://api.anthropic.com").is_ok());
        assert!(Url::parse("http://localhost:3000").is_ok());
        assert!(Url::parse("not-a-url").is_err());
        assert!(Url::parse("ftp://invalid.scheme").is_ok()); // URLs can have other schemes
    }
}

#[cfg(test)]
mod retry_logic_tests {
    use super::*;
    use backoff::{ExponentialBackoff, backoff::Backoff};

    #[test]
    fn test_exponential_backoff() {
        let mut backoff = ExponentialBackoff::default();
        
        let first = backoff.next_backoff();
        let second = backoff.next_backoff();
        
        assert!(first.is_some());
        assert!(second.is_some());
        assert!(second.unwrap() > first.unwrap());
    }

    #[test]
    fn test_should_retry_logic() {
        // Should retry server errors
        assert!(AnthropicError::api_error(500, "Server error".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(502, "Bad gateway".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(503, "Service unavailable".to_string(), None).is_retryable());
        
        // Should retry rate limits
        assert!(AnthropicError::rate_limit("Rate limit exceeded").is_retryable());
        
        // Should retry network errors
        assert!(AnthropicError::network("Connection failed").is_retryable());
        
        // Should retry timeouts
        assert!(AnthropicError::timeout(Duration::from_secs(30)).is_retryable());
        
        // Should not retry client errors
        assert!(!AnthropicError::api_error(400, "Bad request".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(401, "Unauthorized".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(404, "Not found".to_string(), None).is_retryable());
        
        // Should not retry config errors
        assert!(!AnthropicError::config("Invalid config").is_retryable());
    }

    #[tokio::test]
    async fn test_retry_with_backoff() {
        use std::sync::{Arc, Mutex};
        
        let attempt_count = Arc::new(Mutex::new(0));
        let attempt_count_clone = attempt_count.clone();
        
        let operation = || async {
            let mut count = attempt_count_clone.lock().unwrap();
            *count += 1;
            
            if *count < 3 {
                Err(AnthropicError::api_error(500, "Server error".to_string(), None))
            } else {
                Ok("Success".to_string())
            }
        };
        
        // This would be implemented in the actual retry logic
        let mut attempts = 0;
        let max_attempts = 3;
        let mut backoff = ExponentialBackoff::default();
        
        let result = loop {
            attempts += 1;
            
            match operation().await {
                Ok(result) => break Ok(result),
                Err(e) if e.is_retryable() && attempts < max_attempts => {
                    if let Some(delay) = backoff.next_backoff() {
                        tokio::time::sleep(delay).await;
                    }
                    continue;
                },
                Err(e) => break Err(e),
            }
        };
        
        assert!(result.is_ok());
        assert_eq!(*attempt_count.lock().unwrap(), 3);
    }

    #[test]
    fn test_backoff_configuration() {
        let backoff = ExponentialBackoff {
            initial_interval: Duration::from_millis(100),
            max_interval: Duration::from_secs(5),
            multiplier: 2.0,
            max_elapsed_time: Some(Duration::from_secs(30)),
            ..Default::default()
        };
        
        assert_eq!(backoff.initial_interval, Duration::from_millis(100));
        assert_eq!(backoff.max_interval, Duration::from_secs(5));
        assert_eq!(backoff.multiplier, 2.0);
        assert_eq!(backoff.max_elapsed_time, Some(Duration::from_secs(30)));
    }
}

#[cfg(test)]
mod rate_limiting_tests {
    use super::*;
    use governor::{Quota, RateLimiter};
    use nonzero_ext::nonzero;

    #[test]
    fn test_rate_limiter_creation() {
        let quota = Quota::per_second(nonzero!(10u32));
        let limiter = RateLimiter::direct(quota);
        
        // Test that limiter was created successfully
        assert!(limiter.check().is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiting_behavior() {
        let quota = Quota::per_second(nonzero!(2u32)); // 2 requests per second
        let limiter = RateLimiter::direct(quota);
        
        // First two requests should pass immediately
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        
        // Third request should be rate limited
        assert!(limiter.check().is_err());
        
        // Wait and try again
        tokio::time::sleep(Duration::from_millis(600)).await;
        assert!(limiter.check().is_ok());
    }

    #[test]
    fn test_burst_capacity() {
        let quota = Quota::per_second(nonzero!(10u32)).allow_burst(nonzero!(5u32));
        let limiter = RateLimiter::direct(quota);
        
        // Should allow burst of 5 requests
        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }
        
        // Sixth request should be rate limited
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_request_priority() {
        let high_priority = RequestPriority::High;
        let normal_priority = RequestPriority::Normal;
        let low_priority = RequestPriority::Low;
        
        // Test priority ordering
        assert!(high_priority > normal_priority);
        assert!(normal_priority > low_priority);
    }

    #[test]
    fn test_request_options() {
        let options = RequestOptions {
            priority: Some(RequestPriority::High),
            timeout: Some(Duration::from_secs(30)),
            retry_count: Some(5),
            metadata: Some(serde_json::json!({"test": true})),
        };
        
        assert_eq!(options.priority, Some(RequestPriority::High));
        assert_eq!(options.timeout, Some(Duration::from_secs(30)));
        assert_eq!(options.retry_count, Some(5));
        assert!(options.metadata.is_some());
    }
}

#[cfg(test)]
mod encoding_tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose};

    #[test]
    fn test_base64_encoding() {
        let data = b"Hello, World!";
        let encoded = general_purpose::STANDARD.encode(data);
        let decoded = general_purpose::STANDARD.decode(&encoded).unwrap();
        
        assert_eq!(decoded, data);
        assert_eq!(encoded, "SGVsbG8sIFdvcmxkIQ==");
    }

    #[test]
    fn test_image_base64_encoding() {
        // Simple 1x1 PNG image in base64
        let png_data = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==";
        let decoded = general_purpose::STANDARD.decode(png_data).unwrap();
        
        // Should be valid PNG header
        assert_eq!(&decoded[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
    }

    #[test]
    fn test_url_safe_base64() {
        let data = b"Hello, World! This is a test with special chars: +/=";
        let encoded = general_purpose::URL_SAFE.encode(data);
        let decoded = general_purpose::URL_SAFE.decode(&encoded).unwrap();
        
        assert_eq!(decoded, data);
        // URL-safe encoding should not contain +, /, or =
        assert!(!encoded.contains('+'));
        assert!(!encoded.contains('/'));
    }
}

#[cfg(test)]
mod json_handling_tests {
    use super::*;
    use serde_json::{json, Value};

    #[test]
    fn test_json_serialization() {
        let data = json!({
            "model": "claude-3-5-haiku-20241022",
            "max_tokens": 100,
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });
        
        let serialized = serde_json::to_string(&data).unwrap();
        let deserialized: Value = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(data, deserialized);
    }

    #[test]
    fn test_json_error_handling() {
        let invalid_json = r#"{"invalid": json}"#;
        let result = serde_json::from_str::<Value>(invalid_json);
        
        assert!(result.is_err());
    }

    #[test]
    fn test_json_pretty_printing() {
        let data = json!({
            "model": "claude-3-5-haiku-20241022",
            "messages": [{"role": "user", "content": "Hello"}]
        });
        
        let pretty = serde_json::to_string_pretty(&data).unwrap();
        
        assert!(pretty.contains('\n'));
        assert!(pretty.contains("  ")); // Should have indentation
    }

    #[test]
    fn test_json_value_manipulation() {
        let mut data = json!({"key": "value"});
        
        // Test modification
        data["new_key"] = json!("new_value");
        assert_eq!(data["new_key"], "new_value");
        
        // Test array operations
        data["array"] = json!([1, 2, 3]);
        if let Some(array) = data["array"].as_array_mut() {
            array.push(json!(4));
        }
        assert_eq!(data["array"], json!([1, 2, 3, 4]));
    }
}

#[cfg(test)]
mod time_handling_tests {
    use super::*;
    use chrono::{DateTime, Utc, Duration as ChronoDuration};

    #[test]
    fn test_timestamp_creation() {
        let now = Utc::now();
        let timestamp = now.timestamp();
        
        assert!(timestamp > 0);
        
        let recreated = DateTime::from_timestamp(timestamp, 0).unwrap();
        assert_eq!(recreated.timestamp(), timestamp);
    }

    #[test]
    fn test_duration_operations() {
        let duration1 = Duration::from_secs(30);
        let duration2 = Duration::from_millis(500);
        
        let total = duration1 + duration2;
        assert_eq!(total, Duration::from_millis(30500));
    }

    #[test]
    fn test_chrono_duration() {
        let duration = ChronoDuration::seconds(30);
        assert_eq!(duration.num_seconds(), 30);
        
        let future = Utc::now() + duration;
        let past = Utc::now() - duration;
        
        assert!(future > past);
    }

    #[test]
    fn test_timeout_calculation() {
        let start = Instant::now();
        let timeout = Duration::from_millis(100);
        let deadline = start + timeout;
        
        assert!(deadline > start);
        
        // Test if timeout has elapsed (should be false immediately)
        assert!(!start.elapsed() > timeout);
    }
}

#[cfg(test)]
mod enhanced_utils_tests {
    use super::*;
    use threatflux::utils::{
        http::{HttpClient, RateLimitInfo},
        retry::{RetryClient, RetryPolicy, RetryStats},
        rate_limit::{RateLimiter, RateLimitConfig, AdaptiveRateLimiter},
    };
    use threatflux::config::Config;
    use std::sync::Arc;
    use chrono::{Utc, DateTime};

    #[test]
    fn test_rate_limit_info_approaching_limit() {
        let rate_limit_info = RateLimitInfo {
            remaining: Some(20),
            limit: Some(100),
            reset: None,
            retry_after: None,
        };

        // Should not be approaching limit at 80% remaining
        assert!(!rate_limit_info.is_approaching_limit(0.8));

        let rate_limit_info = RateLimitInfo {
            remaining: Some(10),
            limit: Some(100),
            reset: None,
            retry_after: None,
        };

        // Should be approaching limit at 90% usage (10% remaining)
        assert!(rate_limit_info.is_approaching_limit(0.8));
    }

    #[test]
    fn test_rate_limit_info_recommended_delay() {
        // Test with explicit retry-after header
        let rate_limit_info = RateLimitInfo {
            remaining: Some(0),
            limit: Some(100),
            reset: None,
            retry_after: Some(Duration::from_secs(30)),
        };
        
        let delay = rate_limit_info.recommended_delay();
        assert_eq!(delay, Some(Duration::from_secs(30)));

        // Test with reset time in future
        let future_time = Utc::now() + chrono::Duration::seconds(60);
        let rate_limit_info = RateLimitInfo {
            remaining: Some(10),
            limit: Some(100),
            reset: Some(future_time),
            retry_after: None,
        };
        
        let delay = rate_limit_info.recommended_delay();
        assert!(delay.is_some());
        if let Some(d) = delay {
            assert!(d <= Duration::from_secs(60)); // Should be capped at 1 minute
        }
    }

    #[test]
    fn test_retry_policy_builder() {
        let policy = RetryPolicy::new()
            .with_max_retries(5)
            .with_initial_delay(Duration::from_millis(500))
            .with_max_delay(Duration::from_secs(30))
            .with_backoff_multiplier(2.5)
            .with_jitter(true);

        assert_eq!(policy.max_retries, 5);
        assert_eq!(policy.initial_delay, Duration::from_millis(500));
        assert_eq!(policy.max_delay, Duration::from_secs(30));
        assert_eq!(policy.backoff_multiplier, 2.5);
        assert!(policy.jitter);
    }

    #[test]
    fn test_retry_stats_calculations() {
        let mut stats = RetryStats::default();
        
        // Record some statistics
        stats.total_requests = 100;
        stats.successful_first_try = 80;
        stats.retried_requests = 15;
        stats.failed_requests = 5;
        stats.total_retry_attempts = 25;

        // Test success rate
        assert_eq!(stats.success_rate(), 0.95); // 95 successful out of 100

        // Test retry rate
        assert_eq!(stats.retry_rate(), 0.15); // 15 retried out of 100

        // Test average retries per request
        assert_eq!(stats.average_retries(), 0.25); // 25 retry attempts for 100 requests
    }

    #[test]
    fn test_rate_limit_config_creation() {
        let config = RateLimitConfig::new(60, Duration::from_secs(60))
            .with_burst(10);

        assert_eq!(config.max_requests.get(), 60);
        assert_eq!(config.window, Duration::from_secs(60));
        assert_eq!(config.burst.unwrap().get(), 10);
    }

    #[test]
    fn test_rate_limiter_convenience_constructors() {
        let per_second = RateLimiter::per_second(10);
        assert_eq!(per_second.config().window, Duration::from_secs(1));
        assert_eq!(per_second.config().max_requests.get(), 10);

        let per_minute = RateLimiter::per_minute(100);
        assert_eq!(per_minute.config().window, Duration::from_secs(60));
        assert_eq!(per_minute.config().max_requests.get(), 100);

        let per_hour = RateLimiter::per_hour(1000);
        assert_eq!(per_hour.config().window, Duration::from_secs(3600));
        assert_eq!(per_hour.config().max_requests.get(), 1000);
    }

    #[test]
    fn test_adaptive_rate_limiter() {
        let config = RateLimitConfig::default();
        let mut adaptive = AdaptiveRateLimiter::new(config.clone());
        
        assert_eq!(adaptive.current_limit(), config.max_requests.get());
        
        // Test adaptation factor setting
        adaptive.set_adaptation_factor(0.9);
        
        // Update from headers
        let rate_limit_info = RateLimitInfo {
            remaining: Some(50),
            limit: Some(200),
            reset: Some(Utc::now() + chrono::Duration::seconds(300)),
            retry_after: None,
        };
        
        adaptive.update_from_headers(&rate_limit_info);
        assert_eq!(adaptive.current_limit(), 200);
    }

    #[tokio::test]
    async fn test_rate_limiter_async_operations() {
        let limiter = RateLimiter::per_second(2);
        
        // Should be able to acquire permits
        assert!(limiter.acquire().await.is_ok());
        assert!(limiter.acquire().await.is_ok());
        
        // Get stats
        let stats = limiter.stats();
        assert_eq!(stats.total_requests, 2);
        
        // Reset stats
        limiter.reset_stats();
        let stats = limiter.stats();
        assert_eq!(stats.total_requests, 0);
    }

    #[test]
    fn test_http_status_code_utilities() {
        use threatflux::utils::http::HttpClient;
        
        // Test client error detection
        assert!(HttpClient::is_client_error(400));
        assert!(HttpClient::is_client_error(404));
        assert!(HttpClient::is_client_error(499));
        assert!(!HttpClient::is_client_error(500));
        
        // Test server error detection
        assert!(HttpClient::is_server_error(500));
        assert!(HttpClient::is_server_error(502));
        assert!(HttpClient::is_server_error(599));
        assert!(!HttpClient::is_server_error(400));
        
        // Test retry logic
        assert!(HttpClient::should_retry(429)); // Too Many Requests
        assert!(HttpClient::should_retry(500)); // Internal Server Error
        assert!(HttpClient::should_retry(502)); // Bad Gateway
        assert!(HttpClient::should_retry(503)); // Service Unavailable
        assert!(HttpClient::should_retry(504)); // Gateway Timeout
        assert!(!HttpClient::should_retry(400)); // Bad Request
        assert!(!HttpClient::should_retry(401)); // Unauthorized
        assert!(!HttpClient::should_retry(404)); // Not Found
    }

    #[test]
    fn test_rate_limit_header_parsing() {
        use threatflux::utils::http::HttpClient;
        use reqwest::header::{HeaderMap, HeaderValue};
        
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-remaining", HeaderValue::from_static("50"));
        headers.insert("x-ratelimit-limit", HeaderValue::from_static("100"));
        headers.insert("x-ratelimit-reset", HeaderValue::from_static("1640995200")); // Jan 1, 2022
        headers.insert("retry-after", HeaderValue::from_static("60"));
        
        let rate_limit_info = HttpClient::parse_rate_limit_headers(&headers);
        
        assert_eq!(rate_limit_info.remaining, Some(50));
        assert_eq!(rate_limit_info.limit, Some(100));
        assert!(rate_limit_info.reset.is_some());
        assert_eq!(rate_limit_info.retry_after, Some(Duration::from_secs(60)));
    }
}