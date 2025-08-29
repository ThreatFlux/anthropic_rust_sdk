//! Simple API test binary
//! Run: ANTHROPIC_API_KEY=your_key cargo run --bin test_api

use std::env;
use threatflux::{builders::MessageBuilder, Client, Config};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Threatflux SDK - API Test\n");

    // Try to load from environment
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        eprintln!("❌ ANTHROPIC_API_KEY not set!");
        eprintln!("Usage: ANTHROPIC_API_KEY=sk-ant-... cargo run --bin test_api");
        std::process::exit(1);
    });

    println!("✅ Using API key: {}...", &api_key[..10.min(api_key.len())]);

    // Create client
    let config = Config::new(api_key)?;
    let client = Client::new(config);

    // Simple test
    println!("\nTesting Messages API...");
    let request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(30)
        .user("Reply with: Hello, SDK is working!")
        .build();

    match client.messages().create(request, None).await {
        Ok(response) => {
            println!("✅ Success!");
            println!("Response: {}", response.text());
            println!("Model: {}", response.model);
            println!(
                "Tokens: {} in, {} out",
                response.usage.input_tokens, response.usage.output_tokens
            );
        }
        Err(e) => {
            eprintln!("❌ Error: {}", e);
            eprintln!("Details: {:?}", e);
        }
    }

    Ok(())
}
