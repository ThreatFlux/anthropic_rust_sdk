//! Unit tests for the types module
//!
//! Tests HTTP methods, request options, pagination, and other common types.

use threatflux::types::*;
use std::time::Duration;
use std::collections::HashMap;

#[cfg(test)]
mod http_method_tests {
    use super::*;

    #[test]
    fn test_http_method_as_str() {
        assert_eq!(HttpMethod::Get.as_str(), "GET");
        assert_eq!(HttpMethod::Post.as_str(), "POST");
        assert_eq!(HttpMethod::Put.as_str(), "PUT");
        assert_eq!(HttpMethod::Patch.as_str(), "PATCH");
        assert_eq!(HttpMethod::Delete.as_str(), "DELETE");
    }

    #[test]
    fn test_http_method_equality() {
        assert_eq!(HttpMethod::Get, HttpMethod::Get);
        assert_ne!(HttpMethod::Get, HttpMethod::Post);
    }

    #[test]
    fn test_http_method_clone() {
        let method1 = HttpMethod::Post;
        let method2 = method1.clone();
        assert_eq!(method1, method2);
    }

    #[test]
    fn test_http_method_debug() {
        let debug_str = format!("{:?}", HttpMethod::Get);
        assert!(debug_str.contains("Get"));
    }

    #[test]
    fn test_http_method_copy() {
        let method1 = HttpMethod::Delete;
        let method2 = method1; // Copy, not move
        assert_eq!(method1, method2);
    }
}

#[cfg(test)]
mod request_options_tests {
    use super::*;

    #[test]
    fn test_request_options_new() {
        let options = RequestOptions::new();
        assert!(options.headers.is_empty());
        assert!(options.timeout.is_none());
        assert!(!options.no_retry);
        assert!(!options.enable_files_api);
        assert!(!options.enable_pdf_support);
        assert!(!options.enable_prompt_caching);
        assert!(!options.enable_1m_context);
        assert!(!options.enable_extended_thinking_tools);
        assert!(options.beta_features.is_empty());
    }

    #[test]
    fn test_request_options_default() {
        let options = RequestOptions::default();
        assert!(options.headers.is_empty());
        assert!(options.timeout.is_none());
        assert!(!options.no_retry);
    }

    #[test]
    fn test_request_options_with_header() {
        let options = RequestOptions::new()
            .with_header("Authorization", "Bearer token")
            .with_header("Content-Type", "application/json");
        
        assert_eq!(options.headers.len(), 2);
        assert_eq!(options.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(options.headers.get("Content-Type"), Some(&"application/json".to_string()));
    }

    #[test]
    fn test_request_options_with_timeout() {
        let timeout = Duration::from_secs(30);
        let options = RequestOptions::new().with_timeout(timeout);
        assert_eq!(options.timeout, Some(timeout));
    }

    #[test]
    fn test_request_options_no_retry() {
        let options = RequestOptions::new().no_retry();
        assert!(options.no_retry);
    }

    #[test]
    fn test_request_options_with_files_api() {
        let options = RequestOptions::new().with_files_api();
        assert!(options.enable_files_api);
    }

    #[test]
    fn test_request_options_with_pdf_support() {
        let options = RequestOptions::new().with_pdf_support();
        assert!(options.enable_pdf_support);
    }

    #[test]
    fn test_request_options_with_prompt_caching() {
        let options = RequestOptions::new().with_prompt_caching();
        assert!(options.enable_prompt_caching);
    }

    #[test]
    fn test_request_options_with_1m_context() {
        let options = RequestOptions::new().with_1m_context();
        assert!(options.enable_1m_context);
    }

    #[test]
    fn test_request_options_with_extended_thinking_tools() {
        let options = RequestOptions::new().with_extended_thinking_tools();
        assert!(options.enable_extended_thinking_tools);
    }

    #[test]
    fn test_request_options_with_beta_feature() {
        let options = RequestOptions::new()
            .with_beta_feature("feature1")
            .with_beta_feature("feature2");
        
        assert_eq!(options.beta_features.len(), 2);
        assert!(options.beta_features.contains(&"feature1".to_string()));
        assert!(options.beta_features.contains(&"feature2".to_string()));
    }

    #[test]
    fn test_request_options_for_claude_4_thinking_low_budget() {
        let options = RequestOptions::for_claude_4_thinking(16000);
        assert!(!options.enable_extended_thinking_tools); // Below threshold
    }

    #[test]
    fn test_request_options_for_claude_4_thinking_high_budget() {
        let options = RequestOptions::for_claude_4_thinking(40000);
        assert!(options.enable_extended_thinking_tools); // Above threshold
    }

    #[test]
    fn test_request_options_for_sonnet_4_large_context() {
        let options = RequestOptions::for_sonnet_4_large_context();
        assert!(options.enable_1m_context);
    }

    #[test]
    fn test_request_options_chaining() {
        let options = RequestOptions::new()
            .with_header("X-Custom", "value")
            .with_timeout(Duration::from_secs(45))
            .no_retry()
            .with_files_api()
            .with_pdf_support()
            .with_prompt_caching()
            .with_1m_context()
            .with_extended_thinking_tools()
            .with_beta_feature("custom-feature");

        assert_eq!(options.headers.len(), 1);
        assert_eq!(options.timeout, Some(Duration::from_secs(45)));
        assert!(options.no_retry);
        assert!(options.enable_files_api);
        assert!(options.enable_pdf_support);
        assert!(options.enable_prompt_caching);
        assert!(options.enable_1m_context);
        assert!(options.enable_extended_thinking_tools);
        assert_eq!(options.beta_features.len(), 1);
    }

    #[test]
    fn test_request_options_clone() {
        let options1 = RequestOptions::new()
            .with_header("test", "value")
            .with_beta_feature("feature");
        let options2 = options1.clone();
        
        assert_eq!(options1.headers, options2.headers);
        assert_eq!(options1.beta_features, options2.beta_features);
    }

    #[test]
    fn test_request_options_debug() {
        let options = RequestOptions::new().with_header("key", "value");
        let debug_str = format!("{:?}", options);
        assert!(debug_str.contains("RequestOptions"));
    }
}

#[cfg(test)]
mod pagination_tests {
    use super::*;

    #[test]
    fn test_pagination_new() {
        let pagination = Pagination::new();
        assert_eq!(pagination.limit, Some(20));
        assert!(pagination.after.is_none());
        assert!(pagination.before.is_none());
    }

    #[test]
    fn test_pagination_default() {
        let pagination = Pagination::default();
        assert_eq!(pagination.limit, Some(20));
        assert!(pagination.after.is_none());
        assert!(pagination.before.is_none());
    }

    #[test]
    fn test_pagination_with_limit() {
        let pagination = Pagination::new().with_limit(50);
        assert_eq!(pagination.limit, Some(50));
    }

    #[test]
    fn test_pagination_with_after() {
        let pagination = Pagination::new().with_after("cursor123");
        assert_eq!(pagination.after, Some("cursor123".to_string()));
    }

    #[test]
    fn test_pagination_with_before() {
        let pagination = Pagination::new().with_before("cursor456");
        assert_eq!(pagination.before, Some("cursor456".to_string()));
    }

    #[test]
    fn test_pagination_chaining() {
        let pagination = Pagination::new()
            .with_limit(100)
            .with_after("start_cursor")
            .with_before("end_cursor");
        
        assert_eq!(pagination.limit, Some(100));
        assert_eq!(pagination.after, Some("start_cursor".to_string()));
        assert_eq!(pagination.before, Some("end_cursor".to_string()));
    }

    #[test]
    fn test_pagination_clone() {
        let pagination1 = Pagination::new().with_limit(25);
        let pagination2 = pagination1.clone();
        assert_eq!(pagination1.limit, pagination2.limit);
    }

    #[test]
    fn test_pagination_debug() {
        let pagination = Pagination::new();
        let debug_str = format!("{:?}", pagination);
        assert!(debug_str.contains("Pagination"));
    }

    #[test]
    fn test_pagination_serialization() {
        let pagination = Pagination::new()
            .with_limit(10)
            .with_after("test_cursor");
        
        let serialized = serde_json::to_string(&pagination).unwrap();
        assert!(serialized.contains("\"limit\":10"));
        assert!(serialized.contains("\"after\":\"test_cursor\""));
    }
}

#[cfg(test)]
mod paginated_response_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_paginated_response_creation() {
        let response: PaginatedResponse<String> = PaginatedResponse {
            data: vec!["item1".to_string(), "item2".to_string()],
            has_more: true,
            first_id: Some("first".to_string()),
            last_id: Some("last".to_string()),
        };

        assert_eq!(response.data.len(), 2);
        assert!(response.has_more);
        assert_eq!(response.first_id, Some("first".to_string()));
        assert_eq!(response.last_id, Some("last".to_string()));
    }

    #[test]
    fn test_paginated_response_serialization() {
        let response: PaginatedResponse<i32> = PaginatedResponse {
            data: vec![1, 2, 3],
            has_more: false,
            first_id: None,
            last_id: None,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["data"], json!([1, 2, 3]));
        assert_eq!(json["has_more"], json!(false));
        assert!(json["first_id"].is_null());
        assert!(json["last_id"].is_null());
    }

    #[test]
    fn test_paginated_response_deserialization() {
        let json = json!({
            "data": ["a", "b"],
            "has_more": true,
            "first_id": "first_cursor",
            "last_id": "last_cursor"
        });

        let response: PaginatedResponse<String> = serde_json::from_value(json).unwrap();
        assert_eq!(response.data, vec!["a", "b"]);
        assert!(response.has_more);
        assert_eq!(response.first_id, Some("first_cursor".to_string()));
        assert_eq!(response.last_id, Some("last_cursor".to_string()));
    }

    #[test]
    fn test_paginated_response_clone() {
        let response1: PaginatedResponse<i32> = PaginatedResponse {
            data: vec![42],
            has_more: false,
            first_id: Some("test".to_string()),
            last_id: None,
        };
        let response2 = response1.clone();
        
        assert_eq!(response1.data, response2.data);
        assert_eq!(response1.has_more, response2.has_more);
        assert_eq!(response1.first_id, response2.first_id);
    }

    #[test]
    fn test_paginated_response_debug() {
        let response: PaginatedResponse<String> = PaginatedResponse {
            data: vec!["test".to_string()],
            has_more: true,
            first_id: None,
            last_id: None,
        };
        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("PaginatedResponse"));
    }
}

#[cfg(test)]
mod api_error_response_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_api_error_response_creation() {
        let error = ApiErrorResponse {
            error_type: "invalid_request_error".to_string(),
            message: "Invalid API key".to_string(),
        };

        assert_eq!(error.error_type, "invalid_request_error");
        assert_eq!(error.message, "Invalid API key");
    }

    #[test]
    fn test_api_error_response_serialization() {
        let error = ApiErrorResponse {
            error_type: "rate_limit_error".to_string(),
            message: "Too many requests".to_string(),
        };

        let json = serde_json::to_value(&error).unwrap();
        assert_eq!(json["type"], "rate_limit_error");
        assert_eq!(json["message"], "Too many requests");
    }

    #[test]
    fn test_api_error_response_deserialization() {
        let json = json!({
            "type": "authentication_error",
            "message": "Invalid credentials"
        });

        let error: ApiErrorResponse = serde_json::from_value(json).unwrap();
        assert_eq!(error.error_type, "authentication_error");
        assert_eq!(error.message, "Invalid credentials");
    }

    #[test]
    fn test_api_error_response_clone() {
        let error1 = ApiErrorResponse {
            error_type: "test".to_string(),
            message: "test message".to_string(),
        };
        let error2 = error1.clone();
        
        assert_eq!(error1.error_type, error2.error_type);
        assert_eq!(error1.message, error2.message);
    }

    #[test]
    fn test_api_error_response_debug() {
        let error = ApiErrorResponse {
            error_type: "test_error".to_string(),
            message: "Test message".to_string(),
        };
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("ApiErrorResponse"));
    }
}

#[cfg(test)]
mod model_capability_tests {
    use super::*;

    #[test]
    fn test_model_capability_variants() {
        // Test all variants exist
        let _capabilities = [
            ModelCapability::TextGeneration,
            ModelCapability::VisionProcessing,
            ModelCapability::FunctionCalling,
            ModelCapability::PromptCaching,
            ModelCapability::ExtendedThinking,
            ModelCapability::LargeContext1M,
            ModelCapability::HybridReasoning,
            ModelCapability::ToolUseDuringThinking,
        ];
    }

    #[test]
    fn test_model_capability_equality() {
        assert_eq!(ModelCapability::TextGeneration, ModelCapability::TextGeneration);
        assert_ne!(ModelCapability::TextGeneration, ModelCapability::VisionProcessing);
    }

    #[test]
    fn test_model_capability_clone() {
        let cap1 = ModelCapability::ExtendedThinking;
        let cap2 = cap1.clone();
        assert_eq!(cap1, cap2);
    }

    #[test]
    fn test_model_capability_debug() {
        let debug_str = format!("{:?}", ModelCapability::LargeContext1M);
        assert!(debug_str.contains("LargeContext1M"));
    }

    #[test]
    fn test_model_capability_copy() {
        let cap1 = ModelCapability::FunctionCalling;
        let cap2 = cap1; // Copy, not move
        assert_eq!(cap1, cap2);
    }
}

#[cfg(test)]
mod request_priority_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_priority_variants() {
        let _priorities = [
            RequestPriority::Low,
            RequestPriority::Normal,
            RequestPriority::High,
        ];
    }

    #[test]
    fn test_request_priority_default() {
        assert_eq!(RequestPriority::default(), RequestPriority::Normal);
    }

    #[test]
    fn test_request_priority_equality() {
        assert_eq!(RequestPriority::Low, RequestPriority::Low);
        assert_ne!(RequestPriority::Low, RequestPriority::High);
    }

    #[test]
    fn test_request_priority_serialization() {
        assert_eq!(serde_json::to_value(RequestPriority::Low).unwrap(), json!("low"));
        assert_eq!(serde_json::to_value(RequestPriority::Normal).unwrap(), json!("normal"));
        assert_eq!(serde_json::to_value(RequestPriority::High).unwrap(), json!("high"));
    }

    #[test]
    fn test_request_priority_deserialization() {
        assert_eq!(serde_json::from_value::<RequestPriority>(json!("low")).unwrap(), RequestPriority::Low);
        assert_eq!(serde_json::from_value::<RequestPriority>(json!("normal")).unwrap(), RequestPriority::Normal);
        assert_eq!(serde_json::from_value::<RequestPriority>(json!("high")).unwrap(), RequestPriority::High);
    }

    #[test]
    fn test_request_priority_clone() {
        let priority1 = RequestPriority::High;
        let priority2 = priority1.clone();
        assert_eq!(priority1, priority2);
    }

    #[test]
    fn test_request_priority_debug() {
        let debug_str = format!("{:?}", RequestPriority::Normal);
        assert!(debug_str.contains("Normal"));
    }

    #[test]
    fn test_request_priority_copy() {
        let priority1 = RequestPriority::High;
        let priority2 = priority1; // Copy, not move
        assert_eq!(priority1, priority2);
    }
}

#[cfg(test)]
mod stream_event_type_tests {
    use super::*;

    #[test]
    fn test_stream_event_type_variants() {
        let _events = [
            StreamEventType::MessageStart,
            StreamEventType::MessageDelta,
            StreamEventType::MessageStop,
            StreamEventType::ContentBlockStart,
            StreamEventType::ContentBlockDelta,
            StreamEventType::ContentBlockStop,
            StreamEventType::Ping,
            StreamEventType::Error,
        ];
    }

    #[test]
    fn test_stream_event_type_display() {
        assert_eq!(StreamEventType::MessageStart.to_string(), "message_start");
        assert_eq!(StreamEventType::MessageDelta.to_string(), "message_delta");
        assert_eq!(StreamEventType::MessageStop.to_string(), "message_stop");
        assert_eq!(StreamEventType::ContentBlockStart.to_string(), "content_block_start");
        assert_eq!(StreamEventType::ContentBlockDelta.to_string(), "content_block_delta");
        assert_eq!(StreamEventType::ContentBlockStop.to_string(), "content_block_stop");
        assert_eq!(StreamEventType::Ping.to_string(), "ping");
        assert_eq!(StreamEventType::Error.to_string(), "error");
    }

    #[test]
    fn test_stream_event_type_from_str() {
        assert_eq!("message_start".parse::<StreamEventType>().unwrap(), StreamEventType::MessageStart);
        assert_eq!("message_delta".parse::<StreamEventType>().unwrap(), StreamEventType::MessageDelta);
        assert_eq!("message_stop".parse::<StreamEventType>().unwrap(), StreamEventType::MessageStop);
        assert_eq!("content_block_start".parse::<StreamEventType>().unwrap(), StreamEventType::ContentBlockStart);
        assert_eq!("content_block_delta".parse::<StreamEventType>().unwrap(), StreamEventType::ContentBlockDelta);
        assert_eq!("content_block_stop".parse::<StreamEventType>().unwrap(), StreamEventType::ContentBlockStop);
        assert_eq!("ping".parse::<StreamEventType>().unwrap(), StreamEventType::Ping);
        assert_eq!("error".parse::<StreamEventType>().unwrap(), StreamEventType::Error);
    }

    #[test]
    fn test_stream_event_type_from_str_invalid() {
        let result = "invalid_event".parse::<StreamEventType>();
        assert!(result.is_err());
        
        let err = result.unwrap_err();
        match err {
            crate::error::AnthropicError::InvalidInput(msg) => {
                assert!(msg.contains("Unknown stream event type: invalid_event"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_stream_event_type_from_str_empty() {
        let result = "".parse::<StreamEventType>();
        assert!(result.is_err());
    }

    #[test]
    fn test_stream_event_type_equality() {
        assert_eq!(StreamEventType::Ping, StreamEventType::Ping);
        assert_ne!(StreamEventType::Ping, StreamEventType::Error);
    }

    #[test]
    fn test_stream_event_type_clone() {
        let event1 = StreamEventType::MessageStart;
        let event2 = event1.clone();
        assert_eq!(event1, event2);
    }

    #[test]
    fn test_stream_event_type_debug() {
        let debug_str = format!("{:?}", StreamEventType::ContentBlockDelta);
        assert!(debug_str.contains("ContentBlockDelta"));
    }

    #[test]
    fn test_stream_event_type_round_trip() {
        let events = [
            StreamEventType::MessageStart,
            StreamEventType::MessageDelta,
            StreamEventType::MessageStop,
            StreamEventType::ContentBlockStart,
            StreamEventType::ContentBlockDelta,
            StreamEventType::ContentBlockStop,
            StreamEventType::Ping,
            StreamEventType::Error,
        ];

        for event in &events {
            let string_form = event.to_string();
            let parsed = string_form.parse::<StreamEventType>().unwrap();
            assert_eq!(*event, parsed);
        }
    }
}

#[cfg(test)]
mod progress_callback_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_progress_callback_creation() {
        let progress = Arc::new(Mutex::new(Vec::new()));
        let progress_clone = Arc::clone(&progress);
        
        let callback: ProgressCallback = Box::new(move |current, total| {
            progress_clone.lock().unwrap().push((current, total));
        });

        // Test the callback
        callback(50, 100);
        callback(75, 100);
        
        let results = progress.lock().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (50, 100));
        assert_eq!(results[1], (75, 100));
    }

    #[test]
    fn test_progress_callback_zero_values() {
        let called = Arc::new(Mutex::new(false));
        let called_clone = Arc::clone(&called);
        
        let callback: ProgressCallback = Box::new(move |current, total| {
            *called_clone.lock().unwrap() = true;
            assert_eq!(current, 0);
            assert_eq!(total, 0);
        });

        callback(0, 0);
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_progress_callback_large_values() {
        let result = Arc::new(Mutex::new(None));
        let result_clone = Arc::clone(&result);
        
        let callback: ProgressCallback = Box::new(move |current, total| {
            *result_clone.lock().unwrap() = Some((current, total));
        });

        let large_current = u64::MAX - 1000;
        let large_total = u64::MAX;
        callback(large_current, large_total);
        
        let stored_result = result.lock().unwrap();
        assert_eq!(*stored_result, Some((large_current, large_total)));
    }
}