//! Real API tests
//!
//! These tests require actual API credentials and test against the real Anthropic API.
//! They are gated behind the `real_api_tests` feature flag.
//!
//! To run these tests:
//! 1. Set ANTHROPIC_API_KEY environment variable
//! 2. Set RUN_REAL_API_TESTS=true environment variable  
//! 3. Run: cargo test --features real_api_tests
//!
//! Note: These tests use claude-3-5-haiku-20241022 for speed and cost efficiency.

mod real_messages_test;

#[cfg(feature = "real_api_tests")]
pub use real_messages_test::*;