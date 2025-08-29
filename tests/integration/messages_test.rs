//! Integration tests for Messages API
//!
//! Tests Messages API with mocked responses, covering all endpoints and scenarios.

use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, header}};
use threatflux::{Client, Config, builders::MessageBuilder, error::AnthropicError};
use serde_json::json;
use pretty_assertions::assert_eq;

mod common;
use crate::common::{fixtures, mock_server};

#[cfg(test)]
mod messages_api_tests {
    use super::*;
    
    async fn setup_test_client(mock_server: &MockServer) -> Client {
        let config = Config::new("test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    #[tokio::test]
    async fn test_create_message_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello, test!")
            .build();
        
        let response = client.messages().create(request, None).await;
        
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.text(), "Test response");
        assert_eq!(response.model, "claude-3-5-haiku-20241022");
        assert!(response.usage.total_tokens() > 0);
    }

    #[tokio::test]
    async fn test_create_message_with_system() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .system("You are a helpful assistant.")
            .user("Hello!")
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_create_message_with_conversation() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .conversation(&[
                (threatflux::models::common::Role::User, "Hello"),
                (threatflux::models::common::Role::Assistant, "Hi there!"),
                (threatflux::models::common::Role::User, "How are you?"),
            ])
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_ok());
        
        let response = response.unwrap();
        assert_eq!(response.text(), "Test response");
    }

    #[tokio::test]
    async fn test_create_message_with_tools() {
        let mock_server = MockServer::start().await;
        
        let mut response_with_tools = fixtures::test_message_response();
        response_with_tools.content = vec![
            threatflux::models::common::ContentBlock::tool_use(
                "tool_123",
                "calculator", 
                json!({"expression": "2+2"})
            )
        ];
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&response_with_tools))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let tool = threatflux::models::common::Tool {
            name: "calculator".to_string(),
            description: "A simple calculator".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "expression": {"type": "string"}
                },
                "required": ["expression"]
            }),
        };
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .tools(vec![tool])
            .user("What is 2+2?")
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_streaming_message() {
        let mock_server = MockServer::start().await;
        
        let stream_events = vec![
            r#"event: message_start"#,
            r#"data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","model":"claude-3-5-haiku-20241022","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#,
            r#""#,
            r#"event: content_block_start"#,
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            r#""#,
            r#"event: content_block_delta"#,
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
            r#""#,
            r#"event: content_block_delta"#,
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}"#,
            r#""#,
            r#"event: content_block_stop"#,
            r#"data: {"type":"content_block_stop","index":0}"#,
            r#""#,
            r#"event: message_delta"#,
            r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":5}}"#,
            r#""#,
            r#"event: message_stop"#,
            r#"data: {"type":"message_stop"}"#,
            r#""#,
        ];
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("accept", "text/event-stream"))
            .respond_with(ResponseTemplate::new(200)
                .set_header("content-type", "text/event-stream")
                .set_body_string(stream_events.join("\n")))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(50)
            .user("Hello")
            .stream()
            .build();
        
        let stream = client.messages().create_stream(request, None).await;
        assert!(stream.is_ok());
        
        let text = stream.unwrap().collect_text().await;
        assert!(text.is_ok());
        assert_eq!(text.unwrap(), "Hello world");
    }

    #[tokio::test]
    async fn test_count_tokens() {
        let mock_server = MockServer::start().await;
        
        let token_response = json!({
            "input_tokens": 15
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/messages/count_tokens"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&token_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .user("Hello, how many tokens is this?")
            .build();
        
        let result = client.messages().count_tokens(request, None).await;
        assert!(result.is_ok());
        
        let count = result.unwrap();
        assert_eq!(count.input_tokens, 15);
    }

    #[tokio::test]
    async fn test_count_tokens_simple() {
        let mock_server = MockServer::start().await;
        
        let token_response = json!({
            "input_tokens": 8
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/messages/count_tokens"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&token_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let result = client.messages()
            .count_tokens_simple("claude-3-5-haiku-20241022", "Hello world", None)
            .await;
        
        assert!(result.is_ok());
        let count = result.unwrap();
        assert_eq!(count.input_tokens, 8);
    }

    #[tokio::test]
    async fn test_message_error_400() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(400)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "invalid_request_error",
                        "message": "Invalid request parameters"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("invalid-model")
            .max_tokens(0) // Invalid
            .user("Hello")
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_err());
        
        if let Err(AnthropicError::Api { status, message, error_type }) = response {
            assert_eq!(status, 400);
            assert!(message.contains("Invalid request"));
            assert_eq!(error_type, Some("invalid_request_error".to_string()));
        } else {
            panic!("Expected API error");
        }
    }

    #[tokio::test]
    async fn test_message_error_401() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(401)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "authentication_error",
                        "message": "Invalid API key"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_err());
        
        if let Err(AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 401);
        } else {
            panic!("Expected 401 error");
        }
    }

    #[tokio::test]
    async fn test_message_error_429() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(429)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "rate_limit_error",
                        "message": "Rate limit exceeded"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_err());
        
        if let Err(error) = response {
            assert!(error.is_retryable());
        } else {
            panic!("Expected error");
        }
    }

    #[tokio::test]
    async fn test_message_error_500() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(500)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "internal_server_error",
                        "message": "Internal server error"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_err());
        
        if let Err(error) = response {
            assert!(error.is_retryable());
        } else {
            panic!("Expected error");
        }
    }

    #[tokio::test]
    async fn test_message_with_metadata() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .metadata(json!({"user_id": "test123", "session": "session_abc"}))
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_message_with_image() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let image_source = threatflux::models::common::ImageSource {
            source_type: "base64".to_string(),
            media_type: "image/jpeg".to_string(),
            data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==".to_string(),
        };
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user_with_image("Describe this image", image_source)
            .build();
        
        let response = client.messages().create(request, None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_message_request_options() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Hello")
            .build();
        
        let options = threatflux::types::RequestOptions {
            priority: Some(threatflux::types::RequestPriority::High),
            timeout: Some(std::time::Duration::from_secs(30)),
            retry_count: Some(2),
            metadata: Some(json!({"test": true})),
        };
        
        let response = client.messages().create(request, Some(options)).await;
        assert!(response.is_ok());
    }
}