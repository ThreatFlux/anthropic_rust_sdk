//! Unit tests for the Error module
//!
//! Tests error types, conversions, retry logic, and error handling scenarios.

use threatflux::error::{AnthropicError, Result};
use std::time::Duration;
use pretty_assertions::assert_eq;
use reqwest::StatusCode;

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn test_error_construction() {
        let config_error = AnthropicError::config("Invalid configuration");
        assert!(matches!(config_error, AnthropicError::Config(_)));
        
        if let AnthropicError::Config(msg) = config_error {
            assert_eq!(msg, "Invalid configuration");
        }
    }

    #[test]
    fn test_auth_error() {
        let auth_error = AnthropicError::auth("Invalid API key");
        assert!(matches!(auth_error, AnthropicError::Auth(_)));
        
        if let AnthropicError::Auth(msg) = auth_error {
            assert_eq!(msg, "Invalid API key");
        }
    }

    #[test]
    fn test_api_error() {
        let api_error = AnthropicError::api_error(404, "Not found".to_string(), Some("model_not_found".to_string()));
        
        if let AnthropicError::Api { status, message, error_type } = api_error {
            assert_eq!(status, 404);
            assert_eq!(message, "Not found");
            assert_eq!(error_type, Some("model_not_found".to_string()));
        } else {
            panic!("Expected API error variant");
        }
    }

    #[test]
    fn test_rate_limit_error() {
        let rate_limit = AnthropicError::rate_limit("Rate limit exceeded");
        assert!(matches!(rate_limit, AnthropicError::RateLimit(_)));
        
        if let AnthropicError::RateLimit(msg) = rate_limit {
            assert_eq!(msg, "Rate limit exceeded");
        }
    }

    #[test]
    fn test_network_error() {
        let network_error = AnthropicError::network("Connection timeout");
        assert!(matches!(network_error, AnthropicError::Network(_)));
        
        if let AnthropicError::Network(msg) = network_error {
            assert_eq!(msg, "Connection timeout");
        }
    }

    #[test]
    fn test_json_error() {
        let json_error = AnthropicError::json("Invalid JSON format");
        assert!(matches!(json_error, AnthropicError::Json(_)));
        
        if let AnthropicError::Json(msg) = json_error {
            assert_eq!(msg, "Invalid JSON format");
        }
    }

    #[test]
    fn test_timeout_error() {
        let timeout_error = AnthropicError::timeout(Duration::from_secs(30));
        assert!(matches!(timeout_error, AnthropicError::Timeout(_)));
        
        if let AnthropicError::Timeout(duration) = timeout_error {
            assert_eq!(duration, Duration::from_secs(30));
        }
    }

    #[test]
    fn test_retryable_errors() {
        // Should be retryable
        assert!(AnthropicError::api_error(500, "Server error".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(502, "Bad gateway".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(503, "Service unavailable".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(504, "Gateway timeout".to_string(), None).is_retryable());
        assert!(AnthropicError::rate_limit("Rate limit").is_retryable());
        assert!(AnthropicError::network("Network error").is_retryable());
        assert!(AnthropicError::timeout(Duration::from_secs(30)).is_retryable());
        
        // Should not be retryable
        assert!(!AnthropicError::api_error(400, "Bad request".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(401, "Unauthorized".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(403, "Forbidden".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(404, "Not found".to_string(), None).is_retryable());
        assert!(!AnthropicError::auth("Auth error").is_retryable());
        assert!(!AnthropicError::config("Config error").is_retryable());
        assert!(!AnthropicError::json("JSON error").is_retryable());
    }

    #[test]
    fn test_status_code_extraction() {
        assert_eq!(AnthropicError::api_error(404, "Not found".to_string(), None).status_code(), Some(404));
        assert_eq!(AnthropicError::config("Config error").status_code(), None);
        assert_eq!(AnthropicError::auth("Auth error").status_code(), None);
        assert_eq!(AnthropicError::network("Network error").status_code(), None);
    }

    #[test]
    fn test_error_display() {
        let config_error = AnthropicError::config("Test config error");
        assert_eq!(format!("{}", config_error), "Configuration error: Test config error");
        
        let api_error = AnthropicError::api_error(429, "Too many requests".to_string(), Some("rate_limit_error".to_string()));
        let display = format!("{}", api_error);
        assert!(display.contains("429"));
        assert!(display.contains("Too many requests"));
        assert!(display.contains("rate_limit_error"));
    }

    #[test]
    fn test_error_debug() {
        let error = AnthropicError::config("Debug test");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("Debug test"));
    }

    #[test]
    fn test_error_from_reqwest() {
        let reqwest_error = reqwest::Error::from(reqwest::ErrorKind::Request);
        let anthropic_error: AnthropicError = reqwest_error.into();
        assert!(matches!(anthropic_error, AnthropicError::Network(_)));
    }

    #[test]
    fn test_error_from_serde_json() {
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let anthropic_error: AnthropicError = json_error.into();
        assert!(matches!(anthropic_error, AnthropicError::Json(_)));
    }

    #[test]
    fn test_error_from_url_parse() {
        let url_error = "not a url".parse::<url::Url>().unwrap_err();
        let anthropic_error: AnthropicError = url_error.into();
        assert!(matches!(anthropic_error, AnthropicError::Config(_)));
    }

    #[test]
    fn test_error_chain() {
        let inner_error = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "Connection refused");
        let network_error = AnthropicError::Network(format!("Network error: {}", inner_error));
        
        assert!(format!("{}", network_error).contains("Connection refused"));
    }

    #[test]
    fn test_result_type() {
        fn returns_ok() -> Result<String> {
            Ok("success".to_string())
        }
        
        fn returns_error() -> Result<String> {
            Err(AnthropicError::config("test error"))
        }
        
        assert!(returns_ok().is_ok());
        assert!(returns_error().is_err());
        
        assert_eq!(returns_ok().unwrap(), "success");
        assert!(matches!(returns_error().unwrap_err(), AnthropicError::Config(_)));
    }

    #[test]
    fn test_error_specific_status_codes() {
        // Test specific HTTP status codes that have special meaning
        assert!(AnthropicError::api_error(429, "Rate limited".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(422, "Validation error".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(413, "Payload too large".to_string(), None).is_retryable());
    }

    #[test]
    fn test_error_type_parsing() {
        let error_with_type = AnthropicError::api_error(
            400, 
            "Invalid request".to_string(), 
            Some("invalid_request_error".to_string())
        );
        
        if let AnthropicError::Api { error_type, .. } = error_with_type {
            assert_eq!(error_type, Some("invalid_request_error".to_string()));
        } else {
            panic!("Expected API error");
        }
        
        let error_without_type = AnthropicError::api_error(500, "Server error".to_string(), None);
        if let AnthropicError::Api { error_type, .. } = error_without_type {
            assert_eq!(error_type, None);
        } else {
            panic!("Expected API error");
        }
    }

    #[test]
    fn test_is_client_error() {
        assert!(AnthropicError::api_error(400, "Bad request".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(404, "Not found".to_string(), None).is_client_error());
        assert!(!AnthropicError::api_error(500, "Server error".to_string(), None).is_client_error());
        assert!(!AnthropicError::config("Config error").is_client_error());
    }

    #[test]
    fn test_is_server_error() {
        assert!(AnthropicError::api_error(500, "Server error".to_string(), None).is_server_error());
        assert!(AnthropicError::api_error(502, "Bad gateway".to_string(), None).is_server_error());
        assert!(!AnthropicError::api_error(400, "Bad request".to_string(), None).is_server_error());
        assert!(!AnthropicError::config("Config error").is_server_error());
    }

    #[test]
    fn test_error_context() {
        let base_error = AnthropicError::config("Base error");
        let contextual_error = base_error.with_context("Additional context");
        
        let error_msg = format!("{}", contextual_error);
        assert!(error_msg.contains("Base error"));
        assert!(error_msg.contains("Additional context"));
    }
}