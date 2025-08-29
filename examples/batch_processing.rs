//! Batch processing example
//!
//! This example demonstrates how to process multiple messages in batches for efficiency.
//!
//! Usage: cargo run --example batch_processing

use std::error::Error;
use std::time::Duration;
use threatflux::{
    builders::{BatchBuilder, MessageBuilder},
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for logs
    tracing_subscriber::fmt::init();

    println!("📦 Threatflux - Batch Processing Example");
    println!("=========================================");

    // Create client from environment variables
    let client = Client::from_env()?;

    // Create a batch of different types of requests
    let batch_request = BatchBuilder::new()
        // Simple text requests
        .add_simple_request(
            "greeting",
            "claude-3-5-haiku-20241022",
            "Say hello in 5 different languages",
            300,
        )
        .add_simple_request(
            "math",
            "claude-3-5-haiku-20241022",
            "What is 15 * 23 + 47?",
            100,
        )
        // Creative writing request
        .add_creative(
            "poem",
            "claude-3-5-haiku-20241022",
            "Write a haiku about programming",
            200,
        )
        // Analytical request
        .add_analytical(
            "analysis",
            "claude-3-5-haiku-20241022",
            "Analyze the pros and cons of using Rust vs Python for web development",
            800,
        )
        // Code generation request
        .add_code_generation(
            "code",
            "claude-3-5-haiku-20241022",
            "Write a simple Rust function to calculate factorial",
            400,
        )
        // Custom request using MessageBuilder
        .add_with_builder(
            "conversation",
            MessageBuilder::new()
                .model("claude-3-5-haiku-20241022")
                .max_tokens(500)
                .temperature(0.6)
                .system("You are a helpful tutor.")
                .user("Explain recursion in programming with a simple example"),
        )
        .build();

    println!(
        "📤 Creating batch with {} requests...",
        batch_request.requests.len()
    );

    // Submit the batch
    let batch = client.message_batches().create(batch_request, None).await?;

    println!("✅ Batch created with ID: {}", batch.id);
    println!("📊 Status: {:?}", batch.processing_status);
    println!("📋 Request counts: {:?}", batch.request_counts);

    // Wait for batch to complete
    println!("\n⏳ Waiting for batch to complete...");
    let completed_batch = client
        .message_batches()
        .wait_for_completion(&batch.id, Duration::from_secs(5), Duration::from_secs(300))
        .await?;

    println!("✅ Batch completed!");
    println!("📊 Final status: {:?}", completed_batch.processing_status);
    println!("📈 Success rate: {:.1}%", completed_batch.success_rate());
    println!(
        "🎯 Completion: {:.1}%",
        completed_batch.completion_percentage()
    );

    if let Some(results_file_id) = &completed_batch.results_file_id {
        println!("📄 Results file ID: {}", results_file_id);

        // Download and display results
        println!("\n📥 Downloading results...");
        let results_data = client.files().download(results_file_id, None).await?;
        let results_text = String::from_utf8_lossy(&results_data);

        println!("\n📋 Batch Results:");
        println!("{}", "=".repeat(60));

        // Parse and display each result
        for line in results_text.lines() {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<threatflux::models::batch::BatchResult>(line) {
                Ok(result) => {
                    println!("\n🔹 Request ID: {}", result.custom_id);
                    if let Some(message) = result.message {
                        println!(
                            "   Response: {}",
                            message.text().chars().take(100).collect::<String>()
                        );
                        if message.text().len() > 100 {
                            println!("   ... (truncated)");
                        }
                        println!(
                            "   Usage: {} in + {} out tokens",
                            message.usage.input_tokens, message.usage.output_tokens
                        );
                    } else if let Some(error) = result.error {
                        println!("   ❌ Error: {}", error.message);
                    }
                }
                Err(e) => {
                    println!("⚠️ Failed to parse result: {}", e);
                }
            }
        }
    }

    // Example with batch variations
    println!("\n{}", "=".repeat(60));
    println!("🔄 Batch Variations Example");
    println!("{}", "=".repeat(60));

    let variations_batch = BatchBuilder::new()
        .add_batch_variations(
            "translate",
            "claude-3-5-haiku-20241022",
            "Translate 'Hello, how are you?' to",
            vec!["Spanish", "French", "German", "Japanese", "Italian"],
            200,
        )
        .build();

    println!("📤 Creating variations batch...");
    let variations_batch_response = client
        .message_batches()
        .create(variations_batch, None)
        .await?;

    println!(
        "✅ Variations batch created: {}",
        variations_batch_response.id
    );
    println!(
        "📊 Total requests: {}",
        variations_batch_response.request_counts.total
    );

    // Example with template-based batch
    println!("\n{}", "=".repeat(60));
    println!("📝 Template-Based Batch Example");
    println!("{}", "=".repeat(60));

    let template_batch = BatchBuilder::new()
        .add_from_template(
            "explain",
            "claude-3-5-haiku-20241022",
            "Explain the concept of {concept} in {language} programming in one sentence.",
            vec![
                ("concept", "variables"),
                ("language", "Rust"),
                ("concept", "functions"),
                ("language", "Python"),
                ("concept", "classes"),
                ("language", "Java"),
                ("concept", "closures"),
                ("language", "JavaScript"),
            ],
            300,
        )
        .build();

    println!("📤 Creating template batch...");
    let template_batch_response = client
        .message_batches()
        .create(template_batch, None)
        .await?;

    println!("✅ Template batch created: {}", template_batch_response.id);

    // Example using default parameters
    println!("\n{}", "=".repeat(60));
    println!("⚙️ Batch with Defaults Example");
    println!("{}", "=".repeat(60));

    let defaults_batch = BatchBuilder::new()
        .with_defaults("claude-3-5-haiku-20241022", 250)
        .add("fact1", "Tell me a fun fact about space")
        .add("fact2", "Tell me a fun fact about the ocean")
        .add("fact3", "Tell me a fun fact about animals")
        .add_qa_with_defaults(
            "qa1",
            "What is the capital of Australia?",
            Some("Please be accurate and concise."),
        )
        .add_creative_with_defaults(
            "creative1",
            "Write a short story about a robot learning to paint",
        )
        .build();

    println!("📤 Creating batch with defaults...");
    let defaults_batch_response = client
        .message_batches()
        .create(defaults_batch, None)
        .await?;

    println!("✅ Defaults batch created: {}", defaults_batch_response.id);

    // List recent batches
    println!("\n{}", "=".repeat(60));
    println!("📋 Recent Batches");
    println!("{}", "=".repeat(60));

    let recent_batches = client
        .message_batches()
        .list(
            Some(threatflux::types::Pagination::new().with_limit(5)),
            None,
        )
        .await?;

    println!("📦 Found {} recent batches:", recent_batches.data.len());
    for batch in recent_batches.data {
        println!(
            "  🔹 {} - Status: {:?} - {} requests",
            batch.id, batch.processing_status, batch.request_counts.total
        );
    }

    // Demonstrate batch cancellation (on a new batch)
    println!("\n{}", "=".repeat(60));
    println!("❌ Batch Cancellation Example");
    println!("{}", "=".repeat(60));

    let cancel_batch = BatchBuilder::new()
        .add_simple_request(
            "cancel_test",
            "claude-3-5-haiku-20241022",
            "This will be cancelled",
            100,
        )
        .build();

    let cancel_batch_response = client.message_batches().create(cancel_batch, None).await?;
    println!(
        "📤 Created batch for cancellation: {}",
        cancel_batch_response.id
    );

    // Cancel immediately
    let cancelled_batch = client
        .message_batches()
        .cancel(&cancel_batch_response.id, None)
        .await?;
    println!(
        "❌ Batch cancelled: {} (Status: {:?})",
        cancelled_batch.id, cancelled_batch.processing_status
    );

    println!("\n✅ All batch processing examples completed successfully!");
    Ok(())
}
