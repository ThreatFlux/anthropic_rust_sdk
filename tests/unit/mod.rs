//! Unit tests for the Threatflux SDK
//! 
//! These tests cover individual components and functions in isolation.
//! All tests use mocks and don't require external API access.

mod client_test;
mod config_test;
mod error_test;
mod builders_test;
mod utils_test;
mod models_test;
mod streaming_test;
mod claude_4_test;
mod types_test;

#[cfg(test)]
mod legacy_config_tests {
    use threatflux::{Config, error::AnthropicError};
    use std::time::Duration;

    #[test]
    fn test_config_creation() {
        let config = Config::new("test-key").unwrap();
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_config_empty_key() {
        let result = Config::new("");
        assert!(matches!(result, Err(AnthropicError::Config(_))));
    }

    #[test]
    fn test_config_builder_pattern() {
        let config = Config::new("test-key")
            .unwrap()
            .with_timeout(Duration::from_secs(30))
            .with_max_retries(5)
            .with_default_model("claude-3-sonnet-20240229");

        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.default_model, "claude-3-sonnet-20240229");
    }

    #[test]
    fn test_config_validation() {
        let config = Config::new("valid-key").unwrap();
        assert!(config.validate().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.api_key = String::new();
        assert!(invalid_config.validate().is_err());
    }
}

#[cfg(test)]
mod legacy_message_builder_tests {
    use threatflux::builders::MessageBuilder;
    use threatflux::models::common::Role;

    #[test]
    fn test_message_builder_basic() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .build();

        assert_eq!(request.model, "claude-3-5-haiku-20241022");
        assert_eq!(request.max_tokens, 100);
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
    }

    #[test]
    fn test_message_builder_conversation() {
        let request = MessageBuilder::new()
            .conversation(&[
                (Role::User, "Hello"),
                (Role::Assistant, "Hi there!"),
                (Role::User, "How are you?"),
            ])
            .build();

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[1].role, Role::Assistant);
        assert_eq!(request.messages[2].role, Role::User);
    }

    #[test]
    fn test_message_builder_presets() {
        let creative = MessageBuilder::new().creative().build();
        assert!(creative.temperature.unwrap() > 0.8);

        let analytical = MessageBuilder::new().analytical().build();
        assert!(analytical.temperature.unwrap() < 0.4);

        let code = MessageBuilder::new().code_generation().build();
        assert!(code.temperature.unwrap() < 0.2);
        assert!(code.stop_sequences.is_some());
    }

    #[test]
    fn test_message_builder_validation() {
        // Valid request
        let valid = MessageBuilder::new()
            .user("Hello")
            .build_validated();
        assert!(valid.is_ok());

        // Invalid - no messages
        let invalid = MessageBuilder::new().build_validated();
        assert!(invalid.is_err());

        // Invalid - zero max_tokens
        let invalid = MessageBuilder::new()
            .max_tokens(0)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());

        // Invalid temperature
        let invalid = MessageBuilder::new()
            .temperature(1.5)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());
    }
}

#[cfg(test)]
mod legacy_batch_builder_tests {
    use threatflux::builders::BatchBuilder;
    use threatflux::models::message::MessageRequest;

    #[test]
    fn test_batch_builder_basic() {
        let batch = BatchBuilder::new()
            .add_simple_request("test1", "claude-3-5-haiku-20241022", "Hello", 100)
            .add_simple_request("test2", "claude-3-5-haiku-20241022", "World", 100)
            .build();

        assert_eq!(batch.requests.len(), 2);
        assert_eq!(batch.requests[0].custom_id, "test1");
        assert_eq!(batch.requests[1].custom_id, "test2");
    }

    #[test]
    fn test_batch_builder_validation() {
        // Valid batch
        let valid = BatchBuilder::new()
            .add_simple_request("test", "claude-3-5-haiku-20241022", "Hello", 100)
            .build_validated();
        assert!(valid.is_ok());

        // Invalid - empty batch
        let invalid = BatchBuilder::new().build_validated();
        assert!(invalid.is_err());

        // Invalid - duplicate custom IDs
        let invalid = BatchBuilder::new()
            .add_simple_request("same", "claude-3-5-haiku-20241022", "Hello", 100)
            .add_simple_request("same", "claude-3-5-haiku-20241022", "World", 100)
            .build_validated();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_batch_builder_with_defaults() {
        let batch = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 200)
            .add("test1", "Hello")
            .add("test2", "World")
            .build();

        assert_eq!(batch.requests.len(), 2);
        assert_eq!(batch.requests[0].params.model, "claude-3-5-haiku-20241022");
        assert_eq!(batch.requests[0].params.max_tokens, 200);
    }
}

#[cfg(test)]
mod legacy_error_tests {
    use threatflux::error::AnthropicError;

    #[test]
    fn test_error_types() {
        let config_error = AnthropicError::config("test error");
        assert!(matches!(config_error, AnthropicError::Config(_)));

        let auth_error = AnthropicError::auth("unauthorized");
        assert!(matches!(auth_error, AnthropicError::Auth(_)));

        let api_error = AnthropicError::api_error(404, "not found".to_string(), None);
        assert!(matches!(api_error, AnthropicError::Api { .. }));
    }

    #[test]
    fn test_error_retryable() {
        let retryable = AnthropicError::api_error(500, "server error".to_string(), None);
        assert!(retryable.is_retryable());

        let not_retryable = AnthropicError::api_error(400, "bad request".to_string(), None);
        assert!(!not_retryable.is_retryable());

        let rate_limit = AnthropicError::rate_limit("too many requests");
        assert!(rate_limit.is_retryable());
    }

    #[test]
    fn test_status_code() {
        let api_error = AnthropicError::api_error(404, "not found".to_string(), None);
        assert_eq!(api_error.status_code(), Some(404));

        let config_error = AnthropicError::config("test");
        assert_eq!(config_error.status_code(), None);
    }
}

#[cfg(test)]
mod legacy_model_tests {
    use threatflux::models::{
        common::{ContentBlock, Usage, Role},
        message::Message,
        model::{Model, ModelFamily, ModelSize},
    };
    use chrono::Utc;

    #[test]
    fn test_content_block_creation() {
        let text_block = ContentBlock::text("Hello world");
        assert_eq!(text_block.as_text(), Some("Hello world"));

        let tool_use = ContentBlock::tool_use("test", "calculator", serde_json::json!({"x": 5}));
        if let ContentBlock::ToolUse { name, .. } = tool_use {
            assert_eq!(name, "calculator");
        } else {
            panic!("Expected ToolUse variant");
        }
    }

    #[test]
    fn test_message_creation() {
        let message = Message::user("Hello");
        assert_eq!(message.role, Role::User);
        assert_eq!(message.text(), "Hello");

        let message = Message::assistant("Hi there!");
        assert_eq!(message.role, Role::Assistant);
        assert_eq!(message.text(), "Hi there!");
    }

    #[test]
    fn test_usage_calculation() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.total_tokens(), 150);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
    }

    #[test]
    fn test_model_family_parsing() {
        assert!(matches!("claude-3-5-haiku-20241022".parse::<ModelFamily>(), Ok(ModelFamily::Claude35)));
        assert!(matches!("claude-3-opus-20240229".parse::<ModelFamily>(), Ok(ModelFamily::Claude3)));
        assert!(matches!("claude-2.1".parse::<ModelFamily>(), Ok(ModelFamily::Legacy)));
    }

    #[test]
    fn test_model_size_parsing() {
        assert!(matches!("claude-3-5-haiku-20241022".parse::<ModelSize>(), Ok(ModelSize::Haiku)));
        assert!(matches!("claude-3-sonnet-20240229".parse::<ModelSize>(), Ok(ModelSize::Sonnet)));
        assert!(matches!("claude-3-opus-20240229".parse::<ModelSize>(), Ok(ModelSize::Opus)));
    }

    #[test]
    fn test_model_capabilities() {
        let model = Model {
            id: "claude-3-5-haiku-20241022".to_string(),
            object_type: "model".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            description: None,
            max_tokens: Some(200000),
            max_output_tokens: Some(8192),
            input_cost_per_token: Some(0.00025),
            output_cost_per_token: Some(0.00125),
            capabilities: Some(vec!["vision".to_string(), "tool_use".to_string()]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deprecated: Some(false),
            deprecation_date: None,
        };

        assert!(model.supports_vision());
        assert!(model.supports_tools());
        assert!(!model.is_deprecated());
        assert_eq!(model.family(), ModelFamily::Claude35);
        assert_eq!(model.size(), ModelSize::Haiku);
    }
}

#[cfg(test)]
mod legacy_pagination_tests {
    use threatflux::types::Pagination;

    #[test]
    fn test_pagination_builder() {
        let pagination = Pagination::new()
            .with_limit(50)
            .with_after("cursor_123")
            .with_before("cursor_456");

        assert_eq!(pagination.limit, Some(50));
        assert_eq!(pagination.after, Some("cursor_123".to_string()));
        assert_eq!(pagination.before, Some("cursor_456".to_string()));
    }

    #[test]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.limit, Some(20));
        assert!(pagination.after.is_none());
        assert!(pagination.before.is_none());
    }
}