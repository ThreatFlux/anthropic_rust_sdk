# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0](https://github.com/ThreatFlux/anthropic_rust_sdk/compare/v0.1.0...v0.2.0) (2026-06-24)


### Features

* complete API parity and move linux workflows to self-hosted ([c40b217](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/c40b2171c9197efefa9fcbe3c0b92cd003c89367))


### Bug Fixes

* align with current Anthropic API for auth + Batch round-trip ([62990a9](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/62990a9868ae2a893f4e3d3a085c3edc0f8d66e3))
* **ci:** route pull request linux jobs to hosted runners ([723eb04](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/723eb0456c18ec03391c661b9297cbb8e34eb62b))
* **ci:** route pull request linux jobs to hosted runners ([134edb4](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/134edb4989a4786984dbef800574cb51e633de55))
* **ci:** satisfy newer stable clippy ([5c5aae4](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/5c5aae45c6cf66d151e9ace0c1666029f2df3637))
* **ci:** skip benchmark publish when no benchmark output ([e14c9fb](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/e14c9fbcd83f20f17d3c67fce1b0012c47daf434))
* **ci:** stabilize pipeline after dependency upgrade ([33b16dd](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/33b16dd4e78d1a01611514867d5df7e1f0eee929))
* **dependabot:** remove invalid org assignee ([328d162](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/328d16260255abe098509df26a980aa222587472))
* **dependabot:** remove invalid org assignee ([59a91d3](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/59a91d3f6336acd83db9b5c652126bd30b44d39a))
* publish anthropic sdk under threatflux crate ([e5e0522](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/e5e052212c7caa0d8b49bc86f0d5ad818c035b5c))
* **security:** harden dependencies and GitHub workflows ([656ca2b](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/656ca2be6affe9a46378501f57f042723997ee3e))
* **skills:** reject symlinks in directory uploads ([7fc8695](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/7fc86959dbc1a4cc500129c2c04f9dc646963d39))
* **skills:** reject symlinks in directory uploads ([e06c15b](https://github.com/ThreatFlux/anthropic_rust_sdk/commit/e06c15b27493f9b44c5d31296b9864c0e1051c39))

## [Unreleased]

### Added (API currency upgrade)
- Current model catalog: `FABLE_5`, `MYTHOS_5`, `OPUS_4_8`, `OPUS_4_7`, `OPUS_4_6`,
  `SONNET_4_6`, `HAIKU_4_5` (plus active legacy `OPUS_4_5`, `SONNET_4_5`, `OPUS_4_1`);
  default model is now `claude-sonnet-4-6`.
- Adaptive thinking (`ThinkingConfig::adaptive` / `adaptive_summarized`, `display`) and
  builder helpers `adaptive_thinking()` / `adaptive_thinking_summarized()`.
- `OutputEffort::XHigh` and agentic `TaskBudget` (`OutputConfig::with_task_budget`).
- Prompt caching wiring: `cache_control` on text/tool/system blocks, cacheable
  `SystemPrompt`/`SystemBlock`, top-level `auto_cache()`, and 1-hour TTL.
- Server-side tool constructors: `Tool::web_search/web_fetch/code_execution/bash/
  text_editor/memory`, plus `strict` and per-tool `cache_control`.
- Refusal handling: `MessageResponse::stop_details` / `is_refusal()`, server-side
  `fallbacks` parameter, `fallback` content block, and new beta-header helpers.
- Models API parity: `Model` now deserializes real list/retrieve responses
  (`max_input_tokens`, nested `capabilities` object, optional `updated_at`).

### Changed
- Retired model constants (`OPUS_4`, `SONNET_4`, `SONNET_3_7`, `HAIKU_3_5`,
  `SONNET_3_5`, `OPUS_3`) marked `#[deprecated]`; examples/docs updated to current models.

### Added
- Initial release of the Anthropic Rust SDK
- Full support for Claude 4 models (Opus 4.1, Opus 4, Sonnet 4)
- Complete API coverage including Messages, Models, Batches, Files, and Admin operations
- Async/await support with tokio runtime
- Server-Sent Events (SSE) streaming for real-time responses
- Builder pattern for constructing API requests
- Automatic retry logic with exponential backoff
- Rate limiting with governor crate
- Comprehensive error handling
- Extended thinking support (up to 64K tokens)
- 1M context window support for Sonnet 4
- Hybrid reasoning modes
- File upload and download capabilities
- Organization and workspace management
- API key management
- Usage tracking and analytics

### Security
- Secure API key handling through environment variables
- TLS/SSL support for all API communications
- Input validation and sanitization

## [0.1.0] - 2024-08-29

### Added
- Initial public release
- Core API functionality
- Basic documentation and examples
- CI/CD pipeline with GitHub Actions
- Code coverage with tarpaulin
- Security auditing with cargo-audit
- Automated dependency updates with Dependabot

### Fixed
- Beta headers logic bug in client.rs
- Code duplication across modules (850+ lines eliminated)
- Clippy warnings and linting issues
- Error handling improvements

### Changed
- Improved builder pattern validation
- Enhanced streaming performance
- Optimized rate limiting with RwLock

### Documentation
- Comprehensive README with examples
- API documentation
- Code examples for all major features
- Contributing guidelines

[Unreleased]: https://github.com/ThreatFlux/anthropic_rust_sdk/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ThreatFlux/anthropic_rust_sdk/releases/tag/v0.1.0
