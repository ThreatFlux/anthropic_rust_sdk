//! Error types for the Threatflux SDK

use std::time::Duration;
use thiserror::Error;

/// Result type alias for Threatflux operations
pub type Result<T> = std::result::Result<T, AnthropicError>;

/// Main error type for the Anthropic API SDK
#[derive(Error, Debug)]
pub enum AnthropicError {
    /// HTTP request error (deprecated - use Network instead)
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(String),

    /// API error response
    #[error("API error: {status} - {message}{}", error_type.as_ref().map(|t| format!(" ({})", t)).unwrap_or_default())]
    Api {
        status: u16,
        message: String,
        error_type: Option<String>,
    },

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Authentication error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Rate limit error
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Invalid input error
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// Stream error
    #[error("Stream error: {0}")]
    Stream(String),

    /// File operation error
    #[error("File error: {0}")]
    File(String),

    /// Network error (includes HTTP, connection, and timeout issues)
    #[error("Network error: {0}")]
    Network(String),

    /// Request timeout error
    #[error("Request timeout: {0:?}")]
    Timeout(Duration),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Base64 decode error
    #[error("Base64 decode error: {0}")]
    Base64Decode(#[from] base64::DecodeError),

    /// Generic error
    #[error("Unknown error: {0}")]
    Unknown(#[from] anyhow::Error),
}

impl AnthropicError {
    /// Create a new API error
    pub fn api_error(status: u16, message: String, error_type: Option<String>) -> Self {
        Self::Api {
            status,
            message,
            error_type,
        }
    }

    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }

    /// Create an authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }

    /// Create a rate limit error
    pub fn rate_limit(message: impl Into<String>) -> Self {
        Self::RateLimit(message.into())
    }

    /// Create an invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput(message.into())
    }

    /// Create a stream error
    pub fn stream(message: impl Into<String>) -> Self {
        Self::Stream(message.into())
    }

    /// Create a file error
    pub fn file_error(message: impl Into<String>) -> Self {
        Self::File(message.into())
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into())
    }

    /// Create a JSON error
    pub fn json(message: impl Into<String>) -> Self {
        Self::Json(message.into())
    }

    /// Create a timeout error
    pub fn timeout(duration: Duration) -> Self {
        Self::Timeout(duration)
    }

    /// Check if this is a retryable error
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect(),
            Self::Api { status, .. } => matches!(status, 429 | 500 | 502 | 503 | 504),
            Self::RateLimit(_) => true,
            Self::Network(_) => true,
            Self::Timeout(_) => true,
            _ => false,
        }
    }

    /// Check if this is a client error (4xx status code)
    pub fn is_client_error(&self) -> bool {
        match self {
            Self::Api { status, .. } => (400..500).contains(status),
            _ => false,
        }
    }

    /// Check if this is a server error (5xx status code)
    pub fn is_server_error(&self) -> bool {
        match self {
            Self::Api { status, .. } => (500..600).contains(status),
            _ => false,
        }
    }

    /// Get the HTTP status code if available
    pub fn status_code(&self) -> Option<u16> {
        match self {
            Self::Api { status, .. } => Some(*status),
            Self::Http(e) => e.status().map(|s| s.as_u16()),
            _ => None,
        }
    }

    /// Add context to an existing error
    pub fn with_context(self, context: impl Into<String>) -> Self {
        let context = context.into();
        match self {
            Self::Config(msg) => Self::Config(format!("{}: {}", context, msg)),
            Self::Auth(msg) => Self::Auth(format!("{}: {}", context, msg)),
            Self::RateLimit(msg) => Self::RateLimit(format!("{}: {}", context, msg)),
            Self::InvalidInput(msg) => Self::InvalidInput(format!("{}: {}", context, msg)),
            Self::Stream(msg) => Self::Stream(format!("{}: {}", context, msg)),
            Self::File(msg) => Self::File(format!("{}: {}", context, msg)),
            Self::Network(msg) => Self::Network(format!("{}: {}", context, msg)),
            Self::Json(msg) => Self::Json(format!("{}: {}", context, msg)),
            Self::Api {
                status,
                message,
                error_type,
            } => Self::Api {
                status,
                message: format!("{}: {}", context, message),
                error_type,
            },
            other => other, // For variants without string messages, return as-is
        }
    }
}

// Custom From implementations to handle automatic conversions
impl From<serde_json::Error> for AnthropicError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err.to_string())
    }
}

impl From<url::ParseError> for AnthropicError {
    fn from(err: url::ParseError) -> Self {
        Self::Config(format!("URL parsing error: {}", err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_api_error_creation() {
        let error =
            AnthropicError::api_error(404, "Not found".to_string(), Some("not_found".to_string()));

        if let AnthropicError::Api {
            status,
            message,
            error_type,
        } = error
        {
            assert_eq!(status, 404);
            assert_eq!(message, "Not found");
            assert_eq!(error_type, Some("not_found".to_string()));
        } else {
            panic!("Expected API error variant");
        }
    }

    #[test]
    fn test_api_error_creation_without_type() {
        let error = AnthropicError::api_error(500, "Server error".to_string(), None);

        if let AnthropicError::Api {
            status,
            message,
            error_type,
        } = error
        {
            assert_eq!(status, 500);
            assert_eq!(message, "Server error");
            assert_eq!(error_type, None);
        } else {
            panic!("Expected API error variant");
        }
    }

    #[test]
    fn test_config_error_creation() {
        let error = AnthropicError::config("Invalid configuration");
        assert!(matches!(error, AnthropicError::Config(_)));

        if let AnthropicError::Config(msg) = error {
            assert_eq!(msg, "Invalid configuration");
        }
    }

    #[test]
    fn test_auth_error_creation() {
        let error = AnthropicError::auth("Invalid API key");
        assert!(matches!(error, AnthropicError::Auth(_)));

        if let AnthropicError::Auth(msg) = error {
            assert_eq!(msg, "Invalid API key");
        }
    }

    #[test]
    fn test_rate_limit_error_creation() {
        let error = AnthropicError::rate_limit("Too many requests");
        assert!(matches!(error, AnthropicError::RateLimit(_)));

        if let AnthropicError::RateLimit(msg) = error {
            assert_eq!(msg, "Too many requests");
        }
    }

    #[test]
    fn test_invalid_input_error_creation() {
        let error = AnthropicError::invalid_input("Invalid input data");
        assert!(matches!(error, AnthropicError::InvalidInput(_)));

        if let AnthropicError::InvalidInput(msg) = error {
            assert_eq!(msg, "Invalid input data");
        }
    }

    #[test]
    fn test_stream_error_creation() {
        let error = AnthropicError::stream("Stream closed unexpectedly");
        assert!(matches!(error, AnthropicError::Stream(_)));

        if let AnthropicError::Stream(msg) = error {
            assert_eq!(msg, "Stream closed unexpectedly");
        }
    }

    #[test]
    fn test_file_error_creation() {
        let error = AnthropicError::file_error("File not found");
        assert!(matches!(error, AnthropicError::File(_)));

        if let AnthropicError::File(msg) = error {
            assert_eq!(msg, "File not found");
        }
    }

    #[test]
    fn test_network_error_creation() {
        let error = AnthropicError::network("Connection timeout");
        assert!(matches!(error, AnthropicError::Network(_)));

        if let AnthropicError::Network(msg) = error {
            assert_eq!(msg, "Connection timeout");
        }
    }

    #[test]
    fn test_json_error_creation() {
        let error = AnthropicError::json("Invalid JSON format");
        assert!(matches!(error, AnthropicError::Json(_)));

        if let AnthropicError::Json(msg) = error {
            assert_eq!(msg, "Invalid JSON format");
        }
    }

    #[test]
    fn test_timeout_error_creation() {
        let duration = Duration::from_secs(30);
        let error = AnthropicError::timeout(duration);
        assert!(matches!(error, AnthropicError::Timeout(_)));

        if let AnthropicError::Timeout(d) = error {
            assert_eq!(d, duration);
        }
    }

    #[test]
    fn test_is_retryable_api_errors() {
        // Should be retryable
        assert!(AnthropicError::api_error(429, "Rate limited".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(500, "Server error".to_string(), None).is_retryable());
        assert!(AnthropicError::api_error(502, "Bad gateway".to_string(), None).is_retryable());
        assert!(
            AnthropicError::api_error(503, "Service unavailable".to_string(), None).is_retryable()
        );
        assert!(AnthropicError::api_error(504, "Gateway timeout".to_string(), None).is_retryable());

        // Should not be retryable
        assert!(!AnthropicError::api_error(400, "Bad request".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(401, "Unauthorized".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(403, "Forbidden".to_string(), None).is_retryable());
        assert!(!AnthropicError::api_error(404, "Not found".to_string(), None).is_retryable());
        assert!(
            !AnthropicError::api_error(422, "Validation error".to_string(), None).is_retryable()
        );
    }

    #[test]
    fn test_is_retryable_other_errors() {
        // Should be retryable
        assert!(AnthropicError::rate_limit("Rate limit").is_retryable());
        assert!(AnthropicError::network("Network error").is_retryable());
        assert!(AnthropicError::timeout(Duration::from_secs(30)).is_retryable());

        // Should not be retryable
        assert!(!AnthropicError::config("Config error").is_retryable());
        assert!(!AnthropicError::auth("Auth error").is_retryable());
        assert!(!AnthropicError::invalid_input("Invalid input").is_retryable());
        assert!(!AnthropicError::stream("Stream error").is_retryable());
        assert!(!AnthropicError::file_error("File error").is_retryable());
        assert!(!AnthropicError::json("JSON error").is_retryable());
    }

    #[test]
    fn test_is_client_error() {
        // 4xx status codes should be client errors
        assert!(AnthropicError::api_error(400, "Bad request".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(401, "Unauthorized".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(403, "Forbidden".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(404, "Not found".to_string(), None).is_client_error());
        assert!(
            AnthropicError::api_error(422, "Validation error".to_string(), None).is_client_error()
        );
        assert!(AnthropicError::api_error(429, "Rate limited".to_string(), None).is_client_error());

        // 5xx status codes should not be client errors
        assert!(
            !AnthropicError::api_error(500, "Server error".to_string(), None).is_client_error()
        );
        assert!(!AnthropicError::api_error(502, "Bad gateway".to_string(), None).is_client_error());
        assert!(
            !AnthropicError::api_error(503, "Service unavailable".to_string(), None)
                .is_client_error()
        );

        // Non-API errors should not be client errors
        assert!(!AnthropicError::config("Config error").is_client_error());
        assert!(!AnthropicError::auth("Auth error").is_client_error());
    }

    #[test]
    fn test_is_server_error() {
        // 5xx status codes should be server errors
        assert!(AnthropicError::api_error(500, "Server error".to_string(), None).is_server_error());
        assert!(AnthropicError::api_error(502, "Bad gateway".to_string(), None).is_server_error());
        assert!(
            AnthropicError::api_error(503, "Service unavailable".to_string(), None)
                .is_server_error()
        );
        assert!(
            AnthropicError::api_error(504, "Gateway timeout".to_string(), None).is_server_error()
        );

        // 4xx status codes should not be server errors
        assert!(!AnthropicError::api_error(400, "Bad request".to_string(), None).is_server_error());
        assert!(
            !AnthropicError::api_error(401, "Unauthorized".to_string(), None).is_server_error()
        );
        assert!(!AnthropicError::api_error(404, "Not found".to_string(), None).is_server_error());
        assert!(
            !AnthropicError::api_error(429, "Rate limited".to_string(), None).is_server_error()
        );

        // Non-API errors should not be server errors
        assert!(!AnthropicError::config("Config error").is_server_error());
        assert!(!AnthropicError::auth("Auth error").is_server_error());
    }

    #[test]
    fn test_status_code() {
        // API errors should return status code
        assert_eq!(
            AnthropicError::api_error(404, "Not found".to_string(), None).status_code(),
            Some(404)
        );
        assert_eq!(
            AnthropicError::api_error(500, "Server error".to_string(), None).status_code(),
            Some(500)
        );

        // Non-API errors should return None
        assert_eq!(AnthropicError::config("Config error").status_code(), None);
        assert_eq!(AnthropicError::auth("Auth error").status_code(), None);
        assert_eq!(AnthropicError::rate_limit("Rate limit").status_code(), None);
        assert_eq!(AnthropicError::network("Network error").status_code(), None);
        assert_eq!(AnthropicError::json("JSON error").status_code(), None);
        assert_eq!(
            AnthropicError::timeout(Duration::from_secs(30)).status_code(),
            None
        );
    }

    #[test]
    fn test_with_context() {
        // Test with_context for string-based errors
        let config_error = AnthropicError::config("Invalid key").with_context("API initialization");
        if let AnthropicError::Config(msg) = config_error {
            assert_eq!(msg, "API initialization: Invalid key");
        } else {
            panic!("Expected Config error");
        }

        let auth_error = AnthropicError::auth("Token expired").with_context("Authentication");
        if let AnthropicError::Auth(msg) = auth_error {
            assert_eq!(msg, "Authentication: Token expired");
        } else {
            panic!("Expected Auth error");
        }

        let rate_limit_error =
            AnthropicError::rate_limit("Exceeded limit").with_context("Request processing");
        if let AnthropicError::RateLimit(msg) = rate_limit_error {
            assert_eq!(msg, "Request processing: Exceeded limit");
        } else {
            panic!("Expected RateLimit error");
        }

        let invalid_input_error =
            AnthropicError::invalid_input("Bad data").with_context("Validation");
        if let AnthropicError::InvalidInput(msg) = invalid_input_error {
            assert_eq!(msg, "Validation: Bad data");
        } else {
            panic!("Expected InvalidInput error");
        }

        let stream_error = AnthropicError::stream("Connection lost").with_context("Streaming");
        if let AnthropicError::Stream(msg) = stream_error {
            assert_eq!(msg, "Streaming: Connection lost");
        } else {
            panic!("Expected Stream error");
        }

        let file_error = AnthropicError::file_error("Read failed").with_context("File operation");
        if let AnthropicError::File(msg) = file_error {
            assert_eq!(msg, "File operation: Read failed");
        } else {
            panic!("Expected File error");
        }

        let network_error = AnthropicError::network("Timeout").with_context("HTTP request");
        if let AnthropicError::Network(msg) = network_error {
            assert_eq!(msg, "HTTP request: Timeout");
        } else {
            panic!("Expected Network error");
        }

        let json_error = AnthropicError::json("Parse failed").with_context("Response parsing");
        if let AnthropicError::Json(msg) = json_error {
            assert_eq!(msg, "Response parsing: Parse failed");
        } else {
            panic!("Expected Json error");
        }

        let api_error = AnthropicError::api_error(400, "Bad request".to_string(), None)
            .with_context("API call");
        if let AnthropicError::Api {
            status,
            message,
            error_type,
        } = api_error
        {
            assert_eq!(status, 400);
            assert_eq!(message, "API call: Bad request");
            assert_eq!(error_type, None);
        } else {
            panic!("Expected API error");
        }
    }

    #[test]
    fn test_with_context_non_string_errors() {
        // Test that non-string errors are returned as-is
        let timeout_error =
            AnthropicError::timeout(Duration::from_secs(30)).with_context("Request timeout");
        assert!(matches!(timeout_error, AnthropicError::Timeout(_)));

        if let AnthropicError::Timeout(duration) = timeout_error {
            assert_eq!(duration, Duration::from_secs(30));
        }
    }

    #[test]
    fn test_error_display_format() {
        // Test display formatting for different error types
        let config_error = AnthropicError::config("Invalid config");
        assert_eq!(
            format!("{}", config_error),
            "Configuration error: Invalid config"
        );

        let auth_error = AnthropicError::auth("Bad token");
        assert_eq!(format!("{}", auth_error), "Authentication error: Bad token");

        let rate_limit_error = AnthropicError::rate_limit("Too many requests");
        assert_eq!(
            format!("{}", rate_limit_error),
            "Rate limit exceeded: Too many requests"
        );

        let api_error = AnthropicError::api_error(
            404,
            "Not found".to_string(),
            Some("resource_not_found".to_string()),
        );
        let display = format!("{}", api_error);
        assert!(display.contains("404"));
        assert!(display.contains("Not found"));
        assert!(display.contains("resource_not_found"));

        let api_error_no_type = AnthropicError::api_error(500, "Server error".to_string(), None);
        let display = format!("{}", api_error_no_type);
        assert!(display.contains("500"));
        assert!(display.contains("Server error"));
        assert!(!display.contains("()"));

        let timeout_error = AnthropicError::timeout(Duration::from_secs(30));
        assert!(format!("{}", timeout_error).contains("30s"));
    }

    #[test]
    fn test_error_debug_format() {
        let error = AnthropicError::config("Test error");
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("Test error"));
    }

    #[test]
    fn test_from_serde_json_error() {
        let json_str = "invalid json";
        let serde_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let anthropic_error: AnthropicError = serde_error.into();

        assert!(matches!(anthropic_error, AnthropicError::Json(_)));
        if let AnthropicError::Json(msg) = anthropic_error {
            assert!(msg.contains("expected"));
        }
    }

    #[test]
    fn test_from_url_parse_error() {
        let invalid_url = "not-a-valid-url";
        let url_error = invalid_url.parse::<url::Url>().unwrap_err();
        let anthropic_error: AnthropicError = url_error.into();

        assert!(matches!(anthropic_error, AnthropicError::Config(_)));
        if let AnthropicError::Config(msg) = anthropic_error {
            assert!(msg.contains("URL parsing error"));
        }
    }

    #[test]
    fn test_from_std_io_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let anthropic_error: AnthropicError = io_error.into();

        assert!(matches!(anthropic_error, AnthropicError::Io(_)));
    }

    #[test]
    fn test_from_base64_decode_error() {
        use base64::Engine;
        let invalid_base64 = "invalid!base64!";
        let decode_error = base64::engine::general_purpose::STANDARD
            .decode(invalid_base64)
            .unwrap_err();
        let anthropic_error: AnthropicError = decode_error.into();

        assert!(matches!(anthropic_error, AnthropicError::Base64Decode(_)));
    }

    #[test]
    fn test_from_anyhow_error() {
        let anyhow_error = anyhow::anyhow!("Test error");
        let anthropic_error: AnthropicError = anyhow_error.into();

        assert!(matches!(anthropic_error, AnthropicError::Unknown(_)));
    }

    #[test]
    fn test_result_type() {
        fn success_function() -> Result<String> {
            Ok("success".to_string())
        }

        fn error_function() -> Result<String> {
            Err(AnthropicError::config("test error"))
        }

        assert!(success_function().is_ok());
        assert_eq!(success_function().unwrap(), "success");

        assert!(error_function().is_err());
        let err = error_function().unwrap_err();
        assert!(matches!(err, AnthropicError::Config(_)));
    }

    #[test]
    fn test_is_retryable_http_error_conditions() {
        // Test HTTP errors that should trigger is_retryable
        // Note: We can't easily create specific reqwest::Error instances in tests,
        // but we can verify the logic for other error types that should be retryable

        // These should definitely be retryable
        assert!(AnthropicError::timeout(Duration::from_secs(10)).is_retryable());
        assert!(AnthropicError::network("Connection failed").is_retryable());
        assert!(AnthropicError::rate_limit("Rate exceeded").is_retryable());

        // These should not be retryable
        assert!(!AnthropicError::config("Bad config").is_retryable());
        assert!(!AnthropicError::auth("Bad auth").is_retryable());
        assert!(!AnthropicError::invalid_input("Bad input").is_retryable());
        assert!(!AnthropicError::stream("Stream error").is_retryable());
        assert!(!AnthropicError::file_error("File error").is_retryable());
        assert!(!AnthropicError::json("JSON error").is_retryable());
    }

    #[test]
    fn test_status_code_http_error() {
        // Test that HTTP error status codes are extracted
        // This is harder to test directly since we'd need a real reqwest::Error
        // but we can test that non-HTTP errors return None
        assert_eq!(AnthropicError::config("test").status_code(), None);
        assert_eq!(AnthropicError::auth("test").status_code(), None);
        assert_eq!(AnthropicError::rate_limit("test").status_code(), None);
        assert_eq!(AnthropicError::network("test").status_code(), None);
        assert_eq!(
            AnthropicError::timeout(Duration::from_secs(1)).status_code(),
            None
        );
    }

    #[test]
    fn test_edge_case_status_codes() {
        // Test edge cases for status code ranges
        assert!(!AnthropicError::api_error(399, "Edge case".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(400, "Client start".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(499, "Client end".to_string(), None).is_client_error());
        assert!(AnthropicError::api_error(500, "Server start".to_string(), None).is_server_error());
        assert!(AnthropicError::api_error(599, "Server end".to_string(), None).is_server_error());
        assert!(!AnthropicError::api_error(600, "Edge case".to_string(), None).is_server_error());
    }

    #[test]
    fn test_comprehensive_constructor_coverage() {
        // Test all constructor methods with different input types

        // String literals
        let _ = AnthropicError::config("literal");
        let _ = AnthropicError::auth("literal");
        let _ = AnthropicError::rate_limit("literal");
        let _ = AnthropicError::invalid_input("literal");
        let _ = AnthropicError::stream("literal");
        let _ = AnthropicError::file_error("literal");
        let _ = AnthropicError::network("literal");
        let _ = AnthropicError::json("literal");

        // String objects
        let s = "string".to_string();
        let _ = AnthropicError::config(s.clone());
        let _ = AnthropicError::auth(s.clone());
        let _ = AnthropicError::rate_limit(s.clone());
        let _ = AnthropicError::invalid_input(s.clone());
        let _ = AnthropicError::stream(s.clone());
        let _ = AnthropicError::file_error(s.clone());
        let _ = AnthropicError::network(s.clone());
        let _ = AnthropicError::json(s.clone());

        // String references
        let s = "reference";
        let _ = AnthropicError::config(s);
        let _ = AnthropicError::auth(s);
        let _ = AnthropicError::rate_limit(s);
        let _ = AnthropicError::invalid_input(s);
        let _ = AnthropicError::stream(s);
        let _ = AnthropicError::file_error(s);
        let _ = AnthropicError::network(s);
        let _ = AnthropicError::json(s);
    }
}
