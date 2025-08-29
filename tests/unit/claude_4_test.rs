//! Unit tests for Claude 4 specific features

use threatflux::{
    builders::MessageBuilder,
    config::models,
    models::message::{MessageRequest, ThinkingConfig},
    types::RequestOptions,
};

#[test]
fn test_claude_4_models() {
    // Test model support checks
    assert!(models::supports_thinking(models::OPUS_4_1));
    assert!(models::supports_thinking(models::OPUS_4));
    assert!(models::supports_thinking(models::SONNET_4));
    assert!(!models::supports_thinking(models::HAIKU_3_5));
    
    // Test 1M context support
    assert!(models::supports_1m_context(models::SONNET_4));
    assert!(!models::supports_1m_context(models::OPUS_4_1));
    
    // Test max thinking tokens
    assert_eq!(models::max_thinking_tokens(models::OPUS_4_1), Some(64000));
    assert_eq!(models::max_thinking_tokens(models::OPUS_4), Some(64000));
    assert_eq!(models::max_thinking_tokens(models::SONNET_4), Some(32000));
    assert_eq!(models::max_thinking_tokens(models::HAIKU_3_5), None);
}

#[test]
fn test_thinking_config() {
    let config = ThinkingConfig::enabled(50000);
    assert_eq!(config.thinking_type, "enabled");
    assert_eq!(config.budget_tokens, Some(50000));
    assert_eq!(config.allow_tool_use, None);
    
    let config_with_tools = ThinkingConfig::enabled_with_tools(30000);
    assert_eq!(config_with_tools.thinking_type, "enabled");
    assert_eq!(config_with_tools.budget_tokens, Some(30000));
    assert_eq!(config_with_tools.allow_tool_use, Some(true));
    
    let disabled = ThinkingConfig::disabled();
    assert_eq!(disabled.thinking_type, "disabled");
    assert_eq!(disabled.budget_tokens, None);
}

#[test]
fn test_message_builder_with_thinking() {
    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .thinking(50000)
        .user("Test message")
        .build();
    
    assert_eq!(request.model, models::OPUS_4_1);
    assert!(request.thinking.is_some());
    if let Some(thinking) = request.thinking {
        assert_eq!(thinking.thinking_type, "enabled");
        assert_eq!(thinking.budget_tokens, Some(50000));
    }
}

#[test]
fn test_message_builder_claude_4_presets() {
    // Test Opus 4 deep thinking preset
    let request = MessageBuilder::new()
        .opus_4_deep_thinking()
        .user("Complex problem")
        .build();
    
    assert_eq!(request.model, models::OPUS_4_1);
    assert_eq!(request.max_tokens, 8192);
    assert!(request.thinking.is_some());
    if let Some(thinking) = request.thinking {
        assert_eq!(thinking.budget_tokens, Some(64000));
    }
    
    // Test Sonnet 4 balanced preset
    let request = MessageBuilder::new()
        .sonnet_4_balanced()
        .user("Balanced task")
        .build();
    
    assert_eq!(request.model, models::SONNET_4);
    assert_eq!(request.max_tokens, 4096);
    assert!(request.thinking.is_some());
    if let Some(thinking) = request.thinking {
        assert_eq!(thinking.budget_tokens, Some(32000));
    }
}

#[test]
fn test_opus_4_1_constraints() {
    // Test that validation would reject both temperature and top_p
    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .temperature(0.7)
        .top_p(0.9) // This combination should be invalid for Opus 4.1
        .user("Test")
        .build();
    
    // The build_validated() method should catch this
    let result = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .temperature(0.7)
        .top_p(0.9)
        .user("Test")
        .build_validated();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("cannot use both temperature and top_p"));
    }
}

#[test]
fn test_thinking_budget_validation() {
    // Valid thinking budget
    let result = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .thinking(50000) // Within limit
        .user("Test")
        .build_validated();
    
    assert!(result.is_ok());
    
    // Exceeding thinking budget
    let result = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .thinking(100000) // Exceeds 64000 limit
        .user("Test")
        .build_validated();
    
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("exceeds maximum"));
    }
}

#[test]
fn test_request_options_for_claude_4() {
    let options = RequestOptions::for_claude_4_thinking(50000);
    assert!(options.enable_extended_thinking_tools);
    
    let options = RequestOptions::for_claude_4_thinking(30000);
    assert!(!options.enable_extended_thinking_tools);
    
    let options = RequestOptions::for_sonnet_4_large_context();
    assert!(options.enable_1m_context);
}

#[test]
fn test_beta_features() {
    let mut options = RequestOptions::new()
        .with_1m_context()
        .with_extended_thinking_tools()
        .with_beta_feature("custom-feature");
    
    assert!(options.enable_1m_context);
    assert!(options.enable_extended_thinking_tools);
    assert_eq!(options.beta_features.len(), 1);
    assert_eq!(options.beta_features[0], "custom-feature");
}

#[test]
fn test_message_request_serialization_with_thinking() {
    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .thinking(50000)
        .user("Test")
        .build();
    
    let json = serde_json::to_value(request).unwrap();
    
    assert!(json["thinking"].is_object());
    assert_eq!(json["thinking"]["type"], "enabled");
    assert_eq!(json["thinking"]["budget_tokens"], 50000);
}