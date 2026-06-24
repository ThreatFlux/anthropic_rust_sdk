# Anthropic Rust SDK

[![CI](https://github.com/ThreatFlux/anthropic_rust_sdk/workflows/CI/badge.svg)](https://github.com/ThreatFlux/anthropic_rust_sdk/actions)
[![Coverage Status](https://codecov.io/gh/ThreatFlux/anthropic_rust_sdk/branch/main/graph/badge.svg)](https://codecov.io/gh/ThreatFlux/anthropic_rust_sdk)
[![Crates.io](https://img.shields.io/crates/v/threatflux-anthropic-sdk.svg)](https://crates.io/crates/threatflux-anthropic-sdk)
[![Documentation](https://docs.rs/threatflux-anthropic-sdk/badge.svg)](https://docs.rs/threatflux-anthropic-sdk)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.95%2B-blue.svg)](https://www.rust-lang.org/)
[![Security Audit](https://github.com/ThreatFlux/anthropic_rust_sdk/workflows/Security%20Audit/badge.svg)](https://github.com/ThreatFlux/anthropic_rust_sdk/security)
[![Dependency Status](https://deps.rs/repo/github/ThreatFlux/anthropic_rust_sdk/status.svg)](https://deps.rs/repo/github/ThreatFlux/anthropic_rust_sdk)

A comprehensive Rust SDK for the Anthropic API, providing async support, streaming capabilities, and broad coverage of Anthropic API endpoints including Messages, Models, Batches, Files, Skills, and Admin operations. Tracks the **current model generation** (Opus 4.x, Sonnet 4.6, Haiku 4.5, Fable 5) with adaptive thinking, the `effort` parameter, prompt caching, server-side tools, structured outputs, and refusal fallbacks.

## Features

- **🚀 Broad API Coverage**: Messages, Models, Batches, Files, Skills, and Admin endpoints
- **🧠 Adaptive Thinking + Effort**: `thinking: {type: "adaptive"}` with `low`/`medium`/`high`/`xhigh`/`max` effort
- **📜 1M Context Window**: Supported on current Opus/Sonnet/Fable models
- **💾 Prompt Caching**: Cacheable system/content/tool blocks with 5m/1h TTLs
- **🔧 Server-Side Tools**: Web search, web fetch, code execution, bash, text editor, memory
- **📐 Structured Outputs**: JSON-schema-constrained responses
- **🛟 Refusal Fallbacks**: Server-side fallback models for Claude Fable 5
- **⚡ Async/Await**: Built on `tokio` for high-performance async operations
- **🌊 Streaming Support**: Real-time streaming responses with Server-Sent Events
- **📦 Batch Processing**: Efficient batch message processing
- **📁 File Management**: Upload, download, and manage files
- **👑 Admin Operations**: Organization and workspace management
- **🛠 Builder Pattern**: Intuitive request builders for easy usage
- **🔄 Automatic Retries**: Intelligent retry logic with exponential backoff
- **⚖️ Rate Limiting**: Built-in rate limiting and throttling
- **🛡️ Type Safety**: Fully typed API with comprehensive error handling
- **📚 Rich Documentation**: Extensive examples and documentation

## Quick Start

Add the Anthropic Rust SDK to your `Cargo.toml`:

```toml
[dependencies]
threatflux-anthropic-sdk = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

The Cargo package uses hyphens, while Rust imports use underscores:
`use threatflux_anthropic_sdk::Client;`.

### Basic Usage

```rust
use threatflux_anthropic_sdk::{Client, builders::MessageBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the client with your API key
    let client = Client::from_env()?;
    
    // Create a message using the builder pattern
    let request = MessageBuilder::new()
        .model("claude-sonnet-4-6")  // Or the most capable: "claude-opus-4-8" / "claude-fable-5"
        .max_tokens(1000)
        .user("Hello, Claude! Tell me about Rust programming.")
        .build();
    
    // Send the message and get response
    let response = client.messages().create(request, None).await?;
    
    println!("Claude says: {}", response.text());
    println!("Usage: {} input + {} output tokens", 
             response.usage.input_tokens, 
             response.usage.output_tokens);
    
    Ok(())
}
```

### Environment Setup

Create a `.env` file in your project root:

```env
ANTHROPIC_API_KEY=your_api_key_here
# Optional: for admin operations
ANTHROPIC_ADMIN_KEY=your_admin_key_here
```

Or set the environment variable directly:

```bash
export ANTHROPIC_API_KEY="your_api_key_here"
```

## Supported Models

### Current Models
- **Claude Fable 5** (`claude-fable-5`) - Most capable widely released model (always-on thinking; 30-day retention)
- **Claude Opus 4.8** (`claude-opus-4-8`) - Most capable Opus-tier model, 1M context
- **Claude Opus 4.7** (`claude-opus-4-7`) - Previous-generation Opus, 1M context
- **Claude Opus 4.6** (`claude-opus-4-6`) - 1M context
- **Claude Sonnet 4.6** (`claude-sonnet-4-6`) - Best balance of speed and intelligence, 1M context
- **Claude Haiku 4.5** (`claude-haiku-4-5`) - Fastest and most cost-effective

Model ids are passed as plain strings; constants for the current catalog live in
`config::models` (retired ids are kept, marked deprecated). Use `claude-opus-4-8`
or `claude-fable-5` for the most capable, `claude-haiku-4-5` for the cheapest.

## Examples

### Adaptive thinking with the effort parameter

```rust
use threatflux_anthropic_sdk::{Client, builders::MessageBuilder, config::models};
use threatflux_anthropic_sdk::models::message::OutputEffort;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;

    // Adaptive thinking lets Claude decide how much to reason; `effort`
    // controls depth. `budget_tokens` is removed on current models.
    let request = MessageBuilder::new()
        .model(models::OPUS_4_8)
        .max_tokens(16000)
        .adaptive_thinking_summarized()
        .effort(OutputEffort::XHigh)  // best for coding/agentic work
        .user("Solve this complex algorithmic problem...")
        .build();
    
    let response = client.messages().create(request, None).await?;
    println!("Solution: {}", response.text());
    
    Ok(())
}
```

### Streaming Responses

```rust
use threatflux_anthropic_sdk::{Client, builders::MessageBuilder};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    let request = MessageBuilder::new()
        .model("claude-haiku-4-5")
        .max_tokens(500)
        .user("Write a short story about a robot learning to paint")
        .stream()  // Enable streaming
        .build();
    
    let mut stream = client.messages().create_stream(request, None).await?;
    
    print!("Claude: ");
    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                if let Some(text) = delta.text {
                    print!("{}", text);
                }
            }
            StreamEvent::MessageStop => break,
            _ => continue,
        }
    }
    println!();
    
    Ok(())
}
```

### Conversation with Context

```rust
use threatflux_anthropic_sdk::{Client, builders::MessageBuilder};
use threatflux_anthropic_sdk::models::common::Role;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    let request = MessageBuilder::new()
        .model("claude-haiku-4-5")
        .max_tokens(800)
        .system("You are a helpful coding mentor specializing in Rust.")
        .conversation(&[
            (Role::User, "I'm new to Rust. What should I learn first?"),
            (Role::Assistant, "Great choice! I'd recommend starting with ownership and borrowing, as these are Rust's key concepts."),
            (Role::User, "Can you give me a simple example of ownership?"),
        ])
        .build();
    
    let response = client.messages().create(request, None).await?;
    println!("{}", response.text());
    
    Ok(())
}
```

### Batch Processing

```rust
use threatflux_anthropic_sdk::{Client, builders::BatchBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    // Create a batch of requests
    let batch = BatchBuilder::new()
        .add_simple_request("req1", "claude-haiku-4-5", "What is 2+2?", 100)
        .add_simple_request("req2", "claude-haiku-4-5", "What is 3+3?", 100)
        .add_creative("story", "claude-haiku-4-5", "Write a haiku about coding", 200)
        .build();
    
    // Submit the batch
    let batch_response = client.message_batches().create(batch, None).await?;
    println!("Batch created: {}", batch_response.id);
    
    // Wait for completion
    let completed = client.message_batches()
        .wait_for_completion(&batch_response.id, 
                           std::time::Duration::from_secs(5),  // poll interval
                           std::time::Duration::from_secs(300)) // max wait
        .await?;
    
    println!("Batch completed with {} requests", completed.request_counts.completed);
    
    Ok(())
}
```

### Image Analysis (Vision)

```rust
use threatflux_anthropic_sdk::{Client, builders::MessageBuilder};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    let request = MessageBuilder::new()
        .model("claude-haiku-4-5")
        .max_tokens(500)
        .user_with_image_file(
            "What do you see in this image?",
            "path/to/your/image.jpg"
        ).await?
        .build();
    
    let response = client.messages().create(request, None).await?;
    println!("Claude sees: {}", response.text());
    
    Ok(())
}
```

### File Operations

```rust
use threatflux_anthropic_sdk::{Client, models::file::FileUploadRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    
    // Upload a file
    let content = std::fs::read("document.pdf")?;
    let request = FileUploadRequest::new(content, "document.pdf", "application/pdf")
        .purpose("user_data");
    
    let file = client.files().upload(request, None).await?;
    println!("File uploaded: {} ({})", file.file.filename, file.file.id);
    
    // List files
    let files = client.files().list(None, None).await?;
    println!("You have {} files", files.data.len());
    
    Ok(())
}
```

### Admin Operations

```rust
use threatflux_anthropic_sdk::{Client, models::admin::MemberCreateRequest, models::admin::MemberRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let admin = client.admin()?; // Requires admin key
    
    // Get organization info
    let org = admin.organization().get(None).await?;
    println!("Organization: {}", org.name);
    
    // List members
    let members = admin.organization().list_members(None, None).await?;
    println!("Members: {}", members.data.len());
    
    // Get usage report
    let usage = admin.usage().get_current_billing_usage(None, None).await?;
    println!("Usage: {} input tokens, {} output tokens", 
             usage.input_tokens, usage.output_tokens);
    
    Ok(())
}
```

## API Coverage

ThreatFlux Anthropic SDK provides broad coverage of the Anthropic API:

### Messages API
- ✅ Create messages
- ✅ Streaming responses
- ✅ Token counting
- ✅ Vision (image analysis)
- ✅ Tool use (function calling)
- ✅ System prompts
- ✅ Conversation history

### Models API
- ✅ List available models
- ✅ Get model details
- ✅ Model capabilities and pricing

### Message Batches API
- ✅ Create batches
- ✅ Retrieve batch status
- ✅ List batches
- ✅ Cancel batches
- ✅ Delete batches
- ✅ Download results

### Files API
- ✅ Upload files
- ✅ List files
- ✅ Get file info
- ✅ Download files
- ✅ Delete files
- ✅ Multiple file formats

### Admin API
- ✅ Organization management
- ✅ Workspace management
- ✅ Member management
- ✅ API key management
- ✅ Usage reporting
- ✅ Billing information

## Advanced Features

### Custom Configuration

```rust
use threatflux_anthropic_sdk::{Config, Client};
use std::time::Duration;

let config = Config::new("your-api-key")?
    .with_timeout(Duration::from_secs(30))
    .with_max_retries(5)
    .with_default_model("claude-3-sonnet-20240229");

let client = Client::new(config);
```

### Error Handling

```rust
use threatflux_anthropic_sdk::{Client, error::AnthropicError};

match client.messages().create(request, None).await {
    Ok(response) => println!("Success: {}", response.text()),
    Err(AnthropicError::RateLimit(msg)) => {
        println!("Rate limited: {}", msg);
        // Implement backoff strategy
    }
    Err(AnthropicError::Api { status, message, .. }) => {
        println!("API error {}: {}", status, message);
    }
    Err(e) => println!("Other error: {}", e),
}
```

### Builder Presets

```rust
use threatflux_anthropic_sdk::builders::MessageBuilder;

// Creative writing preset (high temperature)
let creative = MessageBuilder::new()
    .creative()
    .user("Write a poem about the ocean")
    .build();

// Code generation preset (low temperature, stop sequences)
let code = MessageBuilder::new()
    .code_generation()
    .user("Write a Rust function to reverse a string")
    .build();

// Analytical preset (low temperature, focused)
let analytical = MessageBuilder::new()
    .analytical()
    .user("Analyze the pros and cons of microservices")
    .build();
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
threatflux-anthropic-sdk = "0.1.0"

# Required for async
tokio = { version = "1.0", features = ["full"] }

# Optional: for better error handling
anyhow = "1.0"
```

### Feature Flags

```toml
[dependencies]
threatflux-anthropic-sdk = { version = "0.1.0", default-features = false, features = ["rustls-tls"] }
```

Available features:
- `native-tls` (default): Use system TLS implementation
- `rustls-tls`: Use rustls for TLS (pure Rust)

## Requirements

- **Rust**: 1.95.0 or later
- **Anthropic API Key**: Get one from the [Anthropic Console](https://console.anthropic.com/)
- **Tokio**: For async runtime

## Documentation

- **API Documentation**: [docs.rs/threatflux-anthropic-sdk](https://docs.rs/threatflux-anthropic-sdk)
- **Development Guide**: See [CLAUDE.md](CLAUDE.md)
- **API Reference**: See [API_CURL_DOCS.md](API_CURL_DOCS.md)
- **Examples**: Check the `/examples` directory

## Release Automation

Releases are managed by Release Please from conventional commits on `main`.
When a release PR is merged, the workflow bumps `Cargo.toml`, updates
`CHANGELOG.md`, creates the GitHub Release and tag, publishes the crate to
crates.io, pushes a GHCR image to GitHub Packages, and attaches the `.crate`
archive plus checksums to the release.

Published package targets:

- Crate: `threatflux-anthropic-sdk`
- GitHub Packages image: `ghcr.io/threatflux/anthropic-rust-sdk`

## Testing

```bash
# Unit tests (no API key required)
cargo test --lib

# Integration tests (API key required)
export ANTHROPIC_API_KEY="your-api-key"
cargo test

# Run specific example
export ANTHROPIC_API_KEY="your-api-key"
cargo run --example basic_message
```

## Performance

ThreatFlux Anthropic SDK is designed for high performance:

- **Connection Pooling**: Automatic HTTP connection reuse
- **Streaming**: Low-latency streaming responses
- **Batch Processing**: Efficient bulk operations
- **Rate Limiting**: Built-in request throttling
- **Retry Logic**: Intelligent exponential backoff

## Error Handling

Comprehensive error handling with detailed error types:

```rust
pub enum AnthropicError {
    Http(reqwest::Error),           // Network errors
    Json(serde_json::Error),        // Parsing errors
    Api { status, message, .. },    // API errors
    Auth(String),                   // Authentication errors
    RateLimit(String),              // Rate limiting
    InvalidInput(String),           // Input validation
    // ... more
}
```

## Contributing

Contributions are welcome! Please see our [contributing guidelines](CLAUDE.md#contributing) for details.

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## Security

- **Never commit API keys** to version control
- **Use environment variables** for configuration
- **Rotate keys regularly**
- **Monitor usage** in the Anthropic Console

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- **Issues**: [GitHub Issues](https://github.com/ThreatFlux/anthropic_rust_sdk/issues)
- **Documentation**: [docs.rs/threatflux-anthropic-sdk](https://docs.rs/threatflux-anthropic-sdk)
- **Examples**: See `/examples` directory
- **API Reference**: [Anthropic API Docs](https://docs.anthropic.com/)

## Acknowledgments

- Built with ❤️ by [Wyatt Roersma](https://github.com/wyattroersma), Claude Code, and Codex
- Powered by the [Anthropic API](https://www.anthropic.com/)
- Built on excellent Rust libraries: `tokio`, `reqwest`, `serde`, and more

---

**Made with 🦀 Rust and ⚡ Anthropic Claude**
