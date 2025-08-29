//! Unit tests for Builder modules
//!
//! Tests MessageBuilder and BatchBuilder functionality, validation, and builder patterns.

use threatflux::{
    builders::{
        MessageBuilder, BatchBuilder, BatchBuilderWithDefaults,
        ParameterBuilder, ValidatedBuilder, FluentBuilder, 
        PresetConfig, ValidationUtils
    },
    models::{
        common::{Role, ContentBlock, Tool, ToolChoice, ImageSource},
        message::{MessageRequest, Message},
        batch::{MessageBatchCreateRequest, MessageBatchRequest},
    },
    error::AnthropicError,
};
use pretty_assertions::assert_eq;
use serde_json::json;

#[cfg(test)]
mod message_builder_tests {
    use super::*;

    #[test]
    fn test_message_builder_basic() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello, world!")
            .build();

        assert_eq!(request.model, "claude-3-5-haiku-20241022");
        assert_eq!(request.max_tokens, 100);
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[0].text(), "Hello, world!");
    }

    #[test]
    fn test_message_builder_conversation() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(200)
            .conversation(&[
                (Role::User, "Hello"),
                (Role::Assistant, "Hi there!"),
                (Role::User, "How are you?"),
            ])
            .build();

        assert_eq!(request.messages.len(), 3);
        assert_eq!(request.messages[0].role, Role::User);
        assert_eq!(request.messages[0].text(), "Hello");
        assert_eq!(request.messages[1].role, Role::Assistant);
        assert_eq!(request.messages[1].text(), "Hi there!");
        assert_eq!(request.messages[2].role, Role::User);
        assert_eq!(request.messages[2].text(), "How are you?");
    }

    #[test]
    fn test_message_builder_with_system() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .system("You are a helpful assistant.")
            .user("Hello")
            .build();

        assert_eq!(request.system, Some("You are a helpful assistant.".to_string()));
        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
    }

    #[test]
    fn test_message_builder_presets() {
        // Creative preset
        let creative = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Write a story")
            .creative()
            .build();
        
        assert!(creative.temperature.unwrap() > 0.8);

        // Analytical preset
        let analytical = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Analyze this data")
            .analytical()
            .build();
        
        assert!(analytical.temperature.unwrap() < 0.4);

        // Code generation preset
        let code = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Write a function")
            .code_generation()
            .build();
        
        assert!(code.temperature.unwrap() < 0.2);
        assert!(code.stop_sequences.is_some());
        assert!(code.stop_sequences.as_ref().unwrap().contains(&"```".to_string()));
    }

    #[test]
    fn test_message_builder_parameters() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(500)
            .temperature(0.7)
            .top_p(0.9)
            .top_k(50)
            .stop_sequences(vec!["STOP".to_string(), "END".to_string()])
            .user("Test message")
            .build();

        assert_eq!(request.temperature, Some(0.7));
        assert_eq!(request.top_p, Some(0.9));
        assert_eq!(request.top_k, Some(50));
        assert_eq!(request.stop_sequences, Some(vec!["STOP".to_string(), "END".to_string()]));
    }

    #[test]
    fn test_message_builder_streaming() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Stream this message")
            .stream()
            .build();

        assert_eq!(request.stream, Some(true));
    }

    #[test]
    fn test_message_builder_with_tools() {
        let tool = Tool {
            name: "calculator".to_string(),
            description: "A simple calculator".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "Mathematical expression to evaluate"
                    }
                },
                "required": ["expression"]
            }),
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .tools(vec![tool.clone()])
            .tool_choice(ToolChoice::Tool { name: "calculator".to_string() })
            .user("Calculate 2+2")
            .build();

        assert_eq!(request.tools, Some(vec![tool]));
        assert!(matches!(request.tool_choice, Some(ToolChoice::Tool { .. })));
    }

    #[test]
    fn test_message_builder_with_image() {
        let image_source = ImageSource {
            source_type: "base64".to_string(),
            media_type: "image/jpeg".to_string(),
            data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user_with_image("Describe this image", image_source.clone())
            .build();

        assert_eq!(request.messages.len(), 1);
        assert_eq!(request.messages[0].role, Role::User);
        
        // Check that the message contains both text and image content
        assert!(request.messages[0].content.len() >= 2);
        let has_text = request.messages[0].content.iter().any(|c| matches!(c, ContentBlock::Text { .. }));
        let has_image = request.messages[0].content.iter().any(|c| matches!(c, ContentBlock::Image { .. }));
        assert!(has_text && has_image);
    }

    #[test]
    fn test_message_builder_validation() {
        // Valid request
        let valid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .build_validated();
        assert!(valid.is_ok());

        // Invalid - no messages
        let invalid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - zero max_tokens
        let invalid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(0)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - negative temperature
        let invalid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .temperature(-0.1)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - temperature > 1.0
        let invalid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .temperature(1.1)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - top_p out of range
        let invalid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .top_p(1.5)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - negative top_k
        let invalid = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .top_k(-1)
            .user("Hello")
            .build_validated();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_message_builder_metadata() {
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .metadata(json!({"user_id": "test123", "session": "abc"}))
            .build();

        assert_eq!(request.metadata, Some(json!({"user_id": "test123", "session": "abc"})));
    }

    #[test]
    fn test_message_builder_edge_cases() {
        // Very long message
        let long_text = "a".repeat(10000);
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user(&long_text)
            .build();
        assert_eq!(request.messages[0].text(), long_text);

        // Multiple stop sequences
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .stop_sequences(vec!["STOP".to_string(), "END".to_string(), "QUIT".to_string()])
            .user("Hello")
            .build();
        assert_eq!(request.stop_sequences.as_ref().unwrap().len(), 3);

        // Maximum reasonable values
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(200000)
            .temperature(1.0)
            .top_p(1.0)
            .top_k(1000)
            .user("Hello")
            .build();
        assert_eq!(request.max_tokens, 200000);
        assert_eq!(request.temperature, Some(1.0));
        assert_eq!(request.top_p, Some(1.0));
        assert_eq!(request.top_k, Some(1000));
    }
}

#[cfg(test)]
mod batch_builder_tests {
    use super::*;

    #[test]
    fn test_batch_builder_basic() {
        let batch = BatchBuilder::new()
            .add_simple_request("req1", "claude-3-5-haiku-20241022", "Hello", 100)
            .add_simple_request("req2", "claude-3-5-haiku-20241022", "World", 100)
            .build();

        assert_eq!(batch.requests.len(), 2);
        assert_eq!(batch.requests[0].custom_id, "req1");
        assert_eq!(batch.requests[1].custom_id, "req2");
    }

    #[test]
    fn test_batch_builder_with_message_request() {
        let message_request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(200)
            .user("Complex request")
            .temperature(0.8)
            .build();

        let batch = BatchBuilder::new()
            .add_request("complex1", message_request)
            .build();

        assert_eq!(batch.requests.len(), 1);
        assert_eq!(batch.requests[0].custom_id, "complex1");
        assert_eq!(batch.requests[0].params.temperature, Some(0.8));
    }

    #[test]
    fn test_batch_builder_with_defaults() {
        let batch = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 150)
            .add("req1", "First message")
            .add("req2", "Second message")
            .build();

        assert_eq!(batch.requests.len(), 2);
        assert_eq!(batch.requests[0].params.model, "claude-3-5-haiku-20241022");
        assert_eq!(batch.requests[0].params.max_tokens, 150);
        assert_eq!(batch.requests[1].params.model, "claude-3-5-haiku-20241022");
        assert_eq!(batch.requests[1].params.max_tokens, 150);
    }

    #[test]
    fn test_batch_builder_mixed_requests() {
        let custom_request = MessageBuilder::new()
            .model("claude-3-sonnet-20240229")
            .max_tokens(500)
            .user("Custom model request")
            .build();

        let batch = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 100)
            .add("simple", "Simple request")
            .add_request("custom", custom_request)
            .add_simple_request("explicit", "claude-3-5-haiku-20241022", "Explicit request", 200)
            .build();

        assert_eq!(batch.requests.len(), 3);
        assert_eq!(batch.requests[0].custom_id, "simple");
        assert_eq!(batch.requests[1].custom_id, "custom");
        assert_eq!(batch.requests[2].custom_id, "explicit");
        
        // Check models
        assert_eq!(batch.requests[0].params.model, "claude-3-5-haiku-20241022");
        assert_eq!(batch.requests[1].params.model, "claude-3-sonnet-20240229");
        assert_eq!(batch.requests[2].params.model, "claude-3-5-haiku-20241022");
    }

    #[test]
    fn test_batch_builder_validation() {
        // Valid batch
        let valid = BatchBuilder::new()
            .add_simple_request("req1", "claude-3-5-haiku-20241022", "Hello", 100)
            .build_validated();
        assert!(valid.is_ok());

        // Invalid - empty batch
        let invalid = BatchBuilder::new().build_validated();
        assert!(invalid.is_err());

        // Invalid - duplicate custom IDs
        let invalid = BatchBuilder::new()
            .add_simple_request("same", "claude-3-5-haiku-20241022", "First", 100)
            .add_simple_request("same", "claude-3-5-haiku-20241022", "Second", 100)
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - empty custom ID
        let invalid = BatchBuilder::new()
            .add_simple_request("", "claude-3-5-haiku-20241022", "Hello", 100)
            .build_validated();
        assert!(invalid.is_err());

        // Invalid - invalid message request
        let invalid_message = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(0) // Invalid
            .user("Hello")
            .build();
            
        let invalid = BatchBuilder::new()
            .add_request("invalid", invalid_message)
            .build_validated();
        assert!(invalid.is_err());
    }

    #[test]
    fn test_batch_builder_large_batch() {
        let mut builder = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 100);
        
        // Add many requests
        for i in 0..100 {
            builder = builder.add(&format!("req{}", i), &format!("Message {}", i));
        }
        
        let batch = builder.build();
        assert_eq!(batch.requests.len(), 100);
        
        // Check that all have unique IDs
        let mut ids = std::collections::HashSet::new();
        for request in &batch.requests {
            assert!(ids.insert(&request.custom_id), "Duplicate ID: {}", request.custom_id);
        }
    }

    #[test]
    fn test_batch_builder_edge_cases() {
        // Very long custom ID
        let long_id = "a".repeat(1000);
        let batch = BatchBuilder::new()
            .add_simple_request(&long_id, "claude-3-5-haiku-20241022", "Hello", 100)
            .build();
        assert_eq!(batch.requests[0].custom_id, long_id);

        // Special characters in custom ID
        let special_id = "req-with_special.chars@123!";
        let batch = BatchBuilder::new()
            .add_simple_request(special_id, "claude-3-5-haiku-20241022", "Hello", 100)
            .build();
        assert_eq!(batch.requests[0].custom_id, special_id);

        // Unicode custom ID
        let unicode_id = "req_测试_🚀";
        let batch = BatchBuilder::new()
            .add_simple_request(unicode_id, "claude-3-5-haiku-20241022", "Hello", 100)
            .build();
        assert_eq!(batch.requests[0].custom_id, unicode_id);
    }

    #[test]
    fn test_batch_builder_reset_defaults() {
        let batch = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 100)
            .add("req1", "First")
            .with_defaults("claude-3-sonnet-20240229", 200)
            .add("req2", "Second")
            .build();

        assert_eq!(batch.requests[0].params.model, "claude-3-5-haiku-20241022");
        assert_eq!(batch.requests[0].params.max_tokens, 100);
        assert_eq!(batch.requests[1].params.model, "claude-3-sonnet-20240229");
        assert_eq!(batch.requests[1].params.max_tokens, 200);
    }

    #[test]
    fn test_batch_builder_preset_methods() {
        let batch = BatchBuilder::new()
            .add_creative("creative1", "claude-3-5-haiku-20241022", "Write a story", 2000)
            .add_analytical("analytical1", "claude-3-5-haiku-20241022", "Analyze this", 1500)
            .add_code_generation("code1", "claude-3-5-haiku-20241022", "Write a function", 2000)
            .add_conversational("conv1", "claude-3-5-haiku-20241022", "Hello there", 1000)
            .build();

        assert_eq!(batch.requests.len(), 4);
        
        // Check all have expected parameters
        for request in &batch.requests {
            assert_eq!(request.params.model, "claude-3-5-haiku-20241022");
            assert!(request.params.temperature.is_some());
            assert!(request.params.top_p.is_some());
        }

        // Verify creative has higher temperature
        let creative_temp = batch.requests[0].params.temperature.unwrap();
        let analytical_temp = batch.requests[1].params.temperature.unwrap();
        assert!(creative_temp > analytical_temp);
    }

    #[test]
    fn test_batch_builder_with_defaults_presets() {
        let batch = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 1000)
            .add_creative_with_defaults("creative1", "Tell me a story")
            .add_analytical_with_defaults("analytical1", "Explain quantum mechanics")
            .add_code_generation_with_defaults("code1", "Write a sorting algorithm")
            .add_conversational_with_defaults("conv1", "How's the weather?")
            .build();

        assert_eq!(batch.requests.len(), 4);
        
        // All should use default model and max_tokens
        for request in &batch.requests {
            assert_eq!(request.params.model, "claude-3-5-haiku-20241022");
            assert_eq!(request.params.max_tokens, 1000);
        }
    }

    #[test]
    fn test_batch_builder_comprehensive_validation() {
        // Test validation with temperature/top_p violations
        let invalid_request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .temperature(2.0) // Invalid
            .user("Hello")
            .build();
            
        let invalid = BatchBuilder::new()
            .add_request("invalid_temp", invalid_request)
            .build_validated();
        assert!(invalid.is_err());

        // Test validation with Claude 4 constraints
        let opus_violation = MessageBuilder::new()
            .model("claude-opus-4-1")
            .max_tokens(100)
            .temperature(0.5)
            .top_p(0.9) // Both temperature and top_p - invalid for Opus 4.1
            .user("Hello")
            .build();
            
        let invalid = BatchBuilder::new()
            .add_request("opus_violation", opus_violation)
            .build_validated();
        assert!(invalid.is_err());
    }
}

#[cfg(test)]
mod common_traits_tests {
    use super::*;

    #[test]
    fn test_parameter_builder_trait() {
        let builder = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello");

        // Test trait methods
        let builder = builder.creative();
        let request = builder.build();
        
        assert_eq!(request.temperature, Some(0.9));
        assert_eq!(request.top_p, Some(0.95));
        assert_eq!(request.max_tokens, 2000);
    }

    #[test]
    fn test_preset_configs() {
        let creative = PresetConfig::CREATIVE;
        assert_eq!(creative.temperature, 0.9);
        assert_eq!(creative.top_p, 0.95);
        assert_eq!(creative.max_tokens, 2000);

        let analytical = PresetConfig::ANALYTICAL;
        assert_eq!(analytical.temperature, 0.3);
        assert_eq!(analytical.top_p, 0.85);
        assert_eq!(analytical.max_tokens, 1500);

        let code_gen = PresetConfig::CODE_GENERATION;
        assert_eq!(code_gen.temperature, 0.1);
        assert_eq!(code_gen.top_p, 0.9);
        assert_eq!(code_gen.max_tokens, 2000);

        let conversational = PresetConfig::CONVERSATIONAL;
        assert_eq!(conversational.temperature, 0.7);
        assert_eq!(conversational.top_p, 0.9);
        assert_eq!(conversational.max_tokens, 1000);
    }

    #[test]
    fn test_validated_builder_trait() {
        // Test MessageBuilder implementation
        let message_builder = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello");
        
        let result: Result<MessageRequest, _> = message_builder.build_validated();
        assert!(result.is_ok());

        // Test BatchBuilder implementation
        let batch_builder = BatchBuilder::new()
            .add_simple_request("req1", "claude-3-5-haiku-20241022", "Hello", 100);
        
        let result: Result<MessageBatchCreateRequest, _> = batch_builder.build_validated();
        assert!(result.is_ok());
    }

    #[test]
    fn test_fluent_builder_trait() {
        let message_builder = MessageBuilder::new();
        let debug_info = message_builder.inspect();
        assert!(debug_info.is_some());

        let batch_builder = BatchBuilder::new();
        let debug_info = batch_builder.inspect();
        assert!(debug_info.is_some());
    }

    #[test]
    fn test_validation_utils() {
        // Test max_tokens validation
        assert!(ValidationUtils::validate_max_tokens(0, "Test").is_err());
        assert!(ValidationUtils::validate_max_tokens(1, "Test").is_ok());
        assert!(ValidationUtils::validate_max_tokens(1000, "Test").is_ok());

        // Test temperature validation
        assert!(ValidationUtils::validate_temperature(-0.1).is_err());
        assert!(ValidationUtils::validate_temperature(0.0).is_ok());
        assert!(ValidationUtils::validate_temperature(0.5).is_ok());
        assert!(ValidationUtils::validate_temperature(1.0).is_ok());
        assert!(ValidationUtils::validate_temperature(1.1).is_err());

        // Test top_p validation
        assert!(ValidationUtils::validate_top_p(-0.1).is_err());
        assert!(ValidationUtils::validate_top_p(0.0).is_ok());
        assert!(ValidationUtils::validate_top_p(0.5).is_ok());
        assert!(ValidationUtils::validate_top_p(1.0).is_ok());
        assert!(ValidationUtils::validate_top_p(1.1).is_err());

        // Test messages validation
        assert!(ValidationUtils::validate_messages_not_empty(0, "Test").is_err());
        assert!(ValidationUtils::validate_messages_not_empty(1, "Test").is_ok());
        assert!(ValidationUtils::validate_messages_not_empty(10, "Test").is_ok());

        // Test Claude 4 constraints
        assert!(ValidationUtils::validate_claude_4_constraints(
            "claude-3-sonnet",
            Some(0.5),
            Some(0.9)
        ).is_ok());
        
        assert!(ValidationUtils::validate_claude_4_constraints(
            "claude-opus-4-1",
            Some(0.5),
            Some(0.9)
        ).is_err());
        
        assert!(ValidationUtils::validate_claude_4_constraints(
            "claude-opus-4-1",
            Some(0.5),
            None
        ).is_ok());
    }

    #[test]
    fn test_preset_application() {
        let builder = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .user("Test message");

        // Test each preset
        let creative_request = builder.clone().with_preset(PresetConfig::CREATIVE).build();
        assert_eq!(creative_request.temperature, Some(0.9));
        assert_eq!(creative_request.max_tokens, 2000);

        let analytical_request = builder.clone().with_preset(PresetConfig::ANALYTICAL).build();
        assert_eq!(analytical_request.temperature, Some(0.3));
        assert_eq!(analytical_request.max_tokens, 1500);

        let code_request = builder.clone().with_preset(PresetConfig::CODE_GENERATION).build();
        assert_eq!(code_request.temperature, Some(0.1));
        assert_eq!(code_request.max_tokens, 2000);
    }

    #[test]
    fn test_builder_consistency() {
        // Test that both old and new preset methods give same results
        let old_creative = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .user("Write a story")
            .temperature(0.9)
            .top_p(0.95)
            .max_tokens(2000)
            .build();

        let new_creative = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .user("Write a story")
            .creative()
            .build();

        assert_eq!(old_creative.temperature, new_creative.temperature);
        assert_eq!(old_creative.top_p, new_creative.top_p);
        assert_eq!(old_creative.max_tokens, new_creative.max_tokens);
    }

    #[test]
    fn test_error_message_context() {
        // Test that validation errors include context
        let error = ValidationUtils::validate_max_tokens(0, "TestContext");
        assert!(error.is_err());
        let error_msg = format!("{}", error.unwrap_err());
        assert!(error_msg.contains("TestContext"));
    }
}