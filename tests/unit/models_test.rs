//! Unit tests for Models modules
//!
//! Tests serialization/deserialization of all model types and their functionality.

use chrono::Utc;
use serde_json::{from_str, json, to_string};
use threatflux_anthropic_sdk::models::{
    admin::{ApiKey, MemberRole, Organization, UsageReport, Workspace, WorkspaceStatus},
    batch::{
        BatchRequestItem, MessageBatch, MessageBatchCreateRequest, MessageBatchStatus,
        RequestCounts,
    },
    common::{ContentBlock, ImageSource, Role, StopReason, ToolResultContent, Usage},
    file::{File, FileDownload, FilePurpose, FileStatus, FileUploadRequest},
    message::{Message, MessageRequest, MessageResponse, StreamEvent, SystemPrompt},
    model::{Model, ModelFamily, ModelListResponse, ModelSize},
};

#[cfg(test)]
mod common_models_tests {
    use super::*;

    #[test]
    fn test_content_block_text() {
        let block = ContentBlock::text("Hello, world!");

        assert_eq!(block.as_text(), Some("Hello, world!"));
        assert!(block.as_image().is_none());

        // Test serialization
        let json = to_string(&block).unwrap();
        let deserialized: ContentBlock = from_str(&json).unwrap();
        assert_eq!(deserialized.as_text(), Some("Hello, world!"));
    }

    #[test]
    fn test_content_block_image() {
        let image_source = ImageSource::base64(
            "image/jpeg",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==",
        );

        let block = ContentBlock::image(image_source.clone());

        assert!(block.as_text().is_none());
        assert!(block.as_image().is_some());
        assert_eq!(block.as_image().unwrap(), &image_source);

        // Test serialization
        let json = to_string(&block).unwrap();
        let deserialized: ContentBlock = from_str(&json).unwrap();
        assert_eq!(deserialized.as_image(), Some(&image_source));
    }

    #[test]
    fn test_content_block_tool_use() {
        let tool_use = ContentBlock::tool_use("tool_123", "calculator", json!({"x": 5, "y": 3}));

        assert!(tool_use.as_text().is_none());
        assert!(tool_use.as_image().is_none());

        if let ContentBlock::ToolUse { id, name, input } = &tool_use {
            assert_eq!(id, "tool_123");
            assert_eq!(name, "calculator");
            assert_eq!(input, &json!({"x": 5, "y": 3}));
        } else {
            panic!("Expected ToolUse variant");
        }

        // Test serialization
        let json = to_string(&tool_use).unwrap();
        let deserialized: ContentBlock = from_str(&json).unwrap();
        if let ContentBlock::ToolUse { name, .. } = deserialized {
            assert_eq!(name, "calculator");
        }
    }

    #[test]
    fn test_content_block_tool_result() {
        let tool_result = ContentBlock::tool_result("tool_123", Some("Result: 8".to_string()));

        if let ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error,
        } = &tool_result
        {
            assert_eq!(tool_use_id, "tool_123");
            assert_eq!(
                content,
                &Some(ToolResultContent::Text("Result: 8".to_string()))
            );
            assert_eq!(*is_error, Some(false));
        } else {
            panic!("Expected ToolResult variant");
        }

        // Test serialization
        let json = to_string(&tool_result).unwrap();
        let deserialized: ContentBlock = from_str(&json).unwrap();
        if let ContentBlock::ToolResult { content, .. } = deserialized {
            assert_eq!(
                content,
                Some(ToolResultContent::Text("Result: 8".to_string()))
            );
        }
    }

    #[test]
    fn test_usage() {
        let usage = Usage::new(100, 50);
        assert_eq!(usage.input_tokens, 100);
        assert_eq!(usage.output_tokens, 50);
        assert_eq!(usage.total_tokens(), 150);

        // Test serialization
        let json = to_string(&usage).unwrap();
        let deserialized: Usage = from_str(&json).unwrap();
        assert_eq!(deserialized.total_tokens(), 150);
    }

    #[test]
    fn test_role() {
        let user = Role::User;
        let assistant = Role::Assistant;

        assert_ne!(user, assistant);

        // Test serialization
        assert_eq!(to_string(&user).unwrap(), "\"user\"");
        assert_eq!(to_string(&assistant).unwrap(), "\"assistant\"");

        // Test deserialization
        assert_eq!(from_str::<Role>("\"user\"").unwrap(), Role::User);
        assert_eq!(from_str::<Role>("\"assistant\"").unwrap(), Role::Assistant);
    }

    #[test]
    fn test_stop_reason() {
        let reasons = vec![
            StopReason::EndTurn,
            StopReason::MaxTokens,
            StopReason::StopSequence,
            StopReason::ToolUse,
        ];

        for reason in reasons {
            let json = to_string(&reason).unwrap();
            let deserialized: StopReason = from_str(&json).unwrap();
            assert_eq!(deserialized, reason);
        }
    }

    #[test]
    fn test_image_source() {
        let image = ImageSource::base64(
            "image/png",
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8/5+hHgAHggJ/PchI7wAAAABJRU5ErkJggg==",
        );

        // Test serialization round trip
        let json = to_string(&image).unwrap();
        let deserialized: ImageSource = from_str(&json).unwrap();
        assert_eq!(deserialized, image);
        let ImageSource::Base64 { media_type, data } = deserialized else {
            panic!("Expected base64 image source");
        };
        assert_eq!(media_type, "image/png");
        assert!(!data.is_empty());
    }
}

#[cfg(test)]
mod message_models_tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let user_msg = Message::user("Hello, Claude!");
        assert_eq!(user_msg.role, Role::User);
        assert_eq!(user_msg.text(), "Hello, Claude!");

        let assistant_msg = Message::assistant("Hello back!");
        assert_eq!(assistant_msg.role, Role::Assistant);
        assert_eq!(assistant_msg.text(), "Hello back!");
    }

    #[test]
    fn test_message_with_multiple_content() {
        let message = Message {
            role: Role::User,
            content: vec![
                ContentBlock::text("Describe this image:"),
                ContentBlock::image(ImageSource::base64("image/jpeg", "base64data")),
            ],
            metadata: None,
        };

        assert_eq!(message.content.len(), 2);
        assert_eq!(message.text(), "Describe this image:");
    }

    #[test]
    fn test_message_request() {
        let request = MessageRequest::new()
            .model("claude-haiku-4-5")
            .max_tokens(100)
            .add_user_message("Hello")
            .system("You are helpful")
            .temperature(0.7)
            .top_p(0.9)
            .top_k(50)
            .add_stop_sequence("STOP")
            .stream(false)
            .metadata(
                threatflux_anthropic_sdk::models::common::Metadata::new()
                    .with_custom("test", json!(true)),
            );

        // Test serialization
        let json = to_string(&request).unwrap();
        let deserialized: MessageRequest = from_str(&json).unwrap();
        assert_eq!(deserialized.model, "claude-haiku-4-5");
        assert_eq!(deserialized.max_tokens, 100);
        assert!(
            matches!(deserialized.system, Some(SystemPrompt::Text(ref s)) if s == "You are helpful")
        );
    }

    #[test]
    fn test_message_response() {
        let response = MessageResponse {
            id: "msg_123".to_string(),
            object_type: "message".to_string(),
            created_at: Utc::now(),
            model: "claude-haiku-4-5".to_string(),
            role: Role::Assistant,
            content: vec![ContentBlock::text("Hello!")],
            stop_reason: Some(StopReason::EndTurn),
            stop_sequence: None,
            stop_details: None,
            usage: Usage::new(10, 5),
            container: None,
        };

        assert_eq!(response.text(), "Hello!");
        assert_eq!(response.usage.total_tokens(), 15);

        // Test serialization
        let json = to_string(&response).unwrap();
        let deserialized: MessageResponse = from_str(&json).unwrap();
        assert_eq!(deserialized.text(), "Hello!");
    }

    #[test]
    fn test_stream_events() {
        let events = vec![
            StreamEvent::MessageStart {
                message: MessageResponse {
                    id: "msg_123".to_string(),
                    object_type: "message".to_string(),
                    created_at: Utc::now(),
                    model: "claude-haiku-4-5".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    stop_reason: None,
                    stop_sequence: None,
                    stop_details: None,
                    usage: Usage::new(10, 0),
                    container: None,
                },
            },
            StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            },
            StreamEvent::ContentBlockStop { index: 0 },
            StreamEvent::MessageStop,
        ];

        for event in events {
            let json = to_string(&event).unwrap();
            let deserialized: StreamEvent = from_str(&json).unwrap();

            // Basic validation that deserialization worked
            match (&event, &deserialized) {
                (StreamEvent::MessageStart { .. }, StreamEvent::MessageStart { .. }) => {}
                (
                    StreamEvent::ContentBlockStart { index: i1, .. },
                    StreamEvent::ContentBlockStart { index: i2, .. },
                ) => {
                    assert_eq!(i1, i2);
                }
                (StreamEvent::MessageStop, StreamEvent::MessageStop) => {}
                _ => {} // Other variants
            }
        }
    }
}

#[cfg(test)]
mod model_info_tests {
    use super::*;

    #[test]
    fn test_model_family_parsing() {
        assert_eq!(
            "claude-3-5-haiku-20241022".parse::<ModelFamily>().unwrap(),
            ModelFamily::Claude35
        );
        assert_eq!(
            "claude-3-5-sonnet-20240620".parse::<ModelFamily>().unwrap(),
            ModelFamily::Claude35
        );
        assert_eq!(
            "claude-3-opus-20240229".parse::<ModelFamily>().unwrap(),
            ModelFamily::Claude3
        );
        assert_eq!(
            "claude-3-sonnet-20240229".parse::<ModelFamily>().unwrap(),
            ModelFamily::Claude3
        );
        assert_eq!(
            "claude-2.1".parse::<ModelFamily>().unwrap(),
            ModelFamily::Legacy
        );
        assert_eq!(
            "claude-2.0".parse::<ModelFamily>().unwrap(),
            ModelFamily::Legacy
        );
        assert_eq!(
            "claude-opus-4-8".parse::<ModelFamily>().unwrap(),
            ModelFamily::Claude4
        );

        // Non-claude model names map to Unknown.
        assert_eq!(
            "invalid-model".parse::<ModelFamily>().unwrap(),
            ModelFamily::Unknown
        );
    }

    #[test]
    fn test_model_size_parsing() {
        assert_eq!(
            "claude-3-5-haiku-20241022".parse::<ModelSize>().unwrap(),
            ModelSize::Haiku
        );
        assert_eq!(
            "claude-3-5-sonnet-20240620".parse::<ModelSize>().unwrap(),
            ModelSize::Sonnet
        );
        assert_eq!(
            "claude-3-opus-20240229".parse::<ModelSize>().unwrap(),
            ModelSize::Opus
        );

        // Names without a known size tier map to Unknown.
        assert_eq!(
            "invalid-model".parse::<ModelSize>().unwrap(),
            ModelSize::Unknown
        );
    }

    #[test]
    fn test_model_struct() {
        let model = Model {
            id: "claude-3-5-haiku-20241022".to_string(),
            object_type: "model".to_string(),
            display_name: "Claude 3.5 Haiku".to_string(),
            description: Some("Fast and accurate".to_string()),
            max_input_tokens: Some(200000),
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

        assert_eq!(model.family(), ModelFamily::Claude35);
        assert_eq!(model.size(), ModelSize::Haiku);
        assert!(model.supports_vision());
        assert!(model.supports_tools());
        assert!(!model.is_deprecated());

        // Test serialization
        let json = to_string(&model).unwrap();
        let deserialized: Model = from_str(&json).unwrap();
        assert_eq!(deserialized.id, "claude-3-5-haiku-20241022");
    }

    #[test]
    fn test_model_list_response() {
        let models = ModelListResponse {
            data: vec![Model {
                id: "claude-3-5-haiku-20241022".to_string(),
                object_type: "model".to_string(),
                display_name: "Claude 3.5 Haiku".to_string(),
                description: None,
                max_input_tokens: Some(200000),
                max_tokens: Some(200000),
                max_output_tokens: Some(8192),
                input_cost_per_token: Some(0.00025),
                output_cost_per_token: Some(0.00125),
                capabilities: Some(vec!["vision".to_string()]),
                created_at: Utc::now(),
                updated_at: Utc::now(),
                deprecated: Some(false),
                deprecation_date: None,
            }],
            has_more: false,
            first_id: Some("claude-3-5-haiku-20241022".to_string()),
            last_id: Some("claude-3-5-haiku-20241022".to_string()),
        };

        // Test serialization
        let json = to_string(&models).unwrap();
        let deserialized: ModelListResponse = from_str(&json).unwrap();
        assert_eq!(deserialized.data.len(), 1);
        assert_eq!(deserialized.data[0].id, "claude-3-5-haiku-20241022");
    }
}

#[cfg(test)]
mod batch_models_tests {
    use super::*;

    #[test]
    fn test_batch_status() {
        let statuses = vec![
            MessageBatchStatus::InProgress,
            MessageBatchStatus::Cancelled,
            MessageBatchStatus::Completed,
        ];

        for status in statuses {
            let json = to_string(&status).unwrap();
            let deserialized: MessageBatchStatus = from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_message_batch() {
        let batch = MessageBatch {
            id: "batch_123".to_string(),
            object_type: "message_batch".to_string(),
            processing_status: MessageBatchStatus::InProgress,
            request_counts: RequestCounts {
                processing: 5,
                completed: 3,
                failed: 1,
                cancelled: 0,
                expired: 0,
                total: 9,
            },
            created_at: Utc::now(),
            in_progress_at: None,
            completed_at: None,
            cancelled_at: None,
            failed_at: None,
            expires_at: Utc::now() + chrono::Duration::hours(24),
            error: None,
            results_file_id: None,
            results_url: Some(
                "https://api.anthropic.com/v1/message_batches/batch_123/results".to_string(),
            ),
        };

        assert_eq!(batch.request_counts.total, 9);
        assert_eq!(batch.processing_status, MessageBatchStatus::InProgress);

        // Test serialization
        let json = to_string(&batch).unwrap();
        let deserialized: MessageBatch = from_str(&json).unwrap();
        assert_eq!(deserialized.id, "batch_123");
    }

    #[test]
    fn test_batch_request() {
        let message_request = MessageRequest::new()
            .model("claude-haiku-4-5")
            .max_tokens(100)
            .add_user_message("Hello")
            .stream(false);

        let batch_request = BatchRequestItem {
            custom_id: "req_123".to_string(),
            params: message_request,
        };

        assert_eq!(batch_request.custom_id, "req_123");
        assert_eq!(batch_request.params.model, "claude-haiku-4-5");

        // Test serialization
        let json = to_string(&batch_request).unwrap();
        let deserialized: BatchRequestItem = from_str(&json).unwrap();
        assert_eq!(deserialized.custom_id, "req_123");
    }

    #[test]
    fn test_batch_create_request() {
        let create_request = MessageBatchCreateRequest {
            requests: vec![BatchRequestItem {
                custom_id: "req_1".to_string(),
                params: MessageRequest::new()
                    .model("claude-haiku-4-5")
                    .max_tokens(100)
                    .add_user_message("First message")
                    .stream(false),
            }],
        };

        assert_eq!(create_request.requests.len(), 1);

        // Test serialization
        let json = to_string(&create_request).unwrap();
        let deserialized: MessageBatchCreateRequest = from_str(&json).unwrap();
        assert_eq!(deserialized.requests.len(), 1);
    }
}

#[cfg(test)]
mod file_models_tests {
    use super::*;

    #[test]
    fn test_file_status() {
        let statuses = vec![FileStatus::Processing, FileStatus::Ready, FileStatus::Error];

        for status in statuses {
            let json = to_string(&status).unwrap();
            let deserialized: FileStatus = from_str(&json).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_file_purpose() {
        let purposes = vec![FilePurpose::UserData, FilePurpose::AssistantData];

        for purpose in purposes {
            let json = to_string(&purpose).unwrap();
            let deserialized: FilePurpose = from_str(&json).unwrap();
            assert_eq!(deserialized, purpose);
        }
    }

    #[test]
    fn test_file_struct() {
        let file = File {
            id: "file_123".to_string(),
            object_type: "file".to_string(),
            filename: "test.txt".to_string(),
            size_bytes: 1024,
            mime_type: "text/plain".to_string(),
            purpose: "user_data".to_string(),
            status: Some(FileStatus::Ready),
            created_at: Utc::now(),
            updated_at: None,
            error: None,
        };

        // Test serialization
        let json = to_string(&file).unwrap();
        let deserialized: File = from_str(&json).unwrap();
        assert_eq!(deserialized.id, "file_123");
        assert_eq!(deserialized.filename, "test.txt");
    }

    #[test]
    fn test_file_upload_request() {
        let upload_request =
            FileUploadRequest::new(b"Hello, world!".to_vec(), "greeting.txt", "text/plain")
                .purpose("user_data");

        assert_eq!(upload_request.filename, "greeting.txt");
        assert_eq!(upload_request.mime_type, "text/plain");
        assert_eq!(upload_request.content, b"Hello, world!");
        assert_eq!(upload_request.purpose, "user_data");
    }

    #[test]
    fn test_file_download() {
        let download = FileDownload::new(
            b"File content here".to_vec(),
            "text/plain".to_string(),
            "downloaded.txt".to_string(),
        );

        assert_eq!(download.content, b"File content here");
        assert_eq!(download.filename, "downloaded.txt");
        assert_eq!(download.content_type, "text/plain");

        // Test serialization
        let json = to_string(&download).unwrap();
        let deserialized: FileDownload = from_str(&json).unwrap();
        assert_eq!(deserialized.filename, "downloaded.txt");
    }
}

#[cfg(test)]
mod admin_models_tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_organization() {
        let org = Organization {
            object_type: Some("organization".to_string()),
            id: "org_123".to_string(),
            name: "Test Organization".to_string(),
            display_name: Some("Test Org".to_string()),
            description: None,
            settings: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            extra: HashMap::new(),
        };

        // Test serialization
        let json = to_string(&org).unwrap();
        let deserialized: Organization = from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Organization");
    }

    #[test]
    fn test_workspace() {
        let workspace = Workspace {
            object_type: Some("workspace".to_string()),
            id: "ws_123".to_string(),
            name: "Test Workspace".to_string(),
            display_name: Some("Test WS".to_string()),
            display_color: None,
            description: None,
            settings: None,
            status: Some(WorkspaceStatus::Active),
            data_residency: None,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
            archived_at: None,
            extra: HashMap::new(),
        };

        // Test serialization
        let json = to_string(&workspace).unwrap();
        let deserialized: Workspace = from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Workspace");
    }

    #[test]
    fn test_member_role() {
        let roles = vec![
            MemberRole::Owner,
            MemberRole::Admin,
            MemberRole::Member,
            MemberRole::Viewer,
        ];

        for role in roles {
            let json = to_string(&role).unwrap();
            let deserialized: MemberRole = from_str(&json).unwrap();
            assert_eq!(deserialized, role);
        }
    }

    #[test]
    fn test_usage_report() {
        let usage = UsageReport {
            input_tokens: 10000,
            output_tokens: 5000,
            request_count: 10,
            usage_by_period: None,
            usage_by_model: None,
            cost: None,
        };

        assert_eq!(usage.input_tokens + usage.output_tokens, 15000);

        // Test serialization
        let json = to_string(&usage).unwrap();
        let deserialized: UsageReport = from_str(&json).unwrap();
        assert_eq!(
            deserialized.input_tokens + deserialized.output_tokens,
            15000
        );
        assert_eq!(deserialized.request_count, 10);
    }

    #[test]
    fn test_api_key() {
        let api_key = ApiKey {
            object_type: Some("api_key".to_string()),
            id: "key_123".to_string(),
            name: "Test Key".to_string(),
            created_by: None,
            workspace_id: None,
            description: None,
            partial_key_hint: Some("sk-ant-...abc123".to_string()),
            status: None,
            permissions: None,
            rate_limits: None,
            created_at: Some(Utc::now()),
            last_used_at: Some(Utc::now()),
            expires_at: None,
            extra: HashMap::new(),
        };

        // Test serialization
        let json = to_string(&api_key).unwrap();
        let deserialized: ApiKey = from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Test Key");
    }
}
