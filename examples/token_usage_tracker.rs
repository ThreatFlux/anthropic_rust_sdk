//! Token Usage Tracker - Track your Anthropic API token usage
//!
//! This example demonstrates how to track token usage across multiple API calls
//! and calculate costs based on current pricing.
//!
//! Run with: ANTHROPIC_API_KEY=your_key cargo run --example token_usage_tracker

use chrono::{DateTime, Utc};
use futures::StreamExt;
use std::collections::HashMap;
use std::error::Error;
use threatflux::{builders::MessageBuilder, config::models, models::message::StreamEvent, Client};

/// Token pricing per million tokens (as of 2025)
#[derive(Debug, Clone)]
struct ModelPricing {
    input_per_million: f64,
    output_per_million: f64,
}

impl ModelPricing {
    fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1_000_000.0) * self.input_per_million;
        let output_cost = (output_tokens as f64 / 1_000_000.0) * self.output_per_million;
        input_cost + output_cost
    }
}

/// Token usage tracker
#[derive(Debug, Default)]
struct TokenUsageTracker {
    total_input_tokens: u64,
    total_output_tokens: u64,
    total_thinking_tokens: u64,
    model_usage: HashMap<String, (u64, u64)>, // (input, output) per model
    api_calls: u32,
    start_time: Option<DateTime<Utc>>,
    pricing: HashMap<String, ModelPricing>,
}

impl TokenUsageTracker {
    fn new() -> Self {
        let mut pricing = HashMap::new();

        // Claude 3.5 models
        pricing.insert(
            models::HAIKU_3_5.to_string(),
            ModelPricing {
                input_per_million: 0.25,
                output_per_million: 1.25,
            },
        );
        pricing.insert(
            models::SONNET_3_5.to_string(),
            ModelPricing {
                input_per_million: 3.0,
                output_per_million: 15.0,
            },
        );
        pricing.insert(
            models::OPUS_3.to_string(),
            ModelPricing {
                input_per_million: 15.0,
                output_per_million: 75.0,
            },
        );

        // Claude 4 models
        pricing.insert(
            models::OPUS_4_1.to_string(),
            ModelPricing {
                input_per_million: 15.0,
                output_per_million: 75.0,
            },
        );
        pricing.insert(
            models::OPUS_4.to_string(),
            ModelPricing {
                input_per_million: 15.0,
                output_per_million: 75.0,
            },
        );
        pricing.insert(
            models::SONNET_4.to_string(),
            ModelPricing {
                input_per_million: 3.0,
                output_per_million: 15.0,
            },
        );

        Self {
            pricing,
            start_time: Some(Utc::now()),
            ..Default::default()
        }
    }

    fn track_usage(&mut self, model: &str, input_tokens: u32, output_tokens: u32) {
        self.total_input_tokens += input_tokens as u64;
        self.total_output_tokens += output_tokens as u64;
        self.api_calls += 1;

        let entry = self.model_usage.entry(model.to_string()).or_insert((0, 0));
        entry.0 += input_tokens as u64;
        entry.1 += output_tokens as u64;
    }

    fn track_thinking(&mut self, thinking_tokens: u32) {
        self.total_thinking_tokens += thinking_tokens as u64;
    }

    fn calculate_total_cost(&self) -> f64 {
        let mut total_cost = 0.0;

        for (model, (input, output)) in &self.model_usage {
            if let Some(pricing) = self.pricing.get(model) {
                total_cost += pricing.calculate_cost(*input as u32, *output as u32);
            }
        }

        total_cost
    }

    fn print_summary(&self) {
        println!("\n📊 Token Usage Summary");
        println!("{}", "=".repeat(60));

        if let Some(start_time) = self.start_time {
            let duration = Utc::now().signed_duration_since(start_time);
            println!("📅 Session Duration: {} minutes", duration.num_minutes());
        }

        println!("🔢 Total API Calls: {}", self.api_calls);
        println!("\n📈 Token Usage:");
        println!("   Input Tokens:    {:>10} tokens", self.total_input_tokens);
        println!(
            "   Output Tokens:   {:>10} tokens",
            self.total_output_tokens
        );
        if self.total_thinking_tokens > 0 {
            println!(
                "   Thinking Tokens: {:>10} tokens",
                self.total_thinking_tokens
            );
        }
        println!(
            "   Total Tokens:    {:>10} tokens",
            self.total_input_tokens + self.total_output_tokens + self.total_thinking_tokens
        );

        println!("\n🤖 Usage by Model:");
        for (model, (input, output)) in &self.model_usage {
            let model_name = model.split('-').take(3).collect::<Vec<_>>().join("-");
            println!("   {}:", model_name);
            println!("      Input:  {:>8} tokens", input);
            println!("      Output: {:>8} tokens", output);

            if let Some(pricing) = self.pricing.get(model) {
                let cost = pricing.calculate_cost(*input as u32, *output as u32);
                println!("      Cost:   ${:.4}", cost);
            }
        }

        println!(
            "\n💰 Total Estimated Cost: ${:.4}",
            self.calculate_total_cost()
        );
        println!("{}", "=".repeat(60));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔍 Anthropic Token Usage Tracker\n");

    // Initialize client
    let client = Client::from_env()?;
    let mut tracker = TokenUsageTracker::new();

    // Example 1: Track basic message usage
    println!("📝 Test 1: Basic Message with Haiku 3.5");
    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(100)
        .user("Write a haiku about API tokens.")
        .build();

    match client.messages().create(request, None).await {
        Ok(response) => {
            println!("Response: {}", response.text());
            println!(
                "Tokens: {} in, {} out",
                response.usage.input_tokens, response.usage.output_tokens
            );

            tracker.track_usage(
                &response.model,
                response.usage.input_tokens,
                response.usage.output_tokens,
            );
        }
        Err(e) => println!("Error: {}", e),
    }

    // Example 2: Track streaming usage
    println!("\n📝 Test 2: Streaming with Token Tracking");
    let request = MessageBuilder::new()
        .model(models::HAIKU_3_5)
        .max_tokens(150)
        .stream()
        .user("List 5 ways to optimize API token usage.")
        .build();

    match client.messages().create_stream(request, None).await {
        Ok(mut stream) => {
            let mut stream_input_tokens = 0;
            let mut stream_output_tokens = 0;

            print!("Response: ");
            while let Some(event) = stream.next().await {
                match event? {
                    StreamEvent::MessageStart { message } => {
                        stream_input_tokens = message.usage.input_tokens;
                    }
                    StreamEvent::ContentBlockDelta { delta, .. } => {
                        if let Some(text) = delta.text {
                            print!("{}", text);
                        }
                    }
                    StreamEvent::MessageDelta { delta: _, usage } => {
                        stream_output_tokens = usage.output_tokens;
                    }
                    StreamEvent::MessageStop => break,
                    _ => {}
                }
            }
            println!("\n");

            println!(
                "Stream tokens: {} in, {} out",
                stream_input_tokens, stream_output_tokens
            );
            tracker.track_usage(models::HAIKU_3_5, stream_input_tokens, stream_output_tokens);
        }
        Err(e) => println!("Stream error: {}", e),
    }

    // Example 3: Count tokens before sending
    println!("\n📝 Test 3: Pre-count Tokens to Optimize Costs");
    let test_message = "This is a test message to demonstrate token counting. \
                        We can count tokens before sending the actual request to estimate costs.";

    match client
        .messages()
        .count_tokens_simple(models::HAIKU_3_5, test_message, None)
        .await
    {
        Ok(count) => {
            println!("Message: \"{}...\"", &test_message[..50]);
            println!("Would use {} input tokens", count.input_tokens);

            if let Some(pricing) = tracker.pricing.get(models::HAIKU_3_5) {
                let estimated_cost = pricing.calculate_cost(count.input_tokens, 100); // Assume 100 output
                println!(
                    "Estimated cost: ${:.6} (assuming 100 output tokens)",
                    estimated_cost
                );
            }
        }
        Err(e) => println!("Token counting error: {}", e),
    }

    // Example 4: Compare token usage across models
    println!("\n📝 Test 4: Compare Token Usage Across Models");
    let test_prompt = "Explain quantum computing in one sentence.";

    for model in &[models::HAIKU_3_5, models::SONNET_3_5] {
        println!("\nTesting {}", model);

        // Count tokens
        match client
            .messages()
            .count_tokens_simple(model, test_prompt, None)
            .await
        {
            Ok(count) => {
                println!("  Input tokens: {}", count.input_tokens);

                // Actually run the request
                let request = MessageBuilder::new()
                    .model(*model)
                    .max_tokens(100)
                    .user(test_prompt)
                    .build();

                match client.messages().create(request, None).await {
                    Ok(response) => {
                        println!("  Output tokens: {}", response.usage.output_tokens);
                        println!(
                            "  Total tokens: {}",
                            response.usage.input_tokens + response.usage.output_tokens
                        );

                        tracker.track_usage(
                            model,
                            response.usage.input_tokens,
                            response.usage.output_tokens,
                        );
                    }
                    Err(e) => println!("  Error: {}", e),
                }
            }
            Err(e) => println!("  Token counting error: {}", e),
        }

        // Small delay between requests
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Example 5: Simulate Claude 4 with thinking tokens (mock calculation)
    println!("\n📝 Test 5: Simulated Claude 4 Thinking Tokens");
    println!("Note: This is a simulation showing how thinking tokens would be tracked");

    // Simulate a Claude 4 request with thinking
    let thinking_tokens = 5000; // Simulated thinking tokens
    let input_tokens = 50;
    let output_tokens = 200;

    println!("Simulated Opus 4.1 usage:");
    println!("  Input tokens: {}", input_tokens);
    println!("  Thinking tokens: {}", thinking_tokens);
    println!("  Output tokens: {}", output_tokens);

    tracker.track_usage(models::OPUS_4_1, input_tokens, output_tokens);
    tracker.track_thinking(thinking_tokens);

    // Print final summary
    tracker.print_summary();

    // Example: Calculate daily/monthly projections
    println!("\n📈 Usage Projections");
    println!("{}", "-".repeat(40));

    let total_tokens = tracker.total_input_tokens + tracker.total_output_tokens;
    let cost_per_call = tracker.calculate_total_cost() / tracker.api_calls.max(1) as f64;

    println!(
        "Average tokens per call: {}",
        total_tokens / tracker.api_calls.max(1) as u64
    );
    println!("Average cost per call: ${:.6}", cost_per_call);

    // Projections (assuming similar usage patterns)
    let calls_per_day = 100;
    let daily_cost = cost_per_call * calls_per_day as f64;
    let monthly_cost = daily_cost * 30.0;

    println!("\nProjected costs:");
    println!("  {} calls/day: ${:.2}/day", calls_per_day, daily_cost);
    println!("  Monthly (30 days): ${:.2}", monthly_cost);
    println!("  Yearly: ${:.2}", monthly_cost * 12.0);

    // Optimization tips
    println!("\n💡 Token Optimization Tips:");
    println!("1. Use Haiku 3.5 for simple tasks (lowest cost)");
    println!("2. Pre-count tokens for expensive models");
    println!("3. Use batch API for 50% cost savings on bulk operations");
    println!("4. Implement prompt caching for repeated contexts");
    println!("5. Set appropriate max_tokens limits");
    println!("6. Use streaming to show progress without waiting");

    Ok(())
}

/// Helper function to track token usage across a session
pub struct SessionTracker {
    client: Client,
    tracker: TokenUsageTracker,
}

impl SessionTracker {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            tracker: TokenUsageTracker::new(),
        }
    }

    pub async fn send_message(
        &mut self,
        model: &str,
        message: &str,
        max_tokens: u32,
    ) -> Result<String, Box<dyn Error>> {
        let request = MessageBuilder::new()
            .model(model)
            .max_tokens(max_tokens)
            .user(message)
            .build();

        let response = self.client.messages().create(request, None).await?;

        self.tracker.track_usage(
            model,
            response.usage.input_tokens,
            response.usage.output_tokens,
        );

        Ok(response.text())
    }

    pub fn get_total_tokens(&self) -> u64 {
        self.tracker.total_input_tokens + self.tracker.total_output_tokens
    }

    pub fn get_total_cost(&self) -> f64 {
        self.tracker.calculate_total_cost()
    }

    pub fn print_session_summary(&self) {
        self.tracker.print_summary();
    }
}
