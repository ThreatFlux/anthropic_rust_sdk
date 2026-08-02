# Anthropic Rust SDK

[![CI](https://github.com/ThreatFlux/anthropic_rust_sdk/actions/workflows/ci.yml/badge.svg)](https://github.com/ThreatFlux/anthropic_rust_sdk/actions/workflows/ci.yml)
[![Documentation](https://docs.rs/threatflux-anthropic-sdk/badge.svg)](https://docs.rs/threatflux-anthropic-sdk)
[![Crates.io](https://img.shields.io/crates/v/threatflux-anthropic-sdk.svg)](https://crates.io/crates/threatflux-anthropic-sdk)
[![MSRV](https://img.shields.io/badge/MSRV-1.95.0-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/crates/l/threatflux-anthropic-sdk.svg)](LICENSE)
[![Dependencies](https://deps.rs/repo/github/ThreatFlux/anthropic_rust_sdk/status.svg)](https://deps.rs/repo/github/ThreatFlux/anthropic_rust_sdk)

An async, typed Rust client for the Anthropic API, with Messages, streaming,
token counting, batches, files, models, administration, and selected beta and
research-preview surfaces.

This is an **unofficial, community-maintained SDK**. It is not developed,
endorsed, or supported by Anthropic. The documentation in this repository
describes the `main` branch; the [crates.io release](https://crates.io/crates/threatflux-anthropic-sdk)
can differ. Anthropic's [API documentation](https://platform.claude.com/docs/en/api/overview)
is authoritative for service behavior and availability.

## Why use it

- Async-first API built on Tokio and Reqwest.
- Typed request, response, error, and SSE event models.
- Fluent builders for messages and batch workloads.
- Configurable timeouts, retries, base URL, and TLS backend.
- Source-level clients for core, administrative, and selected preview APIs.
- Compile-checked examples and rustdoc with warnings denied in CI.

## Requirements

- Rust 1.95.0 or newer (the minimum supported Rust version, or MSRV).
- An Anthropic API key for live requests.
- An Anthropic Admin API key for administration endpoints.

## Installation

The current crates.io release is `0.2.0`. Depend on the compatible `0.2`
series unless you need to pin an exact patch:

```toml
[dependencies]
threatflux-anthropic-sdk = "0.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The Cargo package name uses hyphens; Rust imports use underscores:
`threatflux_anthropic_sdk`.

To test unreleased `main` code, use a Git dependency explicitly:

```toml
[dependencies]
threatflux-anthropic-sdk = { git = "https://github.com/ThreatFlux/anthropic_rust_sdk", branch = "main" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Do not use the Git form when reproducible crates.io releases are required.

## Quickstart

Create an API key in the [Anthropic Console](https://platform.claude.com/settings/keys),
then expose it to the process. Model availability changes independently of this
crate, so the example allows an environment override:

```bash
export ANTHROPIC_API_KEY="your-api-key"
export ANTHROPIC_MODEL="model-id-available-to-your-account" # optional
```

The following program is mirrored by `examples/quickstart.rs` and compiled in
CI on the MSRV and stable Rust.

<!-- BEGIN QUICKSTART -->

```rust
use threatflux_anthropic_sdk::{Client, MessageBuilder, DEFAULT_MODEL};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::from_env()?;
    let model = std::env::var("ANTHROPIC_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());

    let request = MessageBuilder::new()
        .model(model)
        .max_tokens(256)
        .user("Explain Rust ownership in one short paragraph.")
        .build_validated()?;

    let response = client.messages().create(request, None).await?;
    println!("{}", response.text());

    Ok(())
}
```

<!-- END QUICKSTART -->

Run it with:

```bash
cargo run --example quickstart
```

`Client::from_env()` also reads a local `.env` file through `dotenvy`. Keep
that file out of version control and prefer a secrets manager in deployed
environments.

## Cargo features

Choose one TLS backend. The default build uses the platform-native TLS stack.

<!-- BEGIN CARGO FEATURES -->

| Feature | Default | Purpose |
| --- | --- | --- |
| `default` | Yes | Enables the `native-tls` feature. |
| `native-tls` | Yes | Enables Reqwest's platform-native TLS backend. |
| `rustls-tls` | No | Enables Reqwest's Rustls TLS backend. Disable default features when selecting only Rustls. |
| `real_api_tests` | No | Compiles opt-in tests that can call the live Anthropic API and incur usage. |

<!-- END CARGO FEATURES -->

For a Rustls-only build:

```toml
[dependencies]
threatflux-anthropic-sdk = { version = "0.2", default-features = false, features = ["rustls-tls"] }
```

## API surface

The table describes clients present in this source tree. It is not a promise of
complete parity with every current API field or endpoint. See the
[detailed coverage notes](docs/api-coverage.md) before adopting a less common
surface.

| Surface | Source-level support | Entry point |
| --- | --- | --- |
| Messages | Create messages, stream SSE events, and count input tokens | `client.messages()` |
| Models | List, retrieve, and locally filter models | `client.models()` |
| Message batches | Create, retrieve, list, cancel, delete, fetch results, and poll | `client.message_batches()` |
| Files | Upload, list, retrieve, download, and delete | `client.files()` |
| Skills | Skill and version lifecycle helpers | `client.skills()` |
| Administration | Organization users/invites/members, workspaces, API-key listing/updating, and usage reports | `client.admin()?` |
| Text completions | A legacy completion client for existing integrations | `client.completions()` |

The repository also contains clients for Dreams, MCP Tunnels, User Profiles,
and Managed Agents resources. Treat these as beta or research-preview surfaces:
availability may be account-specific, schemas may change, and the appropriate
`RequestOptions` beta header may be required.

## Configuration

`Client::from_env()` recognizes these variables:

| Variable | Default | Meaning |
| --- | --- | --- |
| `ANTHROPIC_API_KEY` | Required | Standard API credential. |
| `ANTHROPIC_ADMIN_KEY` | Unset | Admin credential used by `client.admin()`. |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | API origin or compatible proxy base; the SDK appends `/v1`. |
| `ANTHROPIC_TIMEOUT` | `60` | Per-attempt request timeout in seconds. |
| `ANTHROPIC_MAX_RETRIES` | `3` | Retry attempts after the initial non-streaming request. |
| `ANTHROPIC_DEFAULT_MODEL` | SDK default | Stored in `Config.default_model`; pass it to a builder to apply the override. |
| `ANTHROPIC_ENABLE_RATE_LIMITING` | `true` | Stored rate-limiter setting; see the operational caveat below. |
| `ANTHROPIC_RATE_LIMIT_RPS` | `50` | Stored requests-per-second setting; see the operational caveat below. |

Programmatic configuration is available through `Config`:

```rust
use std::time::Duration;
use threatflux_anthropic_sdk::{Client, Config, Result};

fn configured_client(api_key: &str) -> Result<Client> {
    let config = Config::new(api_key)?
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(2);
    Client::try_new(config)
}
```

Important current behavior:

- Non-streaming calls retry connection/request failures, timeouts, HTTP 429,
  and selected 5xx responses. `max_retries = 3` permits up to four attempts.
- Streaming requests are not automatically retried or resumed.
- `RequestOptions::no_retry()` disables retries for one non-streaming call.
- Retries can duplicate a non-idempotent operation if the server accepted a
  request before the client observed a failure. Choose retry settings with the
  endpoint's semantics in mind.
- The rate-limiter types in `utils::rate_limit` and the related `Config` fields
  are not automatically applied by `Client` in version 0.2.0. Enforce
  application-level concurrency or rate limits where required.
- `MessageBuilder` initializes from the crate's `DEFAULT_MODEL`. To use
  `ANTHROPIC_DEFAULT_MODEL`, pass `client.config().default_model.clone()` to
  `.model(...)`.

See [configuration and operations](docs/configuration.md) for request options,
errors, retries, preview headers, proxy considerations, and production safety.

## Models and beta features

Model IDs are strings. `config::models` provides convenience constants, but
the service catalog can change between crate releases. Check Anthropic's
[models documentation](https://platform.claude.com/docs/en/about-claude/models/overview)
and use a model available to your account.

Beta headers are explicit through `RequestOptions`, for example:

```rust
use threatflux_anthropic_sdk::{AnthropicError, Client, RequestOptions};

async fn list_files(client: &Client) -> Result<(), AnthropicError> {
    let options = RequestOptions::new().with_files_api();
    let files = client.files().list(None, Some(options)).await?;
    println!("{} file(s)", files.data.len());
    Ok(())
}
```

Review the corresponding API module before enabling preview helpers; some
methods add their required header automatically while others accept caller
options.

## Examples

All examples require `ANTHROPIC_API_KEY` at runtime unless noted otherwise.

| Example | Demonstrates | Command |
| --- | --- | --- |
| [`quickstart.rs`](examples/quickstart.rs) | Minimal message request | `cargo run --example quickstart` |
| [`basic_message.rs`](examples/basic_message.rs) | Messages, conversations, presets, and token counting | `cargo run --example basic_message` |
| [`streaming_message.rs`](examples/streaming_message.rs) | SSE events and collection helpers | `cargo run --example streaming_message` |
| [`batch_processing.rs`](examples/batch_processing.rs) | Batch creation, polling, results, and cancellation | `cargo run --example batch_processing` |
| [`claude_4_features.rs`](examples/claude_4_features.rs) | Thinking, caching, tools, structured output, and beta options | `cargo run --example claude_4_features` |
| [`token_usage_tracker.rs`](examples/token_usage_tracker.rs) | Local token accounting sample | `cargo run --example token_usage_tracker` |

Pricing embedded in an example is illustrative, not a billing authority. Check
Anthropic's current pricing before using it for cost reporting.

## Errors

API operations return `threatflux_anthropic_sdk::Result<T>`, whose error type is
`AnthropicError`. Match specific variants when behavior differs by failure
class, and use `status_code()` or `is_retryable()` when appropriate:

```rust
use threatflux_anthropic_sdk::{AnthropicError, Client, MessageRequest};

async fn send(client: &Client, request: MessageRequest) -> Result<(), AnthropicError> {
    match client.messages().create(request, None).await {
        Ok(message) => println!("{}", message.text()),
        Err(AnthropicError::Api { status: 429, .. }) => eprintln!("rate limited"),
        Err(error) if error.is_retryable() => eprintln!("retryable failure: {error}"),
        Err(error) => return Err(error),
    }
    Ok(())
}
```

## Documentation

- [API documentation](https://docs.rs/threatflux-anthropic-sdk)
- [API coverage and maturity](docs/api-coverage.md)
- [Configuration and operations](docs/configuration.md)
- [Endpoint request notes](API_CURL_DOCS.md)
- [Changelog](CHANGELOG.md)
- [Contributing guide](CONTRIBUTING.md)
- [Security policy](SECURITY.md)
- [Official Anthropic API documentation](https://platform.claude.com/docs/en/api/overview)

## Development

The fast, credential-free validation path is:

```bash
cargo +1.95.0 fmt --all -- --check
cargo +1.95.0 clippy --all-targets --all-features -- -D warnings
cargo +1.95.0 test --test unit_suite
python3 scripts/check_docs.py
```

Live API tests are opt-in and can incur usage. See [CONTRIBUTING.md](CONTRIBUTING.md)
for the complete workflow and release process.

## Security

Never commit API keys, admin keys, captured authorization headers, or unredacted
customer prompts and responses. A custom `ANTHROPIC_BASE_URL` receives the
configured credential, so use only a trusted endpoint. Report vulnerabilities
privately according to [SECURITY.md](SECURITY.md).

## Support and license

Use [GitHub Issues](https://github.com/ThreatFlux/anthropic_rust_sdk/issues) for
reproducible SDK bugs and feature requests. Use Anthropic's support channels for
account, billing, model-access, and service-availability questions.

This project is available under the [MIT License](LICENSE).
