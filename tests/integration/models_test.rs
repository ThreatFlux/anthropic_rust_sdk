//! Integration tests for Models API
//!
//! Tests Models API endpoints with mocked responses.

use wiremock::{MockServer, Mock, ResponseTemplate, matchers::{method, path, header, query_param}};
use threatflux::{Client, Config, types::Pagination};
use serde_json::json;
use pretty_assertions::assert_eq;

mod common;
use crate::common::{fixtures, mock_server};

#[cfg(test)]
mod models_api_tests {
    use super::*;
    
    async fn setup_test_client(mock_server: &MockServer) -> Client {
        let config = Config::new("test-key")
            .unwrap()
            .with_base_url(mock_server.uri().parse().unwrap());
        Client::new(config)
    }

    #[tokio::test]
    async fn test_list_models_success() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_model_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().list(None, None).await;
        
        assert!(response.is_ok());
        let models = response.unwrap();
        assert_eq!(models.object, "list");
        assert!(!models.data.is_empty());
        assert_eq!(models.data[0].id, "claude-3-5-haiku-20241022");
    }

    #[tokio::test]
    async fn test_list_models_with_pagination() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(query_param("limit", "10"))
            .and(query_param("after", "cursor_123"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_model_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let pagination = Pagination::new()
            .with_limit(10)
            .with_after("cursor_123");
        
        let response = client.models().list(Some(pagination), None).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_get_specific_model() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/models/claude-3-5-haiku-20241022"))
            .and(header("x-api-key", "test-key"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_model()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().get("claude-3-5-haiku-20241022", None).await;
        
        assert!(response.is_ok());
        let model = response.unwrap();
        assert_eq!(model.id, "claude-3-5-haiku-20241022");
        assert_eq!(model.display_name, "Claude 3.5 Haiku");
        assert!(model.supports_vision());
        assert!(model.supports_tools());
    }

    #[tokio::test]
    async fn test_get_model_not_found() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/models/invalid-model"))
            .respond_with(ResponseTemplate::new(404)
                .set_body_json(&json!({
                    "type": "error",
                    "error": {
                        "type": "not_found_error",
                        "message": "Model not found"
                    }
                })))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().get("invalid-model", None).await;
        
        assert!(response.is_err());
        if let Err(threatflux::error::AnthropicError::Api { status, .. }) = response {
            assert_eq!(status, 404);
        } else {
            panic!("Expected 404 error");
        }
    }

    #[tokio::test]
    async fn test_list_models_with_capabilities() {
        let mock_server = MockServer::start().await;
        
        let models_with_capabilities = json!({
            "object": "list",
            "data": [
                {
                    "id": "claude-3-5-haiku-20241022",
                    "type": "model",
                    "display_name": "Claude 3.5 Haiku",
                    "description": "Fast and accurate AI assistant",
                    "max_tokens": 200000,
                    "max_output_tokens": 8192,
                    "input_cost_per_token": 0.00025,
                    "output_cost_per_token": 0.00125,
                    "capabilities": ["vision", "tool_use", "computer_use"],
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z",
                    "deprecated": false,
                    "deprecation_date": null
                }
            ],
            "has_more": false,
            "first_id": "claude-3-5-haiku-20241022",
            "last_id": "claude-3-5-haiku-20241022"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&models_with_capabilities))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().list(None, None).await;
        assert!(response.is_ok());
        
        let models = response.unwrap();
        let model = &models.data[0];
        assert!(model.supports_vision());
        assert!(model.supports_tools());
        assert!(model.supports_computer_use());
    }

    #[tokio::test]
    async fn test_model_family_detection() {
        let mock_server = MockServer::start().await;
        
        let various_models = json!({
            "object": "list",
            "data": [
                {
                    "id": "claude-3-5-sonnet-20240620",
                    "type": "model",
                    "display_name": "Claude 3.5 Sonnet",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                },
                {
                    "id": "claude-3-opus-20240229",
                    "type": "model", 
                    "display_name": "Claude 3 Opus",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                },
                {
                    "id": "claude-2.1",
                    "type": "model",
                    "display_name": "Claude 2.1",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-01T00:00:00Z"
                }
            ],
            "has_more": false
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&various_models))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().list(None, None).await;
        assert!(response.is_ok());
        
        let models = response.unwrap();
        
        // Test family detection
        assert_eq!(models.data[0].family(), threatflux::models::model::ModelFamily::Claude35);
        assert_eq!(models.data[1].family(), threatflux::models::model::ModelFamily::Claude3);
        assert_eq!(models.data[2].family(), threatflux::models::model::ModelFamily::Legacy);
        
        // Test size detection
        assert_eq!(models.data[0].size(), threatflux::models::model::ModelSize::Sonnet);
        assert_eq!(models.data[1].size(), threatflux::models::model::ModelSize::Opus);
    }

    #[tokio::test]
    async fn test_deprecated_models() {
        let mock_server = MockServer::start().await;
        
        let deprecated_model = json!({
            "id": "claude-1-deprecated",
            "type": "model",
            "display_name": "Claude 1 (Deprecated)",
            "deprecated": true,
            "deprecation_date": "2024-12-31T23:59:59Z",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models/claude-1-deprecated"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&deprecated_model))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().get("claude-1-deprecated", None).await;
        assert!(response.is_ok());
        
        let model = response.unwrap();
        assert!(model.is_deprecated());
        assert!(model.deprecation_date.is_some());
    }

    #[tokio::test]
    async fn test_model_pricing_info() {
        let mock_server = MockServer::start().await;
        
        let model_with_pricing = json!({
            "id": "claude-3-opus-20240229",
            "type": "model",
            "display_name": "Claude 3 Opus",
            "max_tokens": 200000,
            "max_output_tokens": 4096,
            "input_cost_per_token": 0.015,
            "output_cost_per_token": 0.075,
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models/claude-3-opus-20240229"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&model_with_pricing))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().get("claude-3-opus-20240229", None).await;
        assert!(response.is_ok());
        
        let model = response.unwrap();
        assert_eq!(model.input_cost_per_token, Some(0.015));
        assert_eq!(model.output_cost_per_token, Some(0.075));
        assert_eq!(model.max_tokens, Some(200000));
        assert_eq!(model.max_output_tokens, Some(4096));
    }

    #[tokio::test]
    async fn test_list_models_empty() {
        let mock_server = MockServer::start().await;
        
        let empty_response = json!({
            "object": "list",
            "data": [],
            "has_more": false,
            "first_id": null,
            "last_id": null
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&empty_response))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let response = client.models().list(None, None).await;
        assert!(response.is_ok());
        
        let models = response.unwrap();
        assert!(models.data.is_empty());
        assert!(!models.has_more);
    }

    #[tokio::test]
    async fn test_models_error_handling() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
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
        
        let response = client.models().list(None, None).await;
        assert!(response.is_err());
        
        if let Err(error) = response {
            assert!(error.is_retryable());
        }
    }

    #[tokio::test]
    async fn test_models_with_request_options() {
        let mock_server = MockServer::start().await;
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&fixtures::test_model_list_response()))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let options = threatflux::types::RequestOptions {
            priority: Some(threatflux::types::RequestPriority::Low),
            timeout: Some(std::time::Duration::from_secs(10)),
            retry_count: Some(1),
            metadata: None,
        };
        
        let response = client.models().list(None, Some(options)).await;
        assert!(response.is_ok());
    }

    #[tokio::test]
    async fn test_models_pagination_has_more() {
        let mock_server = MockServer::start().await;
        
        let response_with_more = json!({
            "object": "list",
            "data": [fixtures::test_model()],
            "has_more": true,
            "first_id": "claude-3-5-haiku-20241022",
            "last_id": "claude-3-5-haiku-20241022"
        });
        
        Mock::given(method("GET"))
            .and(path("/v1/models"))
            .and(query_param("limit", "1"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_json(&response_with_more))
            .mount(&mock_server)
            .await;
        
        let client = setup_test_client(&mock_server).await;
        
        let pagination = Pagination::new().with_limit(1);
        let response = client.models().list(Some(pagination), None).await;
        
        assert!(response.is_ok());
        let models = response.unwrap();
        assert!(models.has_more);
        assert_eq!(models.data.len(), 1);
    }
}