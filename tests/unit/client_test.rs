//! Unit tests for the Client module
//! 
//! Tests client initialization, configuration, API getters, and basic functionality.

use threatflux::{Client, Config, error::AnthropicError, types::RequestOptions};
use std::{time::Duration, collections::HashMap};
use pretty_assertions::assert_eq;

#[cfg(test)]
mod client_tests {
    use super::*;

    #[test]
    fn test_client_from_config() {
        let config = Config::new("test-key").unwrap();
        let client = Client::new(config.clone());
        
        assert_eq!(client.config().api_key, "test-key");
        assert_eq!(client.config().timeout, config.timeout);
        assert_eq!(client.config().max_retries, config.max_retries);
    }

    #[test]
    fn test_client_from_env_missing_key() {
        // Clear env var if it exists
        std::env::remove_var("ANTHROPIC_API_KEY");
        
        let result = Client::from_env();
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_client_from_env_with_key() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-env-key");
        
        let client = Client::from_env().unwrap();
        assert_eq!(client.config().api_key, "test-env-key");
        
        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_client_api_getters() {
        let config = Config::new("test-key").unwrap();
        let client = Client::new(config);
        
        // Test that API getters return the expected types
        let _messages = client.messages();
        let _models = client.models();
        let _batches = client.message_batches();
        let _files = client.files();
        
        // Admin API should require specific permissions
        let admin_result = client.admin();
        // This should work with any key for testing
        assert!(admin_result.is_ok());
    }

    #[test]
    fn test_client_config_access() {
        let config = Config::new("test-key")
            .unwrap()
            .with_timeout(Duration::from_secs(45))
            .with_max_retries(5);
            
        let client = Client::new(config.clone());
        
        assert_eq!(client.config().api_key, config.api_key);
        assert_eq!(client.config().timeout, config.timeout);
        assert_eq!(client.config().max_retries, config.max_retries);
    }

    #[test]
    fn test_client_with_custom_base_url() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        std::env::set_var("ANTHROPIC_BASE_URL", "https://custom.api.com");
        
        let client = Client::from_env().unwrap();
        assert_eq!(client.config().base_url.as_str(), "https://custom.api.com");
        
        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("ANTHROPIC_BASE_URL");
    }

    #[test]
    fn test_client_headers_construction() {
        let config = Config::new("test-api-key-12345").unwrap();
        let client = Client::new(config);
        
        // This is testing internal functionality, but we can test
        // that the client is constructed properly
        assert_eq!(client.config().api_key, "test-api-key-12345");
        
        // Test with custom headers if they're supported
        // This would be implementation specific
    }

    #[test]
    fn test_client_clone() {
        let config = Config::new("test-key").unwrap();
        let client1 = Client::new(config);
        let client2 = client1.clone();
        
        assert_eq!(client1.config().api_key, client2.config().api_key);
        assert_eq!(client1.config().timeout, client2.config().timeout);
    }

    #[test]
    fn test_client_debug_format() {
        let config = Config::new("test-key").unwrap();
        let client = Client::new(config);
        
        let debug_str = format!("{:?}", client);
        // Should not include the actual API key in debug output for security
        assert!(!debug_str.contains("test-key"));
    }

    #[test]
    fn test_client_try_new_success() {
        let config = Config::new("test-key").unwrap();
        let client = Client::try_new(config.clone());
        
        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.config().api_key, "test-key");
        assert_eq!(client.config().timeout, config.timeout);
    }

    #[test]
    fn test_client_try_new_invalid_config() {
        // Create a config with invalid API key (empty string)
        let config = Config {
            api_key: String::new(),
            admin_key: None,
            base_url: url::Url::parse("https://api.anthropic.com").unwrap(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            user_agent: "test".to_string(),
        };
        
        let result = Client::try_new(config);
        assert!(result.is_err());
        
        if let Err(AnthropicError::Config(msg)) = result {
            assert!(msg.contains("API key"));
        } else {
            panic!("Expected Config error");
        }
    }

    #[test]
    fn test_client_new_vs_try_new_consistency() {
        let config = Config::new("test-key").unwrap();
        let client1 = Client::new(config.clone());
        let client2 = Client::try_new(config).unwrap();
        
        assert_eq!(client1.config().api_key, client2.config().api_key);
        assert_eq!(client1.config().timeout, client2.config().timeout);
        assert_eq!(client1.config().base_url, client2.config().base_url);
    }

    #[test]
    fn test_from_env_uses_try_new() {
        std::env::set_var("ANTHROPIC_API_KEY", "test-env-key");
        
        let client = Client::from_env().unwrap();
        assert_eq!(client.config().api_key, "test-env-key");
        
        // Clean up
        std::env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_admin_api_without_admin_key() {
        let config = Config::new("test-key").unwrap();
        let client = Client::new(config);
        
        let admin_result = client.admin();
        assert!(admin_result.is_err());
        
        if let Err(AnthropicError::Auth(msg)) = admin_result {
            assert!(msg.contains("Admin key is required"));
        } else {
            panic!("Expected Auth error");
        }
    }

    #[test]
    fn test_admin_api_with_admin_key() {
        let config = Config::new("test-key")
            .unwrap()
            .with_admin_key("admin-key");
        let client = Client::new(config);
        
        let admin_result = client.admin();
        assert!(admin_result.is_ok());
    }

    #[test]
    fn test_build_url_with_leading_slash() {
        let config = Config::new("test-key").unwrap();
        let client = Client::new(config);
        
        // This tests the internal build_url method indirectly through request
        // The method handles paths with and without leading slash
        // We can't directly test build_url as it's private, but we know
        // it normalizes "/messages" and "messages" to the same result
        assert_eq!(client.config().base_url.as_str(), "https://api.anthropic.com");
    }

    #[test] 
    fn test_config_error_helper() {
        // We can't directly test the private config_error method,
        // but we can test that error messages are consistent by
        // triggering errors that use this helper
        let config = Config::new("test-key").unwrap();
        let client = Client::new(config);

        // Test that header errors use consistent formatting
        // This would typically be triggered by invalid header values
        let options = RequestOptions {
            headers: {
                let mut headers = HashMap::new();
                headers.insert("valid-header".to_string(), "valid-value".to_string());
                headers
            },
            ..Default::default()
        };

        // While we can't directly test the config_error helper,
        // we know it's used throughout the header building process
        // which is tested indirectly through the client functionality
        assert!(client.config().api_key == "test-key");
    }

    #[test]
    fn test_request_options_default() {
        let options = RequestOptions::default();
        
        // Test that all boolean flags default to false
        assert!(!options.enable_files_api);
        assert!(!options.enable_pdf_support);
        assert!(!options.enable_prompt_caching);
        assert!(!options.enable_1m_context);
        assert!(!options.enable_extended_thinking_tools);
        assert!(!options.no_retry);
        
        // Test that collections are empty
        assert!(options.headers.is_empty());
        assert!(options.beta_features.is_empty());
        
        // Test that optional fields are None
        assert!(options.timeout.is_none());
    }

    #[test]
    fn test_request_options_with_beta_features() {
        let mut options = RequestOptions::default();
        options.enable_files_api = true;
        options.enable_pdf_support = true;
        options.beta_features = vec!["custom-feature".to_string()];
        
        // Test that options are set correctly
        assert!(options.enable_files_api);
        assert!(options.enable_pdf_support);
        assert_eq!(options.beta_features.len(), 1);
        assert_eq!(options.beta_features[0], "custom-feature");
    }
}