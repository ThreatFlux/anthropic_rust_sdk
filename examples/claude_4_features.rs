//! Example demonstrating current Claude API features.
//!
//! This example shows how to use:
//! - Adaptive thinking + the `effort` parameter
//! - Prompt caching (system + auto-cache)
//! - Server-side tools (web search, code execution)
//! - Structured outputs (JSON schema)
//! - Streaming with summarized reasoning
//! - Refusal fallbacks (Claude Fable 5)

use futures::StreamExt;
use std::error::Error;
use threatflux::{
    builders::MessageBuilder,
    config::models,
    models::{
        common::Tool,
        message::{OutputConfig, OutputEffort, StreamEvent},
    },
    types::RequestOptions,
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenvy::dotenv().ok();
    let client = Client::from_env()?;

    println!("=== Example 1: Adaptive thinking + effort ===\n");
    adaptive_thinking(&client).await?;

    println!("\n=== Example 2: Prompt caching ===\n");
    prompt_caching(&client).await?;

    println!("\n=== Example 3: Server-side tools ===\n");
    server_tools(&client).await?;

    println!("\n=== Example 4: Structured outputs ===\n");
    structured_output(&client).await?;

    println!("\n=== Example 5: Streaming with summarized reasoning ===\n");
    streaming(&client).await?;

    println!("\n=== Example 6: Refusal fallbacks (Fable 5) ===\n");
    refusal_fallbacks(&client).await?;

    Ok(())
}

/// Adaptive thinking with `xhigh` effort — the recommended setup for hard,
/// agentic work on current models.
async fn adaptive_thinking(client: &Client) -> Result<(), Box<dyn Error>> {
    let request = MessageBuilder::new()
        .model(models::OPUS_4_8)
        .max_tokens(16000)
        .adaptive_thinking_summarized()
        .effort(OutputEffort::XHigh)
        .user(
            "Design a distributed system architecture for a real-time collaborative \
             code editor supporting millions of users. Consider scalability, fault \
             tolerance, and data consistency.",
        )
        .build();

    let response = client.messages().create(request, None).await?;
    println!("Response: {}", response.text());
    println!("Tokens used: {:?}", response.usage);
    Ok(())
}

/// Cache a large shared system prompt so repeated requests reuse the prefix.
async fn prompt_caching(client: &Client) -> Result<(), Box<dyn Error>> {
    let large_context = "This is a placeholder for a very large document.\n".repeat(1000);

    let request = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .max_tokens(2048)
        // Cache the system prompt (5-minute ephemeral breakpoint).
        .system_cached(format!(
            "You are an expert analyst. Reference:\n{large_context}"
        ))
        .user("Summarize the main points.")
        .build();

    let response = client.messages().create(request, None).await?;
    println!("Summary: {}", response.text());
    println!(
        "Cache: created={}, read={}",
        response.usage.cache_creation_input_tokens, response.usage.cache_read_input_tokens
    );
    Ok(())
}

/// Let Claude run Anthropic-hosted tools (web search + code execution).
async fn server_tools(client: &Client) -> Result<(), Box<dyn Error>> {
    let request = MessageBuilder::new()
        .model(models::OPUS_4_8)
        .max_tokens(4096)
        .adaptive_thinking()
        .tool(Tool::web_search())
        .tool(Tool::code_execution())
        .user(
            "Find the current global AI market size, then compute the projected 2030 \
             value at a 35% CAGR.",
        )
        .build();

    let response = client.messages().create(request, None).await?;
    println!("Analysis: {}", response.text());
    Ok(())
}

/// Constrain the response to a JSON schema (structured outputs).
async fn structured_output(client: &Client) -> Result<(), Box<dyn Error>> {
    let request = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .max_tokens(1024)
        .output_config(OutputConfig::json_schema(serde_json::json!({
            "type": "object",
            "properties": {
                "language": {"type": "string"},
                "difficulty": {"type": "string", "enum": ["easy", "medium", "hard"]}
            },
            "required": ["language", "difficulty"],
            "additionalProperties": false
        })))
        .user("Classify: 'Write a recursive Fibonacci function in Rust with memoization.'")
        .build();

    let response = client.messages().create(request, None).await?;
    println!("Structured JSON: {}", response.text());
    Ok(())
}

/// Stream a response with summarized reasoning blocks.
async fn streaming(client: &Client) -> Result<(), Box<dyn Error>> {
    let request = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .max_tokens(2000)
        .adaptive_thinking_summarized()
        .stream()
        .user("Explain quantum computing for beginners.")
        .build();

    let mut stream = client.messages().create_stream(request, None).await?;
    print!("Streaming response: ");
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = delta.text {
                    print!("{text}");
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

/// Opt into server-side refusal fallbacks on Claude Fable 5: a policy decline is
/// transparently re-served by the fallback model in the same call.
async fn refusal_fallbacks(client: &Client) -> Result<(), Box<dyn Error>> {
    let request = MessageBuilder::new()
        .model(models::FABLE_5)
        .max_tokens(4096)
        .add_fallback(models::OPUS_4_8)
        .user("Give me a concise overview of modern cryptographic hash functions.");

    // The `server-side-fallback-2026-06-01` beta header is required for fallbacks.
    let options = RequestOptions::new().with_server_side_fallback();

    let response = client
        .messages()
        .create(request.build(), Some(options))
        .await?;
    if response.is_refusal() {
        println!("Request was declined; fallback chain also refused.");
    } else {
        println!("Served by {}: {}", response.model, response.text());
    }
    Ok(())
}
