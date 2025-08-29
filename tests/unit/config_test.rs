//! Unit tests for the Config module
//!
//! Tests configuration loading, environment variables, validation, and builder patterns.

use threatflux::{Config, error::AnthropicError, config::models};
use std::time::Duration;
use pretty_assertions::assert_eq;

#[cfg(test)]
mod config_tests {
    use super::*;

    #[test]
    fn test_config_new_valid_key() {
        let config = Config::new("valid-api-key").unwrap();
        assert_eq!(config.api_key, "valid-api-key");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.default_model, "claude-3-5-haiku-20241022");
        assert_eq!(config.base_url.as_str(), "https://api.anthropic.com");
    }

    #[test]
    fn test_config_new_empty_key() {
        let result = Config::new("");
        assert!(matches!(result, Err(AnthropicError::Config(_))));
        
        if let Err(AnthropicError::Config(msg)) = result {
            assert!(msg.contains("API key cannot be empty"));
        }
    }

    #[test]
    fn test_config_new_whitespace_key() {
        let result = Config::new("   ");
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = Config::new("test-key")
            .unwrap()
            .with_timeout(Duration::from_secs(30))
            .with_max_retries(5)
            .with_default_model("claude-3-sonnet-20240229")
            .with_base_url("https://custom.api.com".parse().unwrap());

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.default_model, "claude-3-sonnet-20240229");
        assert_eq!(config.base_url.as_str(), "https://custom.api.com");
    }

    #[test]
    fn test_config_from_env_complete() {
        // Setup environment variables
        std::env::set_var("ANTHROPIC_API_KEY", "env-api-key");
        std::env::set_var("ANTHROPIC_BASE_URL", "https://env.api.com");
        std::env::set_var("ANTHROPIC_MAX_RETRIES", "7");
        std::env::set_var("ANTHROPIC_TIMEOUT", "90");
        std::env::set_var("ANTHROPIC_DEFAULT_MODEL", "claude-3-opus-20240229");

        let config = Config::from_env().unwrap();
        
        assert_eq!(config.api_key, "env-api-key");
        assert_eq!(config.base_url.as_str(), "https://env.api.com");
        assert_eq!(config.max_retries, 7);
        assert_eq!(config.timeout, Duration::from_secs(90));
        assert_eq!(config.default_model, "claude-3-opus-20240229");

        // Cleanup
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_BASE_URL");
        std::env::remove_var("ANTHROPIC_MAX_RETRIES");
        std::env::remove_var("ANTHROPIC_TIMEOUT");
        std::env::remove_var("ANTHROPIC_DEFAULT_MODEL");
    }

    #[test]
    fn test_config_from_env_missing_api_key() {
        std::env::remove_var("ANTHROPIC_API_KEY");
        
        let result = Config::from_env();
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_config_from_env_invalid_numbers() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        std::env::set_var("ANTHROPIC_MAX_RETRIES", "not-a-number");
        std::env::set_var("ANTHROPIC_TIMEOUT", "invalid");
        
        let config = Config::from_env().unwrap(); // Should use defaults for invalid values
        assert_eq!(config.max_retries, 3); // default value
        assert_eq!(config.timeout, Duration::from_secs(60)); // default value
        
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_MAX_RETRIES");
        std::env::remove_var("ANTHROPIC_TIMEOUT");
    }

    #[test]
    fn test_config_from_env_invalid_url() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        std::env::set_var("ANTHROPIC_BASE_URL", "not-a-valid-url");
        
        let result = Config::from_env();
        assert!(matches!(result, Err(AnthropicError::Config(_))));
        
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_BASE_URL");
    }

    #[test]
    fn test_config_validation_valid() {
        let config = Config::new("valid-key").unwrap();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_empty_key() {
        let mut config = Config::new("valid-key").unwrap();
        config.api_key = String::new();
        
        let result = config.validate();
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_config_validation_zero_timeout() {
        let mut config = Config::new("valid-key").unwrap();
        config.timeout = Duration::from_secs(0);
        
        let result = config.validate();
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_config_validation_valid_retries() {
        let mut config = Config::new("valid-key").unwrap();
        config.max_retries = 10; // Should be valid
        
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_config_validation_empty_model() {
        let mut config = Config::new("valid-key").unwrap();
        config.default_model = String::new();
        
        let result = config.validate();
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_config_clone() {
        let config1 = Config::new("test-key").unwrap();
        let config2 = config1.clone();
        
        assert_eq!(config1.api_key, config2.api_key);
        assert_eq!(config1.timeout, config2.timeout);
        assert_eq!(config1.max_retries, config2.max_retries);
        assert_eq!(config1.base_url, config2.base_url);
    }

    #[test]
    fn test_config_debug_format() {
        let config = Config::new("secret-api-key").unwrap();
        let debug_str = format!("{:?}", config);
        
        // Debug should contain the API key (no redaction in this implementation)
        assert!(debug_str.contains("secret-api-key"));
    }

    #[test]
    fn test_config_with_rate_limiting() {
        let config = Config::new("test-key")
            .unwrap()
            .with_rate_limiting(false)
            .with_rate_limit_rps(10);
        
        assert_eq!(config.rate_limit_rps, 10);
        assert!(!config.enable_rate_limiting);
    }

    #[test]
    fn test_config_edge_cases() {
        // Very long API key
        let long_key = "a".repeat(1000);
        let config = Config::new(&long_key).unwrap();
        assert_eq!(config.api_key, long_key);
        
        // Very high timeout
        let config = Config::new("test-key")
            .unwrap()
            .with_timeout(Duration::from_secs(3600)); // 1 hour
        assert_eq!(config.timeout, Duration::from_secs(3600));
        
        // Maximum reasonable retries
        let config = Config::new("test-key")
            .unwrap()
            .with_max_retries(10);
        assert_eq!(config.max_retries, 10);
    }

    #[test]
    fn test_config_partial_env() {
        // Only set API key, others should use defaults
        std::env::set_var("ANTHROPIC_API_KEY", "partial-env-key");
        std::env::remove_var("ANTHROPIC_BASE_URL");
        std::env::remove_var("ANTHROPIC_MAX_RETRIES");
        std::env::remove_var("ANTHROPIC_TIMEOUT");
        
        let config = Config::from_env().unwrap();
        
        assert_eq!(config.api_key, "partial-env-key");
        assert_eq!(config.base_url.as_str(), "https://api.anthropic.com"); // default
        assert_eq!(config.max_retries, 3); // default
        assert_eq!(config.timeout, Duration::from_secs(60)); // default
        
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(!config.api_key.is_empty());
        assert_eq!(config.base_url.as_str(), "https://api.anthropic.com");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.default_model, "claude-3-5-haiku-20241022");
        assert!(config.enable_rate_limiting);
        assert_eq!(config.rate_limit_rps, 50);
    }

    #[test]
    fn test_config_with_admin_key() {
        let config = Config::new("test-key")
            .unwrap()
            .with_admin_key("admin-key");
        assert_eq!(config.admin_key, Some("admin-key".to_string()));
    }

    #[test]
    fn test_config_with_user_agent() {
        let config = Config::new("test-key")
            .unwrap()
            .with_user_agent("custom-agent/1.0");
        assert_eq!(config.user_agent, "custom-agent/1.0");
    }
}

#[cfg(test)]
mod model_tests {
    use super::*;

    #[test]
    fn test_supports_thinking() {
        assert!(models::supports_thinking(models::OPUS_4_1));
        assert!(models::supports_thinking(models::OPUS_4));
        assert!(models::supports_thinking(models::SONNET_4));
        assert!(!models::supports_thinking(models::HAIKU_3_5));
        assert!(!models::supports_thinking(models::SONNET_3_5));
        assert!(!models::supports_thinking("unknown-model"));
        assert!(!models::supports_thinking(""));
    }

    #[test]
    fn test_supports_1m_context() {
        assert!(models::supports_1m_context(models::SONNET_4));
        assert!(!models::supports_1m_context(models::OPUS_4_1));
        assert!(!models::supports_1m_context(models::HAIKU_3_5));
        assert!(!models::supports_1m_context("unknown-model"));
        assert!(!models::supports_1m_context(""));
    }

    #[test]
    fn test_max_thinking_tokens() {
        assert_eq!(models::max_thinking_tokens(models::OPUS_4_1), Some(64000));
        assert_eq!(models::max_thinking_tokens(models::OPUS_4), Some(64000));
        assert_eq!(models::max_thinking_tokens(models::SONNET_4), Some(32000));
        assert_eq!(models::max_thinking_tokens(models::HAIKU_3_5), None);
        assert_eq!(models::max_thinking_tokens("unknown-model"), None);
        assert_eq!(models::max_thinking_tokens(""), None);
    }

    #[test]
    fn test_all_models() {
        let all_models = models::all_models();
        assert!(all_models.contains(&models::OPUS_4_1));
        assert!(all_models.contains(&models::OPUS_4));
        assert!(all_models.contains(&models::SONNET_4));
        assert!(all_models.contains(&models::SONNET_3_7));
        assert!(all_models.contains(&models::HAIKU_3_5));
        assert!(all_models.contains(&models::SONNET_3_5));
        assert!(all_models.contains(&models::OPUS_3));
        assert_eq!(all_models.len(), 7);
    }

    #[test]
    fn test_is_valid_model() {
        assert!(models::is_valid_model(models::OPUS_4_1));
        assert!(models::is_valid_model(models::SONNET_4));
        assert!(models::is_valid_model(models::HAIKU_3_5));
        assert!(!models::is_valid_model("unknown-model"));
        assert!(!models::is_valid_model(""));
        assert!(!models::is_valid_model("   "));
    }

    #[test]
    fn test_model_constants() {
        // Verify all model constants are properly formatted
        assert!(models::OPUS_4_1.starts_with("claude-opus-4-1"));
        assert!(models::OPUS_4.starts_with("claude-opus-4"));
        assert!(models::SONNET_4.starts_with("claude-sonnet-4"));
        assert!(models::SONNET_3_7.starts_with("claude-3-7-sonnet"));
        assert!(models::HAIKU_3_5.starts_with("claude-3-5-haiku"));
        assert!(models::SONNET_3_5.starts_with("claude-3-5-sonnet"));
        assert!(models::OPUS_3.starts_with("claude-3-opus"));
        
        // Verify they all have date suffixes
        for model in models::all_models() {
            assert!(model.matches('-').count() >= 3, "Model {} should have date suffix", model);
        }
    }
}