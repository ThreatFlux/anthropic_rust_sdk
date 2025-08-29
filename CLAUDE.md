# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Threatflux is a comprehensive Rust SDK for the Anthropic API, providing async support, streaming capabilities, and full coverage of all Anthropic API endpoints including Messages, Models, Batches, Files, and Admin operations.

## Common Development Commands

### Build and Test
```bash
# Build the project
cargo build

# Run all tests (requires ANTHROPIC_API_KEY)
cargo test

# Run unit tests only (no API key required)
cargo test --lib

# Run integration tests with real API (requires API key)
cargo test --features real_api_tests

# Run a specific test
cargo test test_name -- --nocapture

# Build release version
cargo build --release

# Check code without building
cargo check
```

### Code Quality
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy -- -D warnings

# Check for security vulnerabilities
cargo audit

# Generate documentation
cargo doc --open
```

### Running Examples
```bash
# Requires ANTHROPIC_API_KEY environment variable
cargo run --example basic_message
cargo run --example streaming_message
cargo run --example batch_processing
cargo run --example claude_4_features
cargo run --example token_usage_tracker
```

## Architecture Overview

### Core Components

**Client (`src/client.rs`)**: Central entry point that manages API communication, authentication, and provides access to all API endpoints. Uses builder pattern for configuration.

**API Modules (`src/api/`)**: Each API endpoint has its own module:
- `messages.rs`: Message creation and streaming
- `models.rs`: Model information and listing
- `message_batches.rs`: Batch processing operations
- `files.rs`: File upload/download management
- `admin/`: Organization, workspace, API key, and usage management

**Builders (`src/builders/`)**: Fluent interfaces for constructing requests:
- `MessageBuilder`: Constructs message requests with presets (creative, analytical, code_generation)
- `BatchBuilder`: Builds batch processing requests

**Models (`src/models/`)**: Strongly typed request/response structures matching Anthropic's API schema

**Streaming (`src/streaming/`)**: Server-Sent Events parsing and async stream handling for real-time responses

**Utils (`src/utils/`)**: Cross-cutting concerns:
- `retry.rs`: Exponential backoff retry logic
- `rate_limit.rs`: Governor-based rate limiting
- `http.rs`: HTTP client utilities

### Key Design Patterns

- **Builder Pattern**: All complex requests use builders for ergonomic API
- **Error Propagation**: Custom error types with thiserror for comprehensive error handling
- **Async-First**: Built on tokio with futures for all I/O operations
- **Rate Limiting**: Uses governor crate with configurable limits per endpoint
- **Automatic Retries**: Configurable exponential backoff for transient failures

## API Authentication

The SDK expects API keys via environment variables:
- `ANTHROPIC_API_KEY`: Required for standard API operations
- `ANTHROPIC_ADMIN_KEY`: Required for admin operations (organization/workspace management)

Keys can be loaded from `.env` file or environment.

## Testing Strategy

- **Unit Tests**: Mock HTTP responses using mockito/wiremock
- **Integration Tests**: Test against mock servers
- **Real API Tests**: Optional tests against live API (feature-gated with `real_api_tests`)
- Test files follow the pattern: `tests/unit/*_test.rs`, `tests/integration/*_test.rs`

## Model Constants

Model identifiers are available in `src/config.rs`:
- Claude 4: `OPUS_4_1`, `OPUS_4`, `SONNET_4`
- Claude 3.5: `HAIKU_3_5`, `SONNET_3_5`
- Claude 3: `OPUS_3`, `SONNET_3`, `HAIKU_3`

## Error Handling

The SDK uses a comprehensive error enum (`src/error.rs`):
- `AnthropicError::Http`: Network/HTTP errors
- `AnthropicError::Json`: Serialization errors
- `AnthropicError::Api`: API-specific errors with status codes
- `AnthropicError::RateLimit`: Rate limiting errors
- `AnthropicError::Auth`: Authentication failures

## Streaming Implementation

Message streaming uses Server-Sent Events:
1. Request with `.stream()` flag
2. Parse SSE events in `event_parser.rs`
3. Convert to typed `StreamEvent` enum
4. Expose as async Stream

## Dependency Management

Core dependencies (Cargo.toml):
- `reqwest`: HTTP client with TLS support
- `tokio`: Async runtime
- `serde`/`serde_json`: JSON serialization
- `governor`: Rate limiting
- `backoff`: Retry logic
- `futures`/`tokio-stream`: Stream utilities

---

# Codacy Rules
Configuration for AI behavior when interacting with Codacy's MCP Server

## using any tool that accepts the arguments: `provider`, `organization`, or `repository`
- Use `git remote -v` to determine the correct values dynamically
- Extract provider, organization, and repository from the git remote URL
- Provider mapping:
  - GitHub repositories: provider = "gh"
  - Bitbucket repositories: provider = "bb"
  - GitLab repositories: provider = "gl"
- Parse the organization and repository name from the remote URL

## After ANY successful `edit_file` or `reapply` operation
- YOU MUST IMMEDIATELY run the `codacy_cli_analyze` tool from Codacy's MCP Server for each file that was edited, with:
- `rootPath`: set to the workspace path
- `file`: set to the path of the edited file
- `tool`: leave empty or unset
- If any issues are found in the new edits, propose and apply fixes for them.
- > NOTE: Failure to follow this rule is considered a critical error.

## CRITICAL: Dependencies and Security Checks
- IMMEDIATELY after ANY of these actions:
- Running npm/yarn/pnpm install
- Adding dependencies to package.json
- Adding requirements to requirements.txt
- Adding dependencies to pom.xml
- Adding dependencies to build.gradle
- Any other package manager operations
- You MUST run the `codacy_cli_analyze` tool with:
- `rootPath`: set to the workspace path
- `tool`: set to "trivy"
- `file`: leave empty or unset
- If any vulnerabilities are found because of the newly added packages:
- Stop all other operations
- Propose and apply fixes for the security issues
- Only continue with the original task after security issues are resolved
- EXAMPLE:
- After: npm install react-markdown
- Do: Run codacy_cli_analyze with trivy
- Before: Continuing with any other tasks

## General
- Repeat the relevant steps for each modified file.
- "Propose fixes" means to both suggest and, if possible, automatically apply the fixes.
- You MUST NOT wait for the user to ask for analysis or remind you to run the tool.
- Do not run `codacy_cli_analyze` looking for changes in duplicated code or code complexity metrics.
- Do not run `codacy_cli_analyze` looking for changes in code coverage.
- Do not try to manually install Codacy CLI using either brew, npm, npx, or any other package manager.
- If the Codacy CLI is not installed, just run the `codacy_cli_analyze` tool from Codacy's MCP Server.
- When calling `codacy_cli_analyze`, only send provider, organization and repository if the project is a git repository.

## Whenever a call to a Codacy tool that uses `repository` or `organization` as a parameter returns a 404 error
- Offer to run the `codacy_setup_repository` tool to add the repository to Codacy
- If the user accepts, run the `codacy_setup_repository` tool
- Do not ever try to run the `codacy_setup_repository` tool on your own
- After setup, immediately retry the action that failed (only retry once)
---