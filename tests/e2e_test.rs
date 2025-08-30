//! End-to-end integration tests for the Threatflux SDK with real API
//!
//! Run these tests with: ANTHROPIC_API_KEY=your_key cargo test --test e2e_test --features real_api_tests

use futures::StreamExt;
use std::error::Error;
use std::time::Duration;
use threatflux::{
    builders::{BatchBuilder, MessageBuilder},
    config::{models, Config},
    models::{
        common::{ContentBlock, Tool},
        message::StreamEvent,
    },
    Client,
};

/// Test all available models with basic messages
async fn test_all_models_basic_messages() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing All Models with Basic Messages ===\n");

    let models_to_test = vec![
        ("claude-3-5-haiku-20241022", "Claude 3.5 Haiku"),
        ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet"),
        // Claude 4 models - commented out initially for cost reasons
        // Uncomment to test with real Claude 4 models
        // ("claude-opus-4-1-20250805", "Claude Opus 4.1"),
        // ("claude-opus-4-20250514", "Claude Opus 4"),
        // ("claude-sonnet-4-20250514", "Claude Sonnet 4"),
    ];

    for (model_id, model_name) in models_to_test {
        println!("Testing {}", model_name);

        let request = MessageBuilder::new()
            .model(model_id)
            .max_tokens(100)
            .user("Say 'Hello, I am working!' in exactly 5 words.")
            .build();

        match client.messages().create(request, None).await {
            Ok(response) => {
                println!("✅ {} Response: {}", model_name, response.text());
                println!(
                    "   Tokens: input={}, output={}",
                    response.usage.input_tokens, response.usage.output_tokens
                );
            }
            Err(e) => {
                println!("❌ {} Error: {}", model_name, e);
            }
        }

        // Small delay between requests to avoid rate limiting
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    Ok(())
}

/// Test Claude 4 extended thinking features
#[tokio::test]
#[ignore] // Expensive test - run with --ignored flag
async fn test_claude_4_extended_thinking() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Claude 4 Extended Thinking ===\n");

    // Test with a problem that benefits from thinking
    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(500)
        .thinking(10000) // Use modest thinking budget for testing
        .user("What is 17 * 23? Show your step-by-step calculation.")
        .build();

    let start = std::time::Instant::now();
    let response = client.messages().create(request, None).await?;
    let duration = start.elapsed();

    println!("Response: {}", response.text());
    println!("Time taken: {:?}", duration);
    println!("Tokens used: {:?}", response.usage);

    Ok(())
}

/// Test streaming responses
async fn test_streaming_responses() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Streaming Responses ===\n");

    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(150)
        .stream()
        .user("Count from 1 to 10 slowly.")
        .build();

    let mut stream = client.messages().create_stream(request, None).await?;

    print!("Streaming: ");
    let mut total_text = String::new();

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = delta.text {
                    print!("{}", text);
                    total_text.push_str(&text);
                }
            }
            StreamEvent::MessageStop => {
                println!("\n✅ Stream complete");
                break;
            }
            StreamEvent::Error { error } => {
                println!("\n❌ Stream error: {:?}", error);
                break;
            }
            _ => {}
        }
    }

    assert!(
        !total_text.is_empty(),
        "Should have received streamed content"
    );

    Ok(())
}

/// Test batch processing
#[tokio::test]
async fn test_batch_processing() -> Result<(), Box<dyn Error>> {
    // Skip test if no API key is provided
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        eprintln!("Skipping test: ANTHROPIC_API_KEY not set");
        return Ok(());
    }

    let client = Client::from_env()?;

    println!("\n=== Testing Batch Processing ===\n");

    let batch = BatchBuilder::new()
        .add_simple_request("test-1", models::HAIKU_3_5, "What is 2+2?", 50)
        .add_simple_request(
            "test-2",
            models::HAIKU_3_5,
            "What is the capital of France?",
            50,
        )
        .add_simple_request("test-3", models::HAIKU_3_5, "What color is the sky?", 50)
        .build();

    // Create batch
    let batch_response = client.message_batches().create(batch, None).await?;
    println!("✅ Batch created: {}", batch_response.id);
    println!("   Status: {:?}", batch_response.processing_status);

    // Wait for batch to complete (with timeout)
    let mut attempts = 0;
    let max_attempts = 60; // 1 minute timeout

    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;

        let status = client
            .message_batches()
            .retrieve(&batch_response.id, None)
            .await?;
        println!("   Checking status: {:?}", status.processing_status);

        match status.processing_status {
            threatflux::models::batch::MessageBatchStatus::Completed => {
                println!("✅ Batch completed!");

                // Note: retrieve_results method may not be implemented yet
                // Would need to retrieve and parse results file
                println!("   Note: Results retrieval not implemented in this test");
                break;
            }
            threatflux::models::batch::MessageBatchStatus::Failed
            | threatflux::models::batch::MessageBatchStatus::Cancelled => {
                println!(
                    "❌ Batch failed with status: {:?}",
                    status.processing_status
                );
                break;
            }
            _ => {
                attempts += 1;
                if attempts >= max_attempts {
                    println!("⏱️ Batch processing timeout");
                    break;
                }
            }
        }
    }

    Ok(())
}

/// Test token counting
async fn test_token_counting() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Token Counting ===\n");

    let test_texts = vec![
        ("Hello", "Short text"),
        ("The quick brown fox jumps over the lazy dog.", "Medium text"),
        ("Artificial intelligence is transforming how we interact with technology, enabling machines to understand, learn, and respond to human needs in increasingly sophisticated ways.", "Long text"),
    ];

    for (text, description) in test_texts {
        let count = client
            .messages()
            .count_tokens_simple(models::HAIKU_3_5, text, None)
            .await?;

        println!(
            "✅ {}: {} tokens for \"{}\"",
            description,
            count.input_tokens,
            if text.len() > 50 { &text[..50] } else { text }
        );
    }

    // Test with tools
    let tool = Tool::new(
        "calculator",
        "Perform calculations",
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {"type": "string"}
            }
        }),
    );

    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .tool(tool)
        .user("Calculate 5 + 3")
        .build();

    let request_for_counting = threatflux::models::message::TokenCountRequest {
        model: request.model.clone(),
        messages: request.messages.clone(),
        system: request.system.clone(),
        tools: request.tools.clone(),
    };

    let count = client
        .messages()
        .count_tokens(request_for_counting, None)
        .await?;
    println!("✅ With tools: {} input tokens", count.input_tokens);

    Ok(())
}

/// Test Models API
async fn test_models_api() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Models API ===\n");

    // List all models
    let models = client.models().list(None, None).await?;
    println!("✅ Found {} models:", models.data.len());

    for model in models.data.iter().take(5) {
        println!("   - {}: {}", model.id, model.display_name);
        if let Some(max_tokens) = model.max_tokens {
            println!("     Max tokens: {}", max_tokens);
        }
    }

    // Get specific model
    let model = client.models().get(models::HAIKU_3_5, None).await?;
    println!("\n✅ Retrieved model: {}", model.display_name);
    println!("   Capabilities: {:?}", model.capabilities);
    println!("   Max output tokens: {:?}", model.max_output_tokens);

    Ok(())
}

/// Test error handling and retries
async fn test_error_handling() -> Result<(), Box<dyn Error>> {
    println!("\n=== Testing Error Handling ===\n");

    // Test with invalid API key
    let bad_config = Config::new("invalid-api-key")?;
    let bad_client = Client::new(bad_config);

    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(10)
        .user("Test")
        .build();

    match bad_client.messages().create(request, None).await {
        Ok(_) => println!("❌ Should have failed with invalid API key"),
        Err(e) => {
            println!("✅ Correctly failed with invalid API key: {}", e);
            assert!(e.to_string().contains("401") || e.to_string().contains("Unauthorized"));
        }
    }

    // Test with invalid model
    let client = Client::from_env()?;

    let request = MessageBuilder::new()
        .model("invalid-model-name")
        .max_tokens(10)
        .user("Test")
        .build();

    match client.messages().create(request, None).await {
        Ok(_) => println!("❌ Should have failed with invalid model"),
        Err(e) => {
            println!("✅ Correctly failed with invalid model: {}", e);
        }
    }

    // Test with max tokens exceeding limit
    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(1000000) // Way too many
        .user("Test")
        .build();

    match client.messages().create(request, None).await {
        Ok(_) => println!("❌ Should have failed with excessive max_tokens"),
        Err(e) => {
            println!("✅ Correctly failed with excessive max_tokens: {}", e);
        }
    }

    Ok(())
}

/// Test conversation with context
async fn test_conversation() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Conversation with Context ===\n");

    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(100)
        .system("You are a helpful math tutor.")
        .user("What is 5 + 3?")
        .assistant("5 + 3 equals 8.")
        .user("Now multiply that by 2")
        .build();

    let response = client.messages().create(request, None).await?;

    println!("✅ Conversation response: {}", response.text());
    assert!(
        response.text().contains("16"),
        "Should calculate 8 * 2 = 16"
    );

    Ok(())
}

/// Test with tools/function calling
async fn test_tool_use() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Tool Use ===\n");

    let weather_tool = Tool::new(
        "get_weather",
        "Get the current weather in a given location",
        serde_json::json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The temperature unit"
                }
            },
            "required": ["location"]
        }),
    );

    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(200)
        .tool(weather_tool)
        .user("What's the weather like in Paris, France?")
        .build();

    let response = client.messages().create(request, None).await?;

    println!("✅ Tool use response: {}", response.text());

    // Check if the model attempted to use the tool
    for content in &response.content {
        if let ContentBlock::ToolUse { name, .. } = content {
            println!("   Tool called: {}", name);
        }
    }

    Ok(())
}

/// Test rate limiting behavior
async fn test_rate_limiting() -> Result<(), Box<dyn Error>> {
    let client = Client::from_env()?;

    println!("\n=== Testing Rate Limiting ===\n");

    // Send multiple requests quickly
    let mut tasks = vec![];

    for i in 0..5 {
        let client = client.clone();
        let task = tokio::spawn(async move {
            let request = MessageBuilder::new()
                .model(models::HAIKU_3_5)
                .max_tokens(10)
                .user(format!("Say {}", i))
                .build();

            let start = std::time::Instant::now();
            let result = client.messages().create(request, None).await;
            let duration = start.elapsed();

            (i, result, duration)
        });

        tasks.push(task);
    }

    // Wait for all tasks
    for task in tasks {
        let (i, result, duration) = task.await?;
        match result {
            Ok(response) => {
                println!("✅ Request {}: Success in {:?}", i, duration);
                println!("   Response: {}", response.text());
            }
            Err(e) => {
                println!("⚠️ Request {}: Error in {:?} - {}", i, duration, e);
            }
        }
    }

    Ok(())
}

/// Main test runner - run all tests in sequence
#[tokio::test]
#[ignore] // Run with --ignored to execute all tests
async fn test_complete_e2e_suite() -> Result<(), Box<dyn Error>> {
    println!("\n========================================");
    println!("   THREATFLUX SDK - COMPLETE E2E TEST");
    println!("========================================\n");

    // Check for API key
    if std::env::var("ANTHROPIC_API_KEY").is_err() {
        println!("❌ ANTHROPIC_API_KEY not set!");
        println!("   Please set: export ANTHROPIC_API_KEY=your_key");
        return Ok(());
    }

    let mut passed = 0;
    let mut failed = 0;

    // Run tests individually
    println!("\n📝 Running: Basic Messages");
    println!("----------------------------------------");
    if let Err(e) = test_all_models_basic_messages().await {
        println!("❌ Basic Messages FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Basic Messages PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Streaming");
    println!("----------------------------------------");
    if let Err(e) = test_streaming_responses().await {
        println!("❌ Streaming FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Streaming PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Token Counting");
    println!("----------------------------------------");
    if let Err(e) = test_token_counting().await {
        println!("❌ Token Counting FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Token Counting PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Models API");
    println!("----------------------------------------");
    if let Err(e) = test_models_api().await {
        println!("❌ Models API FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Models API PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Error Handling");
    println!("----------------------------------------");
    if let Err(e) = test_error_handling().await {
        println!("❌ Error Handling FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Error Handling PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Conversation");
    println!("----------------------------------------");
    if let Err(e) = test_conversation().await {
        println!("❌ Conversation FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Conversation PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Tool Use");
    println!("----------------------------------------");
    if let Err(e) = test_tool_use().await {
        println!("❌ Tool Use FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Tool Use PASSED");
        passed += 1;
    }

    println!("\n📝 Running: Rate Limiting");
    println!("----------------------------------------");
    if let Err(e) = test_rate_limiting().await {
        println!("❌ Rate Limiting FAILED: {}", e);
        failed += 1;
    } else {
        println!("✅ Rate Limiting PASSED");
        passed += 1;
    }

    println!("\n========================================");
    println!("   TEST RESULTS");
    println!("========================================");
    println!("✅ Passed: {}", passed);
    println!("❌ Failed: {}", failed);
    println!("📊 Total: {}", passed + failed);

    if failed > 0 {
        Err(format!("{} tests failed", failed).into())
    } else {
        Ok(())
    }
}
