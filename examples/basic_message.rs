//! Basic message example
//!
//! This example demonstrates how to send a simple message to Claude and get a response.
//!
//! Usage: cargo run --example basic_message

use std::error::Error;
use threatflux::{Client, MessageBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    println!("🚀 Threatflux - Basic Message Example");
    println!("=====================================");

    // Create client from environment variables
    let client = Client::from_env()?;

    // Simple message using the builder pattern
    let request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(1000)
        .temperature(0.7)
        .system("You are a helpful AI assistant.")
        .user("Hello! Can you tell me a interesting fact about Rust programming language?")
        .build();

    println!("Sending message to Claude...");

    // Send the message
    let response = client.messages().create(request, None).await?;

    println!("\n✅ Response received!");
    println!("Model: {}", response.model);
    println!("Stop reason: {:?}", response.stop_reason);
    println!(
        "Usage: {} input tokens, {} output tokens",
        response.usage.input_tokens, response.usage.output_tokens
    );

    println!("\n💬 Claude's response:");
    println!("{}", response.text());

    // Example with conversation history
    println!("\n{}", "=".repeat(50));
    println!("💬 Conversation Example");
    println!("{}", "=".repeat(50));

    let conversation_request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(500)
        .user("What's your name?")
        .assistant("I'm Claude, an AI assistant created by Anthropic.")
        .user("What can you help me with?")
        .build();

    let conversation_response = client.messages().create(conversation_request, None).await?;

    println!("💬 Claude's response:");
    println!("{}", conversation_response.text());

    // Example with different presets
    println!("\n{}", "=".repeat(50));
    println!("🎨 Creative Writing Example");
    println!("{}", "=".repeat(50));

    let creative_request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .creative() // Uses creative preset (high temperature)
        .user("Write a short poem about programming in Rust")
        .build();

    let creative_response = client.messages().create(creative_request, None).await?;

    println!("🎨 Creative response:");
    println!("{}", creative_response.text());

    // Token counting example
    println!("\n{}", "=".repeat(50));
    println!("🔢 Token Counting Example");
    println!("{}", "=".repeat(50));

    let token_count = client
        .messages()
        .count_tokens_simple("claude-3-5-haiku-20241022", "Hello, how are you?", None)
        .await?;

    println!(
        "Token count for 'Hello, how are you?': {} tokens",
        token_count.input_tokens
    );

    println!("\n✅ All examples completed successfully!");
    Ok(())
}
