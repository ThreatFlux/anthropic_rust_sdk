//! Common test utilities and helpers for Threatflux SDK tests
//!
//! This module provides shared functionality for unit and integration tests:
//! - Mock server setup helpers
//! - Test data fixtures  
//! - API response builders
//! - Environment setup helpers

use serde_json::{json, Value};
use std::collections::HashMap;
use wiremock::{Mock, MockServer, ResponseTemplate, matchers::{method, path, header}};
use threatflux::models::{
    message::{MessageResponse, Message, StreamEvent},
    model::{Model, ModelListResponse},
    batch::{MessageBatch, MessageBatchListResponse, MessageBatchStatus, MessageBatchRequest},
    file::{File, FileListResponse, FileStatus, FilePurpose, FileUploadResponse},
    admin::{Organization, Workspace, UsageReport, WorkspaceStatus},
    common::{Usage, Role, ContentBlock, StopReason},
};
use chrono::Utc;
use uuid::Uuid;

/// Test fixtures for common data structures
pub mod fixtures {
    use super::*;
    
    /// Create a test usage object
    pub fn test_usage() -> Usage {
        Usage::new(100, 50)
    }
    
    /// Create a test message
    pub fn test_message() -> Message {
        Message::user("Hello, test!")
    }
    
    /// Create a test assistant message
    pub fn test_assistant_message() -> Message {
        Message::assistant("Hello back!")
    }
    
    /// Create a test message response
    pub fn test_message_response() -> MessageResponse {
        MessageResponse {
            id: "msg_test123".to_string(),
            object: "message".to_string(),
            created_at: Utc::now(),
            model: "claude-3-5-haiku-20241022".to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::text("Test response")],
            stop_reason: Some(StopReason::EndTurn),
            stop_sequence: None,
            usage: test_usage(),
        }
    }
    
    /// Create a test model
    pub fn test_model() -> Model {
        Model {
            id: "claude-3-5-haiku-20241022".to_string(),
            object_type: "model".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            description: Some("Fast and accurate AI assistant".to_string()),
            max_tokens: Some(200000),
            max_output_tokens: Some(8192),
            input_cost_per_token: Some(0.00025),
            output_cost_per_token: Some(0.00125),
            capabilities: Some(vec!["vision".to_string(), "tool_use".to_string()]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deprecated: Some(false),
            deprecation_date: None,
        }
    }
    
    /// Create a test model list response
    pub fn test_model_list_response() -> ModelListResponse {
        ModelListResponse {
            object: "list".to_string(),
            data: vec![test_model()],
            has_more: false,
            first_id: Some("claude-3-5-haiku-20241022".to_string()),
            last_id: Some("claude-3-5-haiku-20241022".to_string()),
        }
    }
    
    /// Create a test batch
    pub fn test_batch() -> MessageBatch {
        MessageBatch {
            id: "batch_test123".to_string(),
            object: "message_batch".to_string(),
            processing_status: MessageBatchStatus::InProgress,
            request_counts: threatflux::models::batch::BatchRequestCounts {
                processing: 1,
                succeeded: 0,
                errored: 0,
                canceled: 0,
                expired: 0,
                total: 1,
            },
            ended_at: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            archive_at: Utc::now() + chrono::Duration::days(30),
            cancel_initiated_at: None,
            results_url: None,
        }
    }
    
    /// Create a test batch list response
    pub fn test_batch_list_response() -> MessageBatchListResponse {
        MessageBatchListResponse {
            object: "list".to_string(),
            data: vec![test_batch()],
            has_more: false,
            first_id: Some("batch_test123".to_string()),
            last_id: Some("batch_test123".to_string()),
        }
    }
    
    /// Create a test file
    pub fn test_file() -> File {
        File {
            id: "file_test123".to_string(),
            object: "file".to_string(),
            filename: "test.txt".to_string(),
            size_bytes: 1024,
            mime_type: "text/plain".to_string(),
            purpose: FilePurpose::UserData,
            status: FileStatus::Uploaded,
            created_at: Utc::now(),
            expires_at: Some(Utc::now() + chrono::Duration::days(30)),
        }
    }
    
    /// Create a test file list response
    pub fn test_file_list_response() -> FileListResponse {
        FileListResponse {
            object: "list".to_string(),
            data: vec![test_file()],
            has_more: false,
            first_id: Some("file_test123".to_string()),
            last_id: Some("file_test123".to_string()),
        }
    }
    
    /// Create a test file upload response
    pub fn test_file_upload_response() -> FileUploadResponse {
        FileUploadResponse {
            file: test_file(),
        }
    }
    
    /// Create a test organization
    pub fn test_organization() -> Organization {
        Organization {
            id: "org_test123".to_string(),
            object: "organization".to_string(),
            name: "Test Organization".to_string(),
            display_name: "Test Org".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
    
    /// Create a test workspace
    pub fn test_workspace() -> Workspace {
        Workspace {
            id: "ws_test123".to_string(),
            object: "workspace".to_string(),
            name: "Test Workspace".to_string(),
            display_name: "Test WS".to_string(),
            status: WorkspaceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
        }
    }
    
    /// Create a test usage report
    pub fn test_usage_report() -> UsageReport {
        UsageReport {
            input_tokens: 1000,
            output_tokens: 500,
        }
    }
}

/// Mock server helpers for testing
pub mod mock_server {
    use super::*;
    
    /// Setup a mock server for testing
    pub async fn setup_mock_server() -> MockServer {
        MockServer::start().await
    }
    
    /// Mock successful message creation
    pub fn mock_message_create(server: &MockServer) -> Mock {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_message_response()))
    }
    
    /// Mock message streaming
    pub fn mock_message_stream(server: &MockServer) -> Mock {
        let stream_events = vec![
            r#"event: message_start
data: {"type":"message_start","message":{"id":"msg_test123","type":"message","role":"assistant","model":"claude-3-5-haiku-20241022","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}

"#,
            r#"event: content_block_start
data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}

"#,
            r#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

"#,
            r#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":" world"}}

"#,
            r#"event: content_block_stop
data: {"type":"content_block_stop","index":0}

"#,
            r#"event: message_delta
data: {"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":2}}

"#,
            r#"event: message_stop
data: {"type":"message_stop"}

"#,
        ];
        
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_header("content-type", "text/event-stream")
                .set_body_string(stream_events.join("")))
    }
    
    /// Mock models list
    pub fn mock_models_list(server: &MockServer) -> Mock {
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_model_list_response()))
    }
    
    /// Mock specific model get
    pub fn mock_model_get(server: &MockServer) -> Mock {
        Mock::given(method("GET"))
            .and(path("/v1/models/claude-3-5-haiku-20241022"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_model()))
    }
    
    /// Mock batch creation
    pub fn mock_batch_create(server: &MockServer) -> Mock {
        Mock::given(method("POST"))
            .and(path("/v1/message_batches"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch()))
    }
    
    /// Mock batch list
    pub fn mock_batch_list(server: &MockServer) -> Mock {
        Mock::given(method("GET"))
            .and(path("/v1/message_batches"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch_list_response()))
    }
    
    /// Mock batch get
    pub fn mock_batch_get(server: &MockServer) -> Mock {
        Mock::given(method("GET"))
            .and(path("/v1/message_batches/batch_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch()))
    }
    
    /// Mock file upload
    pub fn mock_file_upload(server: &MockServer) -> Mock {
        Mock::given(method("POST"))
            .and(path("/v1/files"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file_upload_response()))
    }
    
    /// Mock file list
    pub fn mock_file_list(server: &MockServer) -> Mock {
        Mock::given(method("GET"))
            .and(path("/v1/files"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file_list_response()))
    }
    
    /// Mock file get
    pub fn mock_file_get(server: &MockServer) -> Mock {
        Mock::given(method("GET"))
            .and(path("/v1/files/file_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_file()))
    }
    
    /// Mock file delete
    pub fn mock_file_delete(server: &MockServer) -> Mock {
        Mock::given(method("DELETE"))
            .and(path("/v1/files/file_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&json!({"deleted": true})))
    }
    
    /// Mock error response
    pub fn mock_error_response(server: &MockServer, status: u16, error_type: &str, message: &str) -> Mock {
        Mock::given(method("POST"))
            .and(path("/v1/messages"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(status)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": error_type,
                        "message": message
                    }
                })))
    }
    
    /// Mock rate limit error
    pub fn mock_rate_limit_error(server: &MockServer) -> Mock {
        mock_error_response(server, 429, "rate_limit_error", "Rate limit exceeded")
    }
    
    /// Mock authentication error
    pub fn mock_auth_error(server: &MockServer) -> Mock {
        mock_error_response(server, 401, "authentication_error", "Invalid API key")
    }
    
    /// Mock server error
    pub fn mock_server_error(server: &MockServer) -> Mock {
        mock_error_response(server, 500, "internal_server_error", "Internal server error")
    }
}

/// Environment setup helpers
pub mod env {
    use std::env;
    
    /// Setup test environment variables
    pub fn setup_test_env() {
        env::set_var("ANTHROPIC_API_KEY", "test-key");
        env::set_var("ANTHROPIC_BASE_URL", "http://localhost:3000");
    }
    
    /// Cleanup test environment variables
    pub fn cleanup_test_env() {
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("ANTHROPIC_BASE_URL");
    }
    
    /// Check if real API tests should run
    pub fn should_run_real_api_tests() -> bool {
        env::var("ANTHROPIC_API_KEY").is_ok() && 
        env::var("RUN_REAL_API_TESTS").unwrap_or_default() == "true"
    }
}

/// HTTP response builders for testing
pub mod responses {
    use super::*;
    
    /// Build a successful JSON response
    pub fn success_json<T: serde::Serialize>(data: &T) -> ResponseTemplate {
        ResponseTemplate::new(200).set_body_json(data)
    }
    
    /// Build an error response
    pub fn error_response(status: u16, error_type: &str, message: &str) -> ResponseTemplate {
        ResponseTemplate::new(status).set_body_json(&json!({
            "type": "error",
            "error": {
                "type": error_type,
                "message": message
            }
        }))
    }
    
    /// Build a streaming response
    pub fn streaming_response(events: &[&str]) -> ResponseTemplate {
        ResponseTemplate::new(200)
            .set_header("content-type", "text/event-stream")
            .set_body_string(events.join("\n"))
    }
}