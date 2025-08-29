#!/usr/bin/env rust-script
//! Test Threatflux SDK with real Anthropic API
//! Run: ANTHROPIC_API_KEY=your_key cargo run --bin test_real_api

use std::env;
use threatflux::{Client, Config, builders::MessageBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🚀 Threatflux SDK - Real API Testing\n");
    
    // Check for API key
    let api_key = env::var("ANTHROPIC_API_KEY").unwrap_or_else(|_| {
        println!("❌ ANTHROPIC_API_KEY not set!");
        println!("Please run: export ANTHROPIC_API_KEY=your_key");
        std::process::exit(1);
    });
    
    if api_key == "your_anthropic_api_key_here" || api_key.len() < 10 {
        println!("❌ Invalid API key detected!");
        println!("Please set a real API key: export ANTHROPIC_API_KEY=sk-ant-...");
        std::process::exit(1);
    }
    
    println!("✅ API Key found (first 10 chars): {}...", &api_key[..10.min(api_key.len())]);
    
    // Create client
    let config = Config::new(api_key)?;
    let client = Client::new(config);
    
    // Test 1: Basic message with Haiku
    println!("\n📝 Test 1: Basic Message with Claude 3.5 Haiku");
    println!("----------------------------------------");
    
    let request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(50)
        .user("Say 'Hello from Threatflux SDK!' in exactly 5 words.")
        .build();
    
    match client.messages().create(request, None).await {
        Ok(response) => {
            println!("✅ Success!");
            println!("Response: {}", response.text());
            println!("Tokens: input={}, output={}", 
                response.usage.input_tokens, 
                response.usage.output_tokens);
        }
        Err(e) => {
            println!("❌ Error: {}", e);
            if e.to_string().contains("401") {
                println!("   Invalid API key. Please check your key.");
            } else if e.to_string().contains("429") {
                println!("   Rate limited. Please wait and try again.");
            }
        }
    }
    
    // Test 2: Streaming
    println!("\n📝 Test 2: Streaming Response");
    println!("----------------------------------------");
    
    let request = MessageBuilder::new()
        .model("claude-3-5-haiku-20241022")
        .max_tokens(50)
        .stream()
        .user("Count from 1 to 5.")
        .build();
    
    use futures::StreamExt;
    match client.messages().create_stream(request, None).await {
        Ok(mut stream) => {
            print!("Streaming: ");
            while let Some(event) = stream.next().await {
                match event {
                    Ok(evt) => {
                        if let threatflux::models::message::StreamEvent::ContentBlockDelta { delta, .. } = evt {
                            if let Some(text) = delta.text {
                                print!("{}", text);
                            }
                        }
                    }
                    Err(e) => {
                        println!("\n❌ Stream error: {}", e);
                        break;
                    }
                }
            }
            println!("\n✅ Stream complete!");
        }
        Err(e) => {
            println!("❌ Failed to start stream: {}", e);
        }
    }
    
    // Test 3: Token counting
    println!("\n📝 Test 3: Token Counting");
    println!("----------------------------------------");
    
    match client.messages().count_tokens_simple(
        "claude-3-5-haiku-20241022",
        "The quick brown fox jumps over the lazy dog.",
        None
    ).await {
        Ok(count) => {
            println!("✅ Token count: {} tokens", count.input_tokens);
        }
        Err(e) => {
            println!("❌ Token counting error: {}", e);
        }
    }
    
    // Test 4: List models
    println!("\n📝 Test 4: List Available Models");
    println!("----------------------------------------");
    
    match client.models().list(None, None).await {
        Ok(models) => {
            println!("✅ Found {} models:", models.data.len());
            for model in models.data.iter().take(3) {
                println!("   - {}: {}", model.id, model.display_name);
            }
        }
        Err(e) => {
            println!("❌ Failed to list models: {}", e);
        }
    }
    
    // Test 5: Error handling
    println!("\n📝 Test 5: Error Handling");
    println!("----------------------------------------");
    
    let request = MessageBuilder::new()
        .model("invalid-model-name")
        .max_tokens(10)
        .user("Test")
        .build();
    
    match client.messages().create(request, None).await {
        Ok(_) => {
            println!("❌ Should have failed with invalid model!");
        }
        Err(e) => {
            println!("✅ Correctly caught error: {}", e);
        }
    }
    
    println!("\n========================================");
    println!("✅ All basic tests completed!");
    println!("========================================");
    
    Ok(())
}