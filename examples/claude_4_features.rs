//! Example demonstrating Claude 4 specific features
//!
//! This example shows how to use:
//! - Extended thinking mode
//! - 1M context window for Sonnet 4
//! - Tool use during thinking
//! - Hybrid reasoning modes

use futures::StreamExt;
use std::error::Error;
use threatflux::{
    builders::MessageBuilder,
    config::models,
    models::{common::Tool, message::StreamEvent},
    types::RequestOptions,
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Load environment variables
    dotenv::dotenv().ok();

    // Create client
    let client = Client::from_env()?;

    // Example 1: Claude Opus 4.1 with Extended Thinking
    println!("=== Example 1: Opus 4.1 with Extended Thinking ===\n");
    opus_4_extended_thinking(&client).await?;

    // Example 2: Claude Sonnet 4 with 1M Context
    println!("\n=== Example 2: Sonnet 4 with 1M Context ===\n");
    sonnet_4_large_context(&client).await?;

    // Example 3: Claude 4 with Tool Use During Thinking
    println!("\n=== Example 3: Claude 4 with Tool Use During Thinking ===\n");
    claude_4_tool_thinking(&client).await?;

    // Example 4: Streaming with Claude 4 Models
    println!("\n=== Example 4: Streaming with Claude 4 ===\n");
    claude_4_streaming(&client).await?;

    // Example 5: Compare Different Claude 4 Models
    println!("\n=== Example 5: Compare Claude 4 Models ===\n");
    compare_claude_4_models(&client).await?;

    Ok(())
}

/// Example 1: Claude Opus 4.1 with Extended Thinking
async fn opus_4_extended_thinking(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Testing Opus 4.1 with deep thinking for complex problem solving...");

    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(4096)
        .thinking(64000) // Maximum thinking budget for Opus 4.1
        .user(
            "Solve this complex problem: Design a distributed system architecture \
             for a real-time collaborative code editor that supports millions of users, \
             with features like syntax highlighting, auto-completion, and version control. \
             Consider scalability, fault tolerance, and data consistency.",
        )
        .build();

    let response = client.messages().create(request, None).await?;
    println!("Response: {}", response.text());
    println!("Tokens used: {:?}", response.usage);

    Ok(())
}

/// Example 2: Claude Sonnet 4 with 1M Context Window
async fn sonnet_4_large_context(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Testing Sonnet 4 with 1M context window...");

    // For demonstration, we'll use a smaller context
    // In real usage, you could load a large codebase or document
    let large_context = "This is a placeholder for a very large document.\n".repeat(1000);

    let request = MessageBuilder::new()
        .model(models::SONNET_4)
        .max_tokens(2048)
        .user(format!(
            "Here's a large document:\n\n{}\n\nPlease summarize the main points.",
            large_context
        ))
        .build();

    // Enable 1M context window with request options
    let options = RequestOptions::new().with_1m_context();

    let response = client.messages().create(request, Some(options)).await?;
    println!("Summary: {}", response.text());

    Ok(())
}

/// Example 3: Claude 4 with Tool Use During Thinking
async fn claude_4_tool_thinking(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Testing Claude 4 with tool use during extended thinking...");

    // Define tools
    let calculator_tool = Tool::new(
        "calculator",
        "Perform mathematical calculations",
        serde_json::json!({
            "type": "object",
            "properties": {
                "expression": {
                    "type": "string",
                    "description": "Mathematical expression to evaluate"
                }
            },
            "required": ["expression"]
        }),
    );

    let web_search_tool = Tool::new(
        "web_search",
        "Search the web for information",
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        }),
    );

    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(2048)
        .thinking_with_tools(50000) // Enable tool use during thinking
        .tool(calculator_tool)
        .tool(web_search_tool)
        .user(
            "Research and calculate: What is the compound annual growth rate of \
             the global AI market from 2020 to 2025, and what would be the projected \
             market size in 2030 if this growth rate continues?",
        )
        .build();

    // Enable extended thinking with tools beta feature
    let options = RequestOptions::new().with_extended_thinking_tools();

    let response = client.messages().create(request, Some(options)).await?;
    println!("Analysis: {}", response.text());

    Ok(())
}

/// Example 4: Streaming with Claude 4 Models
async fn claude_4_streaming(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Testing streaming with Claude 4 models...");

    let request = MessageBuilder::new()
        .model(models::SONNET_4)
        .max_tokens(1000)
        .thinking(16000) // Moderate thinking for Sonnet 4
        .stream()
        .user("Write a detailed explanation of quantum computing for beginners.")
        .build();

    let mut stream = client.messages().create_stream(request, None).await?;

    print!("Streaming response: ");
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = delta.text {
                    print!("{}", text);
                }
            }
            StreamEvent::MessageStop => {
                println!("\n\n[Stream complete]");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}

/// Example 5: Compare Different Claude 4 Models
async fn compare_claude_4_models(client: &Client) -> Result<(), Box<dyn Error>> {
    println!("Comparing different Claude 4 models on the same task...");

    let test_prompt = "Write a recursive Fibonacci function in Rust with memoization.";

    // Test with different models
    let models_to_test = vec![
        (models::OPUS_4_1, "Opus 4.1", Some(20000)),
        (models::OPUS_4, "Opus 4", Some(20000)),
        (models::SONNET_4, "Sonnet 4", Some(10000)),
        (models::HAIKU_3_5, "Haiku 3.5", None), // No thinking for non-Claude 4
    ];

    for (model, name, thinking_budget) in models_to_test {
        println!("\n--- Testing {} ---", name);

        let mut builder = MessageBuilder::new()
            .model(model)
            .max_tokens(500)
            .user(test_prompt);

        // Add thinking if supported
        if let Some(budget) = thinking_budget {
            if models::supports_thinking(model) {
                builder = builder.thinking(budget);
            }
        }

        let request = builder.build();

        match client.messages().create(request, None).await {
            Ok(response) => {
                println!("Response from {}:", name);
                println!("{}", response.text());
                println!(
                    "Tokens: input={}, output={}",
                    response.usage.input_tokens, response.usage.output_tokens
                );
            }
            Err(e) => {
                eprintln!("Error with {}: {}", name, e);
            }
        }
    }

    Ok(())
}

// Additional helper examples

/// Example: Using Opus 4.1 specific constraints
#[allow(dead_code)]
async fn opus_4_1_constraints(client: &Client) -> Result<(), Box<dyn Error>> {
    // Note: Opus 4.1 cannot use both temperature and top_p simultaneously
    let request = MessageBuilder::new()
        .model(models::OPUS_4_1)
        .max_tokens(1000)
        .temperature(0.7) // Use temperature OR top_p, not both
        // .top_p(0.9) // This would cause an error with Opus 4.1
        .user("Generate creative ideas for a new mobile app")
        .build();

    let response = client.messages().create(request, None).await?;
    println!("Ideas: {}", response.text());

    Ok(())
}

/// Example: Validating thinking budget
#[allow(dead_code)]
async fn validate_thinking_budget(client: &Client) -> Result<(), Box<dyn Error>> {
    // Get maximum thinking tokens for a model
    let model = models::OPUS_4_1;
    let max_tokens = models::max_thinking_tokens(model).unwrap_or(0);
    println!("Maximum thinking tokens for {}: {}", model, max_tokens);

    // Build request with valid thinking budget
    let request = MessageBuilder::new()
        .model(model)
        .max_tokens(1000)
        .thinking(max_tokens.min(50000)) // Use up to 50K or max allowed
        .user("Analyze this code for potential improvements")
        .build();

    let _response = client.messages().create(request, None).await?;
    println!("Analysis complete");

    Ok(())
}
