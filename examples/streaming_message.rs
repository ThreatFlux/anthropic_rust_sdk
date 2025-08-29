//! Streaming message example
//!
//! This example demonstrates how to use streaming to get real-time responses from Claude.
//!
//! Usage: cargo run --example streaming_message

use futures::StreamExt;
use std::error::Error;
use std::io::{self, Write};
use threatflux::{
    builders::{MessageBuilder, ParameterBuilder},
    Client, StreamEvent,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    println!("🌊 Threatflux - Streaming Message Example");
    println!("==========================================");

    // Create client from environment variables
    let client = Client::from_env()?;

    // Create a streaming message request
    let request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(1500)
        .temperature(0.7)
        .system("You are a helpful AI assistant. Be concise but informative.")
        .user("Tell me an interesting story about the development of the Rust programming language. Include some technical details.")
        .stream() // Enable streaming
        .build();

    println!("Starting streaming conversation...");
    println!("\n💬 Claude is typing");
    print!("Response: ");
    io::stdout().flush()?;

    // Create the stream
    let mut stream = client.messages().create_stream(request, None).await?;
    let mut full_response = String::new();

    // Process events as they arrive
    while let Some(event_result) = stream.next().await {
        let event = event_result?;

        match event {
            StreamEvent::MessageStart { message } => {
                println!("\n📝 Message started (ID: {})", message.id);
                print!("📖 ");
                io::stdout().flush()?;
            }
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = &delta.text {
                    print!("{}", text);
                    io::stdout().flush()?;
                    full_response.push_str(text);
                }
            }
            StreamEvent::MessageDelta { usage, delta } => {
                if delta.stop_reason.is_some() {
                    println!(
                        "\n\n📊 Final usage: {} input + {} output = {} total tokens",
                        usage.input_tokens,
                        usage.output_tokens,
                        usage.total_tokens()
                    );
                }
            }
            StreamEvent::MessageStop => {
                println!("\n\n✅ Stream completed!");
                break;
            }
            StreamEvent::Error { error } => {
                println!("\n❌ Stream error: {:?}", error);
                break;
            }
            _ => {
                // Handle other event types (ContentBlockStart, ContentBlockStop, Ping)
            }
        }
    }

    // Alternative approach: collect the entire response
    println!("\n{}", "=".repeat(60));
    println!("🔄 Alternative: Collect Full Response");
    println!("{}", "=".repeat(60));

    let collect_request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(800)
        .temperature(0.5)
        .analytical() // Use analytical preset
        .user("Explain the concept of ownership in Rust in 3 bullet points")
        .stream()
        .build();

    let stream = client
        .messages()
        .create_stream(collect_request, None)
        .await?;

    // Collect the entire response at once
    println!("Collecting full response...");
    let full_message = stream.collect_message().await?;

    println!("\n💬 Complete response:");
    println!("{}", full_message.text());
    println!(
        "\n📊 Usage: {} input + {} output tokens",
        full_message.usage.input_tokens, full_message.usage.output_tokens
    );

    // Text-only streaming example
    println!("\n{}", "=".repeat(60));
    println!("📝 Text-Only Streaming");
    println!("{}", "=".repeat(60));

    let text_request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(500)
        .temperature(0.8)
        .user("Give me 5 creative uses for a paperclip")
        .stream()
        .build();

    let stream = client.messages().create_stream(text_request, None).await?;

    println!("Collecting text only...");
    let text_response = stream.collect_text().await?;

    println!("\n📝 Text response:");
    println!("{}", text_response);

    // Interactive streaming example
    println!("\n{}", "=".repeat(60));
    println!("🎯 Interactive Streaming Demo");
    println!("{}", "=".repeat(60));

    println!("Type a message for Claude (or 'quit' to exit):");
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim() != "quit" {
        let interactive_request = MessageBuilder::new()
            .model("claude-3-5-haiku-20241022")
            .max_tokens(1000)
            .temperature(0.7)
            .conversational() // Use conversational preset
            .user(input.trim())
            .stream()
            .build();

        println!("\n💬 Claude's response:");
        let mut stream = client
            .messages()
            .create_stream(interactive_request, None)
            .await?;

        while let Some(event_result) = stream.next().await {
            let event = event_result?;

            if let StreamEvent::ContentBlockDelta { delta, .. } = event {
                if let Some(text) = &delta.text {
                    print!("{}", text);
                    io::stdout().flush()?;
                }
            } else if let StreamEvent::MessageStop = event {
                println!("\n");
                break;
            }
        }
    }

    println!("\n✅ Streaming examples completed successfully!");
    Ok(())
}
