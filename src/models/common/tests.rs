use super::*;

#[test]
fn test_vec_push_none_option() {
    let mut opt_vec: Option<Vec<String>> = None;
    opt_vec.push_item("test".to_string());
    assert_eq!(opt_vec, Some(vec!["test".to_string()]));
}

#[test]
fn test_vec_push_some_option() {
    let mut opt_vec: Option<Vec<String>> = Some(vec!["existing".to_string()]);
    opt_vec.push_item("new".to_string());
    assert_eq!(
        opt_vec,
        Some(vec!["existing".to_string(), "new".to_string()])
    );
}

#[test]
fn test_vec_push_multiple_items() {
    let mut opt_vec: Option<Vec<i32>> = None;
    opt_vec.push_item(1);
    opt_vec.push_item(2);
    opt_vec.push_item(3);
    assert_eq!(opt_vec, Some(vec![1, 2, 3]));
}

#[test]
fn test_tool_choice_default() {
    let choice = ToolChoice::default();
    assert_eq!(choice, ToolChoice::Auto);
}

#[test]
fn test_metadata_creation() {
    let metadata = Metadata::new().with_user_id("user123").with_custom(
        "key".to_string(),
        serde_json::Value::String("value".to_string()),
    );

    assert_eq!(metadata.user_id, Some("user123".to_string()));
    assert!(metadata.custom.contains_key("key"));
}

#[test]
fn test_usage_total_tokens() {
    let usage = Usage::new(100, 200);
    assert_eq!(usage.total_tokens(), 300);
    assert_eq!(usage.input_tokens, 100);
    assert_eq!(usage.output_tokens, 200);
}

#[test]
fn test_usage_deserializes_partial() {
    let usage: Usage = serde_json::from_str(r#"{"output_tokens":5}"#).unwrap();
    assert_eq!(usage.input_tokens, 0);
    assert_eq!(usage.output_tokens, 5);
    assert_eq!(usage.cache_creation_input_tokens, 0);
    assert_eq!(usage.cache_read_input_tokens, 0);
}

#[test]
fn test_usage_deserializes_extended_fields() {
    let usage: Usage = serde_json::from_str(
        r#"{
                "input_tokens": 10,
                "output_tokens": 5,
                "cache_creation_input_tokens": 3,
                "cache_read_input_tokens": 7,
                "cache_creation": {
                    "ephemeral_5m_input_tokens": 1,
                    "ephemeral_1h_input_tokens": 2
                },
                "server_tool_use": {
                    "web_search_requests": 4
                },
                "inference_geo": "us",
                "service_tier": "standard"
            }"#,
    )
    .unwrap();
    assert_eq!(usage.total_input_tokens(), 20);
    assert_eq!(usage.total_tokens(), 25);
    assert_eq!(
        usage
            .cache_creation
            .as_ref()
            .unwrap()
            .ephemeral_1h_input_tokens,
        2
    );
    assert_eq!(usage.server_tool_use.unwrap().web_search_requests, 4);
    assert_eq!(usage.inference_geo.as_deref(), Some("us"));
    assert_eq!(usage.service_tier.as_deref(), Some("standard"));
}

#[test]
fn test_content_block_creators() {
    let text_block = ContentBlock::text("Hello");
    if let ContentBlock::Text { text, .. } = text_block {
        assert_eq!(text, "Hello");
    } else {
        panic!("Expected text block");
    }

    let tool_result = ContentBlock::tool_result("tool1", Some("result".to_string()));
    if let ContentBlock::ToolResult {
        tool_use_id,
        content,
        is_error,
        ..
    } = tool_result
    {
        assert_eq!(tool_use_id, "tool1");
        assert_eq!(content, Some(ToolResultContent::Text("result".to_string())));
        assert_eq!(is_error, Some(false));
    } else {
        panic!("Expected tool result block");
    }

    let error_result = ContentBlock::tool_error("tool1", "error message");
    if let ContentBlock::ToolResult {
        tool_use_id,
        content,
        is_error,
        ..
    } = error_result
    {
        assert_eq!(tool_use_id, "tool1");
        assert_eq!(
            content,
            Some(ToolResultContent::Text("error message".to_string()))
        );
        assert_eq!(is_error, Some(true));
    } else {
        panic!("Expected error result block");
    }
}

#[test]
fn test_image_source_from_bytes() {
    let bytes = b"fake image data";
    let image_source = ImageSource::from_bytes("image/png", bytes);

    let ImageSource::Base64 {
        media_type, data, ..
    } = image_source
    else {
        panic!("Expected base64 image source");
    };
    assert_eq!(media_type, "image/png");
    // Check that data is base64 encoded
    assert!(!data.is_empty());
}

#[test]
fn test_document_source_file() {
    let source = DocumentSource::file("file_123");
    assert!(matches!(source, DocumentSource::File { .. }));

    let block = ContentBlock::document(source);
    assert!(block.as_document().is_some());
}

#[test]
fn test_role_display() {
    assert_eq!(Role::User.to_string(), "user");
    assert_eq!(Role::Assistant.to_string(), "assistant");
    assert_eq!(Role::System.to_string(), "system");
}

#[test]
fn test_server_tool_serialization() {
    let value = serde_json::to_value(Tool::web_search()).unwrap();
    assert_eq!(value["type"], "web_search_20260209");
    assert_eq!(value["name"], "web_search");
    // Server tools omit description/input_schema.
    assert!(value.get("description").is_none());
    assert!(value.get("input_schema").is_none());

    let code = serde_json::to_value(Tool::code_execution()).unwrap();
    assert_eq!(code["type"], "code_execution_20260120");
}

#[test]
fn test_custom_tool_strict_and_cache() {
    let tool = Tool::new(
        "get_weather",
        "Get weather",
        serde_json::json!({"type": "object"}),
    )
    .with_strict(true)
    .with_cache_control(CacheControl::ephemeral());
    let value = serde_json::to_value(&tool).unwrap();
    assert_eq!(value["name"], "get_weather");
    assert_eq!(value["description"], "Get weather");
    assert_eq!(value["strict"], true);
    assert_eq!(value["cache_control"]["type"], "ephemeral");
    assert!(value.get("type").is_none());
}

#[test]
fn test_text_block_cache_control_roundtrip() {
    let block = ContentBlock::text("hello").with_cache_control(CacheControl::ephemeral_1h());
    let value = serde_json::to_value(&block).unwrap();
    assert_eq!(value["type"], "text");
    assert_eq!(value["cache_control"]["type"], "ephemeral");
    assert_eq!(value["cache_control"]["ttl"], "1h");

    let parsed: ContentBlock = serde_json::from_value(value).unwrap();
    assert_eq!(parsed, block);
}

#[test]
fn test_fallback_content_block_parses() {
    let block: ContentBlock = serde_json::from_value(serde_json::json!({
        "type": "fallback",
        "from": {"model": "claude-fable-5"},
        "to": {"model": "claude-opus-4-8"},
        "trigger": {"type":"refusal"}
    }))
    .unwrap();
    assert!(matches!(block, ContentBlock::Fallback { .. }));
}
