//! Common builder traits and validation utilities

use crate::error::AnthropicError;

/// Common validation utilities for builders
pub struct ValidationUtils;

impl ValidationUtils {
    /// Validate max_tokens parameter
    pub fn validate_max_tokens(max_tokens: u32, context: &str) -> Result<(), AnthropicError> {
        if max_tokens == 0 {
            return Err(AnthropicError::invalid_input(format!(
                "{} max_tokens must be greater than 0",
                context
            )));
        }
        Ok(())
    }

    /// Validate temperature parameter
    pub fn validate_temperature(temperature: f32) -> Result<(), AnthropicError> {
        if !(0.0..=1.0).contains(&temperature) {
            return Err(AnthropicError::invalid_input(
                "temperature must be between 0.0 and 1.0",
            ));
        }
        Ok(())
    }

    /// Validate top_p parameter
    pub fn validate_top_p(top_p: f32) -> Result<(), AnthropicError> {
        if !(0.0..=1.0).contains(&top_p) {
            return Err(AnthropicError::invalid_input(
                "top_p must be between 0.0 and 1.0",
            ));
        }
        Ok(())
    }

    /// Validate that messages are not empty
    pub fn validate_messages_not_empty(
        messages_len: usize,
        context: &str,
    ) -> Result<(), AnthropicError> {
        if messages_len == 0 {
            return Err(AnthropicError::invalid_input(format!(
                "{} must contain at least one message",
                context
            )));
        }
        Ok(())
    }

    /// Validate Claude 4 specific constraints
    pub fn validate_claude_4_constraints(
        model: &str,
        temperature: Option<f32>,
        top_p: Option<f32>,
    ) -> Result<(), AnthropicError> {
        if model.starts_with("claude-opus-4-1") {
            // Opus 4.1 cannot use both temperature and top_p simultaneously
            if temperature.is_some() && top_p.is_some() {
                return Err(AnthropicError::invalid_input(
                    "Claude Opus 4.1 cannot use both temperature and top_p simultaneously",
                ));
            }
        }
        Ok(())
    }

    /// Validate thinking configuration
    pub fn validate_thinking_config(
        model: &str,
        budget_tokens: Option<u32>,
    ) -> Result<(), AnthropicError> {
        if let Some(budget) = budget_tokens {
            let max_allowed = crate::config::models::max_thinking_tokens(model).unwrap_or(0);
            if max_allowed > 0 && budget > max_allowed {
                return Err(AnthropicError::invalid_input(format!(
                    "Thinking budget {} exceeds maximum {} for model {}",
                    budget, max_allowed, model
                )));
            }
        }
        Ok(())
    }
}

/// Trait for builders that can be validated before building
pub trait ValidatedBuilder<T> {
    /// Build and validate the request
    fn build_validated(self) -> Result<T, AnthropicError>;
}

/// Trait for builders that support fluent configuration
pub trait FluentBuilder {
    /// Get the current state for inspection (optional, returns None by default)
    fn inspect(&self) -> Option<&dyn std::fmt::Debug> {
        None
    }
}

/// Preset configurations for consistent parameter combinations
#[derive(Debug, Clone, Copy)]
pub struct PresetConfig {
    pub temperature: f32,
    pub top_p: f32,
    pub max_tokens: u32,
}

impl PresetConfig {
    /// Creative writing preset
    pub const CREATIVE: Self = Self {
        temperature: 0.9,
        top_p: 0.95,
        max_tokens: 2000,
    };

    /// Analytical tasks preset
    pub const ANALYTICAL: Self = Self {
        temperature: 0.3,
        top_p: 0.85,
        max_tokens: 1500,
    };

    /// Code generation preset
    pub const CODE_GENERATION: Self = Self {
        temperature: 0.1,
        top_p: 0.9,
        max_tokens: 2000,
    };

    /// Conversational AI preset
    pub const CONVERSATIONAL: Self = Self {
        temperature: 0.7,
        top_p: 0.9,
        max_tokens: 1000,
    };

    /// Apply this preset to a builder with settable parameters
    pub fn apply_to_builder<B: ParameterBuilder>(self, builder: B) -> B {
        builder
            .temperature(self.temperature)
            .top_p(self.top_p)
            .max_tokens(self.max_tokens)
    }
}

/// Trait for builders that support parameter configuration
pub trait ParameterBuilder: Sized {
    fn temperature(self, temperature: f32) -> Self;
    fn top_p(self, top_p: f32) -> Self;
    fn max_tokens(self, max_tokens: u32) -> Self;

    /// Apply a preset configuration
    fn with_preset(self, preset: PresetConfig) -> Self {
        preset.apply_to_builder(self)
    }

    /// Creative writing preset
    fn creative(self) -> Self {
        self.with_preset(PresetConfig::CREATIVE)
    }

    /// Analytical tasks preset
    fn analytical(self) -> Self {
        self.with_preset(PresetConfig::ANALYTICAL)
    }

    /// Code generation preset
    fn code_generation(self) -> Self {
        self.with_preset(PresetConfig::CODE_GENERATION)
    }

    /// Conversational AI preset
    fn conversational(self) -> Self {
        self.with_preset(PresetConfig::CONVERSATIONAL)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_max_tokens() {
        assert!(ValidationUtils::validate_max_tokens(0, "Test").is_err());
        assert!(ValidationUtils::validate_max_tokens(1, "Test").is_ok());
        assert!(ValidationUtils::validate_max_tokens(1000, "Test").is_ok());
    }

    #[test]
    fn test_validate_temperature() {
        assert!(ValidationUtils::validate_temperature(-0.1).is_err());
        assert!(ValidationUtils::validate_temperature(0.0).is_ok());
        assert!(ValidationUtils::validate_temperature(0.5).is_ok());
        assert!(ValidationUtils::validate_temperature(1.0).is_ok());
        assert!(ValidationUtils::validate_temperature(1.1).is_err());
    }

    #[test]
    fn test_validate_top_p() {
        assert!(ValidationUtils::validate_top_p(-0.1).is_err());
        assert!(ValidationUtils::validate_top_p(0.0).is_ok());
        assert!(ValidationUtils::validate_top_p(0.5).is_ok());
        assert!(ValidationUtils::validate_top_p(1.0).is_ok());
        assert!(ValidationUtils::validate_top_p(1.1).is_err());
    }

    #[test]
    fn test_validate_messages_not_empty() {
        assert!(ValidationUtils::validate_messages_not_empty(0, "Test").is_err());
        assert!(ValidationUtils::validate_messages_not_empty(1, "Test").is_ok());
    }

    #[test]
    fn test_validate_claude_4_constraints() {
        // Non-Claude 4 models should allow both
        assert!(ValidationUtils::validate_claude_4_constraints(
            "claude-3-sonnet",
            Some(0.5),
            Some(0.9)
        )
        .is_ok());

        // Claude 4.1 should reject both temperature and top_p
        assert!(ValidationUtils::validate_claude_4_constraints(
            "claude-opus-4-1",
            Some(0.5),
            Some(0.9)
        )
        .is_err());

        // Claude 4.1 should allow either one
        assert!(
            ValidationUtils::validate_claude_4_constraints("claude-opus-4-1", Some(0.5), None)
                .is_ok()
        );
        assert!(
            ValidationUtils::validate_claude_4_constraints("claude-opus-4-1", None, Some(0.9))
                .is_ok()
        );
    }

    #[test]
    fn test_preset_configs() {
        let creative = PresetConfig::CREATIVE;
        assert_eq!(creative.temperature, 0.9);
        assert_eq!(creative.top_p, 0.95);
        assert_eq!(creative.max_tokens, 2000);

        let analytical = PresetConfig::ANALYTICAL;
        assert_eq!(analytical.temperature, 0.3);
        assert_eq!(analytical.top_p, 0.85);
        assert_eq!(analytical.max_tokens, 1500);
    }

    // Mock builder for testing preset application
    #[derive(Debug, Default)]
    struct MockBuilder {
        temperature: Option<f32>,
        top_p: Option<f32>,
        max_tokens: u32,
    }

    impl ParameterBuilder for MockBuilder {
        fn temperature(mut self, temperature: f32) -> Self {
            self.temperature = Some(temperature);
            self
        }

        fn top_p(mut self, top_p: f32) -> Self {
            self.top_p = Some(top_p);
            self
        }

        fn max_tokens(mut self, max_tokens: u32) -> Self {
            self.max_tokens = max_tokens;
            self
        }
    }

    #[test]
    fn test_parameter_builder_presets() {
        let builder = MockBuilder::default().creative();
        assert_eq!(builder.temperature, Some(0.9));
        assert_eq!(builder.top_p, Some(0.95));
        assert_eq!(builder.max_tokens, 2000);

        let builder = MockBuilder::default().analytical();
        assert_eq!(builder.temperature, Some(0.3));
        assert_eq!(builder.top_p, Some(0.85));
        assert_eq!(builder.max_tokens, 1500);

        let builder = MockBuilder::default().code_generation();
        assert_eq!(builder.temperature, Some(0.1));
        assert_eq!(builder.top_p, Some(0.9));
        assert_eq!(builder.max_tokens, 2000);
    }
}
