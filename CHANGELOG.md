# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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