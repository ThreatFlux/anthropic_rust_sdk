# Configuration and operations

This guide describes behavior implemented by the current `main` source.
Published releases can differ; select the matching crate version in the
[docs.rs version menu](https://docs.rs/crate/threatflux-anthropic-sdk). This
guide complements the type-level API documentation.

## Constructing a client

For applications, prefer `Client::from_env()` or `Client::try_new(config)` so
invalid configuration is returned as an error:

```rust
use std::time::Duration;
use threatflux_anthropic_sdk::{Client, Config, Result};

fn configured_client(api_key: &str) -> Result<Client> {
    let config = Config::new(api_key)?
        .with_timeout(Duration::from_secs(45))
        .with_max_retries(2)
        .with_user_agent("my-service/1.0");
    Client::try_new(config)
}
```

`Client::new(config)` panics when validation fails. `Config::default()` contains
a non-secret placeholder credential and is intended for local construction and
tests, not live requests.

## Environment variables

`Config::from_env()` loads a local `.env` file when present and then reads:

| Variable | Default | Behavior |
| --- | --- | --- |
| `ANTHROPIC_API_KEY` | Required | Standard API credential. Values beginning with `sk-ant-` are sent as `x-api-key`; other values are sent as a bearer token. |
| `ANTHROPIC_ADMIN_KEY` | Unset | Separate credential used for all calls made through `client.admin()?`. |
| `ANTHROPIC_BASE_URL` | `https://api.anthropic.com` | Base origin. The client appends `/v1` and the resource path. |
| `ANTHROPIC_TIMEOUT` | `60` | Per-attempt timeout in seconds. Invalid numeric values fall back to the default. |
| `ANTHROPIC_MAX_RETRIES` | `3` | Additional attempts for eligible non-streaming failures. Invalid numeric values fall back to the default. |
| `ANTHROPIC_DEFAULT_MODEL` | `DEFAULT_MODEL` | Stored in `Config.default_model`; it is not injected automatically into an independently created `MessageBuilder`. |
| `ANTHROPIC_ENABLE_RATE_LIMITING` | `true` | Stored in `Config`; the core client does not currently wire this value into request dispatch. |
| `ANTHROPIC_RATE_LIMIT_RPS` | `50` | Stored in `Config`; the core client does not currently wire this value into request dispatch. |

The environment parser accepts any value Rust can parse for the numeric and
boolean fields. Validate deployment configuration before starting if silently
falling back would be undesirable.

## Model selection

`MessageBuilder::new()` starts with the crate's compile-time `DEFAULT_MODEL`.
To make the environment-configured value effective, apply it explicitly:

```rust
use threatflux_anthropic_sdk::{AnthropicError, Client, MessageBuilder, MessageRequest};

fn request_from_config(client: &Client) -> Result<MessageRequest, AnthropicError> {
    MessageBuilder::new()
        .model(client.config().default_model.clone())
        .max_tokens(256)
        .user("Hello")
        .build_validated()
}
```

The constants in `config::models` are convenience snapshots. Model access and
retirement are service-side concerns, so accept model IDs through application
configuration when deployments need to change them without a rebuild.

## Per-request options

Individual resource request methods accept `Option<RequestOptions>`. Options
can override the timeout, disable retry, add headers, or opt into a beta
feature:

```rust
use std::time::Duration;
use threatflux_anthropic_sdk::{AnthropicError, Client, MessageRequest, RequestOptions};

async fn send_once(client: &Client, request: MessageRequest) -> Result<(), AnthropicError> {
    let options = RequestOptions::new()
        .with_timeout(Duration::from_secs(20))
        .no_retry()
        .with_beta_feature("feature-version-from-official-documentation");

    client.messages().create(request, Some(options)).await?;
    Ok(())
}
```

Custom headers are inserted after the SDK's standard headers and can replace
them. Do not accept arbitrary header names or values from untrusted callers.

Polling and convenience helpers can have a narrower signature. In particular,
message-batch `wait_for_completion` and managed-agent session
`wait_until_idle` call their underlying retrieval methods with `None`; callers
cannot use those helpers to override request options. Poll manually with
`retrieve` or `get` when per-request options are required.

## Retries and idempotency

Non-streaming `Client::request` and `Client::request_admin` calls retry:

- request, connection, and timeout errors reported by Reqwest;
- HTTP 429 responses; and
- HTTP 500, 502, 503, and 504 responses.

`max_retries` counts attempts after the first request. The default of `3`
therefore allows four total attempts. Backoff begins around one second and is
capped at 60 seconds per delay. The request timeout applies to each attempt, so
total wall-clock time can exceed the configured timeout.

The retry path does not know whether a failed mutation was accepted by the
service. For creates, deletes, batch submissions, and other non-idempotent
operations, decide whether duplicate effects are acceptable. Use
`RequestOptions::no_retry()` when the application must make that decision
itself.

Streaming calls bypass the retry client. The SDK does not reconnect, resume, or
replay a partially consumed stream.

## Rate limiting and concurrency

The crate exposes standalone rate-limiter utilities in `utils::rate_limit`, and
`Config` contains rate-limit fields. In the current source those fields are not
applied automatically by the core `Client`. Production callers should enforce
their own concurrency and throughput policy and still handle service-side 429
responses.

## Timeouts and polling

- `Config.timeout` and `RequestOptions::with_timeout` cover an individual HTTP
  attempt.
- Batch `wait_for_completion` and session `wait_until_idle` accept their own
  polling interval and total polling timeout.
- A polling timeout does not cancel the remote operation.
- Application shutdown should stop outstanding tasks and decide whether remote
  operations need explicit cancellation.

## Error handling

`AnthropicError` distinguishes API, authentication, configuration, rate-limit,
timeout, network, stream, file, I/O, decoding, and catch-all failures. Useful
helpers include:

- `status_code()` for an HTTP status when one is available;
- `is_retryable()` for the crate's local retry classification;
- `is_client_error()` and `is_server_error()` for API status families; and
- `with_context()` for variants whose message can be enriched.

Do not assume that `is_retryable()` makes an operation semantically safe to
repeat. That helper classifies the failure, not the endpoint.

## Authentication and secret handling

- Never commit `.env` files, credentials, authorization headers, or captured
  production payloads.
- `Config` derives `Debug` and includes its credential fields. Do not log a
  `Config` value or expose it in panic/error telemetry.
- Keep the admin key separate from the standard API key and grant only the
  permissions the application requires.
- Custom tracing subscribers and HTTP diagnostics must redact secrets and
  sensitive prompt, response, file, and tool data.
- Rotate a credential immediately if it appears in source control, logs, test
  fixtures, shell history, or a pull request.

## Proxies and custom base URLs

`ANTHROPIC_BASE_URL` is useful for a trusted API-compatible gateway or test
server. The SDK sends the configured credential to that host and appends `/v1`,
so:

- configure the origin or gateway prefix expected before `/v1`;
- require HTTPS outside controlled local tests;
- do not let an untrusted tenant choose the base URL; and
- verify the gateway's logging, retention, certificate, and forwarding policy.

This client targets the Anthropic HTTP shape. A base URL alone does not add the
authentication or request transformations required by a different provider.

## TLS backends

The default `native-tls` feature uses the platform-native backend. To add the
crate with Rustls only:

```bash
cargo add threatflux-anthropic-sdk --no-default-features --features rustls-tls
```

Build and test the chosen feature set on every target platform used in
production.

## Beta and preview headers

`RequestOptions` includes named helpers for several beta surfaces and a generic
`with_beta_feature` escape hatch. Beta version strings can change. Copy them
from current official documentation, keep their use close to the endpoint call,
and test response deserialization before rollout.

The source also contains clients labeled beta or research preview. Treat their
types as less stable than core resource types even when Rust's semantic version
rules would otherwise allow an upgrade.

## Live-test safety

The `real_api_tests` Cargo feature can enable calls to the live service. Before
running it:

1. Use a dedicated, least-privilege test credential.
2. Confirm the selected model and account limits.
3. Remove production data from fixtures.
4. Expect usage and possible charges.
5. Revoke or rotate the credential after accidental exposure.
