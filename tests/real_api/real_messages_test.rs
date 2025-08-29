//! Real API tests for Messages
//!
//! These tests use the actual Anthropic API and require valid credentials.
//! They are marked with the `real_api_tests` feature flag.

#[cfg(feature = "real_api_tests")]
mod real_api_tests {
    use threatflux::{Client, builders::MessageBuilder};
    use std::env;
    use dotenv::dotenv;

    fn setup_client() -> Option<Client> {
        dotenv().ok(); // Load .env file if present
        
        if env::var("ANTHROPIC_API_KEY").is_ok() && 
           env::var("RUN_REAL_API_TESTS").unwrap_or_default() == "true" {
            Client::from_env().ok()
        } else {
            None
        }
    }

    #[tokio::test]
    async fn test_real_basic_message() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(50)
            .user("Say 'Hello from Threatflux SDK!' and nothing else.")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_ok(), "Basic message should succeed: {:?}", response.err());
        
        let response = response.unwrap();
        assert!(!response.text().is_empty(), "Response should contain text");
        assert!(response.text().contains("Hello from Threatflux"), "Response should contain expected text");
        assert!(response.usage.input_tokens > 0, "Should have input token usage");
        assert!(response.usage.output_tokens > 0, "Should have output token usage");
    }

    #[tokio::test]
    async fn test_real_streaming_message() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(30)
            .user("Count from 1 to 5, each number on a new line")
            .stream()
            .build();

        let stream = client.messages().create_stream(request, None).await;
        assert!(stream.is_ok(), "Stream creation should succeed: {:?}", stream.err());
        
        let text = stream.unwrap().collect_text().await;
        assert!(text.is_ok(), "Stream collection should succeed: {:?}", text.err());
        
        let collected_text = text.unwrap();
        assert!(!collected_text.is_empty(), "Streamed text should not be empty");
        
        // Should contain numbers
        for i in 1..=5 {
            assert!(collected_text.contains(&i.to_string()), "Should contain number {}", i);
        }
    }

    #[tokio::test]
    async fn test_real_token_counting() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let test_message = "Hello, world! How are you today?";
        let result = client.messages()
            .count_tokens_simple("claude-3-5-haiku-20241022", test_message, None)
            .await;
            
        assert!(result.is_ok(), "Token counting should succeed: {:?}", result.err());
        
        let count = result.unwrap();
        assert!(count.input_tokens > 0, "Should have positive token count, got: {}", count.input_tokens);
        assert!(count.input_tokens < 50, "Token count should be reasonable for short message, got: {}", count.input_tokens);
    }

    #[tokio::test]
    async fn test_real_conversation() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(50)
            .conversation(&[
                (threatflux::models::common::Role::User, "My name is Alice"),
                (threatflux::models::common::Role::Assistant, "Hello Alice! Nice to meet you."),
                (threatflux::models::common::Role::User, "What's my name?"),
            ])
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_ok(), "Conversation should succeed: {:?}", response.err());
        
        let response = response.unwrap();
        let text = response.text().to_lowercase();
        assert!(text.contains("alice"), "Should remember the name Alice, got: {}", response.text());
    }

    #[tokio::test]
    async fn test_real_system_message() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(30)
            .system("You are a helpful assistant that always responds with exactly 3 words.")
            .user("How are you?")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_ok(), "System message should succeed: {:?}", response.err());
        
        let response = response.unwrap();
        let word_count = response.text().split_whitespace().count();
        assert!(word_count <= 5, "Should respond with roughly 3 words due to system prompt, got {} words: '{}'", word_count, response.text());
    }

    #[tokio::test]
    async fn test_real_temperature_variations() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        // Test with very low temperature (deterministic)
        let request_low = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(20)
            .temperature(0.0)
            .user("Complete this: The sky is")
            .build();

        let response_low = client.messages().create(request_low, None).await;
        assert!(response_low.is_ok(), "Low temperature message should succeed");

        // Test with higher temperature (more creative)
        let request_high = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(20)
            .temperature(0.9)
            .user("Complete this: The sky is")
            .build();

        let response_high = client.messages().create(request_high, None).await;
        assert!(response_high.is_ok(), "High temperature message should succeed");

        // Both should have responses
        assert!(!response_low.unwrap().text().is_empty());
        assert!(!response_high.unwrap().text().is_empty());
    }

    #[tokio::test]
    async fn test_real_error_handling() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        // Test with invalid model
        let request = MessageBuilder::new()
            .model("definitely-not-a-real-model-name-12345")
            .max_tokens(100)
            .user("Hello")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_err(), "Invalid model should cause error");
        
        // Should be a client error (400-level)
        if let Err(error) = response {
            assert!(!error.is_retryable(), "Invalid model error should not be retryable");
        }
    }

    #[tokio::test]
    async fn test_real_max_tokens_limit() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        // Test with very high max_tokens (should fail)
        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(999999) // Way too high
            .user("Hello")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_err(), "Excessive max_tokens should cause error");
    }

    #[tokio::test] 
    async fn test_real_stop_sequences() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(100)
            .stop_sequences(vec!["STOP".to_string()])
            .user("Count to 10, then say STOP, then count to 20")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_ok(), "Stop sequence test should succeed");
        
        let response = response.unwrap();
        let text = response.text();
        
        // Should stop at the stop sequence
        if text.contains("STOP") {
            // If STOP appears, it should be at or near the end
            let stop_pos = text.find("STOP").unwrap();
            let after_stop = &text[stop_pos + 4..].trim();
            assert!(after_stop.is_empty() || after_stop.len() < 10, 
                "Should stop generation at STOP sequence, but got: '{}'", text);
        }
    }

    #[tokio::test]
    async fn test_real_metadata() {
        let Some(client) = setup_client() else {
            println!("Skipping real API test - no API key or RUN_REAL_API_TESTS=true");
            return;
        };

        let request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(20)
            .metadata(serde_json::json!({"test_run": true, "test_id": "metadata_test"}))
            .user("Say hello")
            .build();

        let response = client.messages().create(request, None).await;
        assert!(response.is_ok(), "Message with metadata should succeed: {:?}", response.err());
        
        let response = response.unwrap();
        assert!(!response.text().is_empty(), "Should get response with metadata");
    }
}

// Placeholder tests that always pass when feature is disabled
#[cfg(not(feature = "real_api_tests"))]
mod placeholder_tests {
    #[test]
    fn real_api_tests_disabled() {
        println!("Real API tests are disabled. Enable with --features real_api_tests");
        assert!(true);
    }
}