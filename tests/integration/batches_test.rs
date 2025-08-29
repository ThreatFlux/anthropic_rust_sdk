//! Integration tests for Batch API
//!
//! Tests Batch API operations with mocked responses.

use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, header, query_param}};
use threatflux::{Client, Config, builders::BatchBuilder, types::Pagination};
use serde_json::json;
use pretty_assertions::assert_eq;

mod common;
use crate::common::{fixtures, mock_server};

#[cfg(test)]
mod batches_api_tests {
    use super::*;
    
    async fn setup_test_client(mock_server: &MockServer) -> Client {
        let config = Config::new("test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    #[tokio::test]
    async fn test_create_batch_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/message_batches"))
            .and(header("x-api-key", "test-key"))
            .and(header("content-type", "application/json"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let batch_request = BatchBuilder::new()
            .add_simple_request("req1", "claude-3-5-haiku-20241022", "Hello", 100)
            .add_simple_request("req2", "claude-3-5-haiku-20241022", "World", 100)
            .build();
        
        let response = client.message_batches().create(batch_request, None).await;
        
        assert!(response.is_ok());
        let batch = response.unwrap();
        assert_eq!(batch.id, "batch_test123");
        assert_eq!(batch.processing_status, threatflux::models::batch::MessageBatchStatus::InProgress);
        assert_eq!(batch.request_counts.total, 1);
    }

    #[tokio::test]
    async fn test_create_batch_with_builder() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/message_batches"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let batch_request = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 100)
            .add("simple1", "First message")
            .add("simple2", "Second message")
            .add_simple_request("explicit", "claude-3-sonnet-20240229", "Explicit request", 200)
            .build();
        
        let response = client.message_batches().create(batch_request, None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_retrieve_batch() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/message_batches/batch_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().retrieve("batch_test123", None).await;
        
        assert!(response.is_ok());
        let batch = response.unwrap();
        assert_eq!(batch.id, "batch_test123");
    }

    #[tokio::test]
    async fn test_list_batches() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/message_batches"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().list(None, None).await;
        
        assert!(response.is_ok());
        let batches = response.unwrap();
        assert_eq!(batches.object, "list");
        assert!(!batches.data.is_empty());
        assert_eq!(batches.data[0].id, "batch_test123");
    }

    #[tokio::test]
    async fn test_list_batches_with_pagination() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/message_batches"))
            .and(query_param("limit", "10"))
            .and(query_param("after", "batch_cursor"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_batch_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let pagination = Pagination::new()
            .with_limit(10)
            .with_after("batch_cursor");
        
        let response = client.message_batches().list(Some(pagination), None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_cancel_batch() {
        let mock_server = MockServer::start().await;
        
        let mut canceled_batch = fixtures::test_batch();
        canceled_batch.processing_status = threatflux::models::batch::MessageBatchStatus::Canceling;
        canceled_batch.cancel_initiated_at = Some(chrono::Utc::now());
        
        Mock::given(method("POST"))
            .and(path("/v1/message_batches/batch_test123/cancel"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&canceled_batch))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().cancel("batch_test123", None).await;
        
        assert!(response.is_ok());
        let batch = response.unwrap();
        assert_eq!(batch.processing_status, threatflux::models::batch::MessageBatchStatus::Canceling);
        assert!(batch.cancel_initiated_at.is_some());
    }

    #[tokio::test]
    async fn test_delete_batch() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("DELETE"))
            .and(path("/v1/message_batches/batch_test123"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&json!({"deleted": true})))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().delete("batch_test123", None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_batch_results() {
        let mock_server = MockServer::start().await;
        
        let results_response = json!({
            "results": [
                {
                    "custom_id": "req1",
                    "result": {
                        "type": "succeeded",
                        "message": {
                            "id": "msg_123",
                            "type": "message",
                            "role": "assistant",
                            "model": "claude-3-5-haiku-20241022",
                            "content": [{"type": "text", "text": "Hello response"}],
                            "stop_reason": "end_turn",
                            "usage": {"input_tokens": 5, "output_tokens": 10}
                        }
                    }
                }
            ]
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/message_batches/batch_test123/results"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&results_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().results("batch_test123", None).await;
        assert!(response.is_ok());
        
        let results = response.unwrap();
        assert!(!results.results.is_empty());
        assert_eq!(results.results[0].custom_id, "req1");
    }

    #[tokio::test]
    async fn test_batch_status_transitions() {
        let mock_server = MockServer::start().await;
        
        // Test different batch statuses
        let statuses = vec![
            threatflux::models::batch::MessageBatchStatus::InProgress,
            threatflux::models::batch::MessageBatchStatus::Canceling,
            threatflux::models::batch::MessageBatchStatus::Ended,
        ];
        
        for (i, status) in statuses.iter().enumerate() {
            let mut batch = fixtures::test_batch();
            batch.id = format!("batch_{}", i);
            batch.processing_status = status.clone();
            
            Mock::given(method("GET"))
                .and(path(&format!("/v1/message_batches/batch_{}", i)))
                .respond_with(ResponseTemplate::new(200)
                    .set_body_json(&batch))
                .mount(&mock_server)
                .await;
        }
        
        let client = setup_test_client(&mock_server).await;
        
        for (i, expected_status) in statuses.iter().enumerate() {
            let response = client.message_batches().retrieve(&format!("batch_{}", i), None).await;
            assert!(response.is_ok());
            
            let batch = response.unwrap();
            assert_eq!(batch.processing_status, *expected_status);
        }
    }

    #[tokio::test]
    async fn test_batch_error_handling() {
        let mock_server = MockServer::start().await;
        
        // Test 404 for non-existent batch
        Mock::given(method("GET"))
            .and(path("/v1/message_batches/nonexistent"))
            .respond_with(ResponseTemplate::new(404)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "not_found_error",
                        "message": "Batch not found"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().retrieve("nonexistent", None).await;
        assert!(response.is_err());
        
        if let Err(threatflux::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 404);
        } else {
            panic!("Expected 404 error");
        }
    }

    #[tokio::test]
    async fn test_batch_validation_error() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("POST"))
            .and(path("/v1/message_batches"))
            .respond_with(ResponseTemplate::new(400)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "invalid_request_error",
                        "message": "Invalid batch request"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        // Create an invalid batch (empty)
        let batch_request = BatchBuilder::new().build();
        
        let response = client.message_batches().create(batch_request, None).await;
        assert!(response.is_err());
        
        if let Err(threatflux::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 400);
        } else {
            panic!("Expected 400 error");
        }
    }

    #[tokio::test]
    async fn test_batch_with_large_request_count() {
        let mock_server = MockServer::start().await;
        
        let large_batch = json!({
            "id": "batch_large",
            "type": "message_batch",
            "processing_status": "in_progress",
            "request_counts": {
                "processing": 100,
                "succeeded": 0,
                "errored": 0,
                "canceled": 0,
                "expired": 0,
                "total": 100
            },
            "ended_at": null,
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-01-02T00:00:00Z",
            "archive_at": "2024-01-31T00:00:00Z",
            "cancel_initiated_at": null,
            "results_url": null
        });
        
        Mock::given(method("POST"))
            .and(path("/v1/message_batches"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&large_batch))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let mut batch_builder = BatchBuilder::new()
            .with_defaults("claude-3-5-haiku-20241022", 50);
        
        // Add 100 requests
        for i in 0..100 {
            batch_builder = batch_builder.add(&format!("req{}", i), &format!("Message {}", i));
        }
        
        let batch_request = batch_builder.build();
        let response = client.message_batches().create(batch_request, None).await;
        
        assert!(response.is_ok());
        let batch = response.unwrap();
        assert_eq!(batch.request_counts.total, 100);
    }

    #[tokio::test]
    async fn test_batch_with_mixed_results() {
        let mock_server = MockServer::start().await;
        
        let mixed_results = json!({
            "results": [
                {
                    "custom_id": "req1",
                    "result": {
                        "type": "succeeded",
                        "message": {
                            "id": "msg_123",
                            "type": "message",
                            "role": "assistant",
                            "model": "claude-3-5-haiku-20241022",
                            "content": [{"type": "text", "text": "Success"}],
                            "stop_reason": "end_turn",
                            "usage": {"input_tokens": 5, "output_tokens": 7}
                        }
                    }
                },
                {
                    "custom_id": "req2",
                    "result": {
                        "type": "errored",
                        "error": {
                            "type": "invalid_request_error",
                            "message": "Invalid request parameters"
                        }
                    }
                },
                {
                    "custom_id": "req3",
                    "result": {
                        "type": "canceled"
                    }
                }
            ]
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/message_batches/batch_mixed/results"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&mixed_results))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().results("batch_mixed", None).await;
        assert!(response.is_ok());
        
        let results = response.unwrap();
        assert_eq!(results.results.len(), 3);
        
        // Check different result types
        assert_eq!(results.results[0].custom_id, "req1");
        assert_eq!(results.results[1].custom_id, "req2");
        assert_eq!(results.results[2].custom_id, "req3");
    }

    #[tokio::test]
    async fn test_batch_expiration() {
        let mock_server = MockServer::start().await;
        
        let expired_batch = json!({
            "id": "batch_expired",
            "type": "message_batch",
            "processing_status": "ended",
            "request_counts": {
                "processing": 0,
                "succeeded": 0,
                "errored": 0,
                "canceled": 0,
                "expired": 5,
                "total": 5
            },
            "ended_at": "2024-01-02T00:00:00Z",
            "created_at": "2024-01-01T00:00:00Z",
            "expires_at": "2024-01-02T00:00:00Z",
            "archive_at": "2024-01-31T00:00:00Z",
            "cancel_initiated_at": null,
            "results_url": null
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/message_batches/batch_expired"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&expired_batch))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.message_batches().retrieve("batch_expired", None).await;
        assert!(response.is_ok());
        
        let batch = response.unwrap();
        assert_eq!(batch.processing_status, threatflux::models::batch::MessageBatchStatus::Ended);
        assert_eq!(batch.request_counts.expired, 5);
        assert!(batch.ended_at.is_some());
    }
}