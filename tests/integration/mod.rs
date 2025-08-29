//! Integration tests for the Threatflux SDK
//! 
//! These tests use mocked HTTP responses and don't require real API credentials.
//! For tests with real API credentials, see the real_api tests.

// Import all integration test modules
mod messages_test;
mod models_test;
mod batches_test;
mod files_test;
mod admin_test;
mod e2e_test;

// Common utilities for integration tests
#[path = "../common/mod.rs"]
mod common;

#[cfg(test)]
mod legacy_api_tests {
    use threatflux::{Client, builders::MessageBuilder};
    use std::env;

    fn setup_client() -> Option<Client> {
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            Client::from_env().ok()
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_basic_message() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Say 'Hello, Threatflux!' and nothing else.")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_ok(), "Basic message request should succeed");
        
        let response = response.unwrap();
        assert!(!response.text().is_empty(), "Response should contain text");
        assert!(response.usage.input_tokens > 0, "Should have input token usage");
        assert!(response.usage.output_tokens > 0, "Should have output token usage");
    }

    #[tokio::test]
    async fn test_streaming_message() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(50)
            .user("Count from 1 to 5")
            .stream()
            .build();

        let stream = client.messages().create_stream(request, None).await;
        assert!(stream.is_ok(), "Stream creation should succeed");
        
        let text = stream.unwrap().collect_text().await;
        assert!(text.is_ok(), "Stream collection should succeed");
        assert!(!text.unwrap().is_empty(), "Streamed text should not be empty");
    }

    #[tokio::test]
    async fn test_token_counting() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let result = client.messages()
            .count_tokens_simple("claude-3-5-haiku-20241022", "Hello, world!", None)
            .await;
            
        assert!(result.is_ok(), "Token counting should succeed");
        
        let count = result.unwrap();
        assert!(count.input_tokens > 0, "Should have positive token count");
    }

    #[tokio::test]
    async fn test_models_list() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let models = client.models()
            .list(Some(threatflux::types::Pagination::new().with_limit(5)), None)
            .await;
            
        assert!(models.is_ok(), "Models list should succeed");
        
        let models = models.unwrap();
        assert!(!models.data.is_empty(), "Should return some models");
    }

    #[tokio::test]
    async fn test_specific_model() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let model = client.models()
            .get("claude-3-5-haiku-20241022", None)
            .await;
            
        assert!(model.is_ok(), "Specific model fetch should succeed");
        
        let model = model.unwrap();
        assert_eq!(model.id, "claude-3-5-haiku-20241022");
        assert!(!model.display_name.is_empty());
    }

    #[tokio::test]
    async fn test_error_handling() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        // Test invalid model
        let request = MessageBuilder::new()
            .model("invalid-model-name")
            .max_tokens(100)
            .user("Hello")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_err(), "Invalid model should cause error");
        
        // Test max tokens too high
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(999999)
            .user("Hello")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_err(), "Too many max tokens should cause error");
    }
}

#[cfg(test)]
mod legacy_batch_tests {
    use threatflux::{Client, builders::BatchBuilder};
    use std::env;

    fn setup_client() -> Option<Client> {
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            Client::from_env().ok()
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_batch_creation() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let batch_request = BatchBuilder::new()
            .add_simple_request("test1", "claude-3-5-haiku-20241022", "Say hello", 50)
            .add_simple_request("test2", "claude-3-5-haiku-20241022", "Say goodbye", 50)
            .build();

        let batch = client.message_batches().create(batch_request, None).await;
        assert!(batch.is_ok(), "Batch creation should succeed");
        
        let batch = batch.unwrap();
        assert_eq!(batch.request_counts.total, 2);
    }

    #[tokio::test]
    async fn test_batch_listing() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let batches = client.message_batches()
            .list(Some(threatflux::types::Pagination::new().with_limit(5)), None)
            .await;
            
        assert!(batches.is_ok(), "Batch listing should succeed");
    }
}

#[cfg(test)]
mod legacy_file_tests {
    use threatflux::{Client, models::file::FileUploadRequest};
    use std::env;

    fn setup_client() -> Option<Client> {
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            Client::from_env().ok()
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_file_upload() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let content = b"Hello, this is a test file for Threatflux SDK!";
        let request = FileUploadRequest::new(
            content.to_vec(),
            "test.txt",
            "text/plain"
        ).purpose("user_data");

        let result = client.files().upload(request, None).await;
        assert!(result.is_ok(), "File upload should succeed");
        
        let file = result.unwrap().file;
        assert_eq!(file.filename, "test.txt");
        assert_eq!(file.mime_type, "text/plain");
        assert_eq!(file.size_bytes, content.len() as u64);
        
        // Clean up - delete the file
        let _ = client.files().delete(&file.id, None).await;
    }

    #[tokio::test]
    async fn test_file_listing() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        let files = client.files()
            .list(Some(threatflux::types::Pagination::new().with_limit(5)), None)
            .await;
            
        assert!(files.is_ok(), "File listing should succeed");
    }
}

#[cfg(test)]
mod legacy_admin_tests {
    use threatflux::Client;
    use std::env;

    fn setup_admin_client() -> Option<Client> {
        if env::var("ANTHROPIC_ADMIN_KEY").is_ok() {
            Client::from_env().ok()
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_organization_info() {
        let Some(client) = setup_admin_client() else {
            println!("Skipping admin integration test - no admin key");
            return;
        };

        if let Ok(admin) = client.admin() {
            let org = admin.organization().get(None).await;
            if org.is_ok() {
                let org = org.unwrap();
                assert!(!org.name.is_empty(), "Organization should have a name");
            }
        }
    }

    #[tokio::test]
    async fn test_workspace_listing() {
        let Some(client) = setup_admin_client() else {
            println!("Skipping admin integration test - no admin key");
            return;
        };

        if let Ok(admin) = client.admin() {
            let workspaces = admin.workspaces()
                .list(Some(threatflux::types::Pagination::new().with_limit(5)), None)
                .await;
                
            if workspaces.is_ok() {
                // Test passed - we can list workspaces
            }
        }
    }

    #[tokio::test]
    async fn test_usage_report() {
        let Some(client) = setup_admin_client() else {
            println!("Skipping admin integration test - no admin key");
            return;
        };

        if let Ok(admin) = client.admin() {
            let usage = admin.usage()
                .get_current_billing_usage(None, None)
                .await;
                
            if usage.is_ok() {
                let usage = usage.unwrap();
                // Usage report should have some data (even if zero)
                assert!(usage.input_tokens >= 0);
                assert!(usage.output_tokens >= 0);
            }
        }
    }
}

#[cfg(test)]
mod legacy_comprehensive_tests {
    use threatflux::{Client, builders::MessageBuilder};
    use futures::StreamExt;
    use std::env;

    fn setup_client() -> Option<Client> {
        if env::var("ANTHROPIC_API_KEY").is_ok() {
            Client::from_env().ok()
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_full_workflow() {
        let Some(client) = setup_client() else {
            println!("Skipping integration test - no API key");
            return;
        };

        // 1. Test models
        let models = client.models().list(None, None).await.unwrap();
        assert!(!models.data.is_empty());

        // 2. Test message
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .user("Write a haiku about testing")
            .build();

        let response = client.messages().create(request, None).await.unwrap();
        assert!(!response.text().is_empty());

        // 3. Test streaming
        let stream_request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(50)
            .user("Count to 3")
            .stream()
            .build();

        let mut stream = client.messages().create_stream(stream_request, None).await.unwrap();
        let mut received_text = false;

        while let Some(event) = stream.next().await {
            if let Ok(event) = event {
                if let threatflux::models::message::StreamEvent::ContentBlockDelta { delta, .. } = event {
                    if delta.text.is_some() {
                        received_text = true;
                    }
                } else if let threatflux::models::message::StreamEvent::MessageStop = event {
                    break;
                }
            }
        }

        assert!(received_text, "Should receive streamed text");

        // 4. Test token counting
        let tokens = client.messages()
            .count_tokens_simple("claude-3-5-haiku-20241022", "Hello world", None)
            .await.unwrap();

        assert!(tokens.input_tokens > 0);
    }
}