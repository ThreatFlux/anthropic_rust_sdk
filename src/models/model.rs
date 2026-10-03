//! Model-related data models

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Information about an available model.
///
/// Field set is a superset of the Anthropic Models API response so that both
/// the list endpoint (`id`, `type`, `display_name`, `created_at`) and the
/// retrieve endpoint (`max_input_tokens`, `max_tokens`, nested `capabilities`)
/// deserialize. Fields absent from a given response default to `None`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Unique identifier for the model
    pub id: String,
    /// Object type (always "model")
    #[serde(rename = "type", default)]
    pub object_type: String,
    /// Human-readable display name
    #[serde(default)]
    pub display_name: String,
    /// Model description
    #[serde(default)]
    pub description: Option<String>,
    /// Maximum context window in tokens (Models API: `max_input_tokens`)
    #[serde(default)]
    pub max_input_tokens: Option<u32>,
    /// Maximum output tokens (Models API: `max_tokens`)
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Maximum output tokens (legacy alias; populated from custom responses)
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    /// Input cost per token (in cents), when provided
    #[serde(default)]
    pub input_cost_per_token: Option<f64>,
    /// Output cost per token (in cents), when provided
    #[serde(default)]
    pub output_cost_per_token: Option<f64>,
    /// Full capability metadata, including unknown and conditional feature data.
    #[serde(default)]
    pub capabilities: Option<ModelCapabilities>,
    /// When the model was created (synthesized if absent)
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    /// When the model was last updated (synthesized if absent)
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
    /// Whether the model is deprecated
    #[serde(default)]
    pub deprecated: Option<bool>,
    /// Deprecation date if applicable
    #[serde(default)]
    pub deprecation_date: Option<DateTime<Utc>>,
}

/// A single authoritative capability payload, retaining nested metadata.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(transparent)]
pub struct ModelCapabilities(serde_json::Value);

impl ModelCapabilities {
    /// Read the complete Models API payload without guessing unsupported fields.
    pub fn as_json(&self) -> &serde_json::Value {
        &self.0
    }

    /// Distinguish explicit support, explicit rejection, and unknown metadata.
    pub fn support(&self, capability: &str) -> crate::config::models::Support {
        use crate::config::models::Support;
        match &self.0 {
            serde_json::Value::Array(values) => {
                if values
                    .iter()
                    .any(|value| value.as_str() == Some(capability))
                {
                    Support::Supported
                } else {
                    Support::Unknown
                }
            }
            serde_json::Value::Object(values) => match values
                .get(capability)
                .and_then(|value| value.get("supported"))
                .and_then(serde_json::Value::as_bool)
            {
                Some(true) => Support::Supported,
                Some(false) => Support::Unsupported,
                None => Support::Unknown,
            },
            _ => Support::Unknown,
        }
    }

    /// Return true only for explicitly supported capabilities.
    pub fn contains(&self, capability: &str) -> bool {
        self.support(capability) == crate::config::models::Support::Supported
    }
}

impl From<Vec<String>> for ModelCapabilities {
    fn from(names: Vec<String>) -> Self {
        Self(serde_json::Value::Array(
            names.into_iter().map(serde_json::Value::String).collect(),
        ))
    }
}

impl<'de> Deserialize<'de> for ModelCapabilities {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        if !(value.is_object()
            || value
                .as_array()
                .is_some_and(|array| array.iter().all(serde_json::Value::is_string)))
        {
            return Err(serde::de::Error::custom(
                "Model capabilities require an object or string array",
            ));
        }
        Ok(Self(value))
    }
}

impl Model {
    /// Check if the model has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities
            .as_ref()
            .map(|caps| caps.contains(capability))
            .unwrap_or(false)
    }

    /// Check if the model supports vision
    pub fn supports_vision(&self) -> bool {
        self.has_capability("vision") || self.has_capability("image_input")
    }

    /// Check if the model supports tool use
    pub fn supports_tools(&self) -> bool {
        self.has_capability("tool_use") || self.has_capability("tools")
    }

    /// The model's context window in tokens, if known.
    pub fn context_window(&self) -> Option<u32> {
        self.max_input_tokens
    }

    /// Check if the model is deprecated
    pub fn is_deprecated(&self) -> bool {
        self.deprecated.unwrap_or(false)
    }

    /// Get the cost per 1000 tokens for input
    pub fn input_cost_per_1k_tokens(&self) -> Option<f64> {
        self.input_cost_per_token.map(|cost| cost * 1000.0)
    }

    /// Get the cost per 1000 tokens for output
    pub fn output_cost_per_1k_tokens(&self) -> Option<f64> {
        self.output_cost_per_token.map(|cost| cost * 1000.0)
    }

    /// Calculate estimated cost for a request
    pub fn estimate_cost(&self, input_tokens: u32, output_tokens: u32) -> Option<f64> {
        let input_cost = self.input_cost_per_token? * input_tokens as f64;
        let output_cost = self.output_cost_per_token? * output_tokens as f64;
        Some(input_cost + output_cost)
    }
}

/// Response when listing models
pub type ModelListResponse = PaginatedResponse<Model>;

/// Model comparison information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelComparison {
    /// Model being compared
    pub model: Model,
    /// Performance score (if available)
    pub score: Option<f64>,
    /// Benchmark results
    pub benchmarks: Option<Vec<BenchmarkResult>>,
}

/// Benchmark result for a model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Score achieved
    pub score: f64,
    /// Maximum possible score
    pub max_score: Option<f64>,
    /// Units for the score
    pub units: Option<String>,
    /// When the benchmark was run
    pub run_date: Option<DateTime<Utc>>,
}

/// Model family information
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelFamily {
    /// Claude Fable / Mythos family
    Fable,
    /// Claude 5 family.
    Claude5,
    /// Claude 4 family (4.0–4.8)
    Claude4,
    /// Claude 3.5 / 3.7 family
    Claude35,
    /// Claude 3 family
    Claude3,
    /// Legacy models
    Legacy,
    /// Unknown/custom family
    Unknown,
}

impl std::str::FromStr for ModelFamily {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("claude-fable") || s.starts_with("claude-mythos") {
            Ok(Self::Fable)
        } else if s.starts_with("claude-opus-5")
            || s.starts_with("claude-sonnet-5")
            || s.starts_with("claude-haiku-5")
        {
            Ok(Self::Claude5)
        } else if s.starts_with("claude-opus-4")
            || s.starts_with("claude-sonnet-4")
            || s.starts_with("claude-haiku-4")
        {
            Ok(Self::Claude4)
        } else if s.starts_with("claude-3-5") || s.starts_with("claude-3-7") {
            Ok(Self::Claude35)
        } else if s.starts_with("claude-3") {
            Ok(Self::Claude3)
        } else if s.starts_with("claude-") {
            Ok(Self::Legacy)
        } else {
            Ok(Self::Unknown)
        }
    }
}

/// Model size/tier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelSize {
    /// Haiku models (fast, lightweight)
    Haiku,
    /// Sonnet models (balanced performance)
    Sonnet,
    /// Opus models (most capable)
    Opus,
    /// Unknown size
    Unknown,
}

impl std::str::FromStr for ModelSize {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.contains("haiku") {
            Ok(Self::Haiku)
        } else if s.contains("sonnet") {
            Ok(Self::Sonnet)
        } else if s.contains("opus") {
            Ok(Self::Opus)
        } else {
            Ok(Self::Unknown)
        }
    }
}

impl Model {
    /// Get the model family
    pub fn family(&self) -> ModelFamily {
        self.id.parse().unwrap_or(ModelFamily::Unknown)
    }

    /// Get the model size/tier
    pub fn size(&self) -> ModelSize {
        self.id.parse().unwrap_or(ModelSize::Unknown)
    }

    /// Check if this model is suitable for a given use case
    pub fn is_suitable_for(&self, use_case: &str) -> bool {
        match use_case.to_lowercase().as_str() {
            "vision" | "image" | "visual" => self.supports_vision(),
            "tools" | "function_calling" | "functions" => self.supports_tools(),
            "fast" | "lightweight" | "quick" => matches!(self.size(), ModelSize::Haiku),
            "balanced" | "general" => matches!(self.size(), ModelSize::Sonnet),
            "complex" | "advanced" | "capable" => matches!(self.size(), ModelSize::Opus),
            _ => true, // Default to suitable for general use
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_retrieve_response_with_capability_object() {
        // Shape returned by GET /v1/models/{id}: nested capability tree, no
        // `updated_at`, and `max_input_tokens` for the context window.
        let model: Model = serde_json::from_value(json!({
            "id": "claude-opus-4-8",
            "type": "model",
            "display_name": "Claude Opus 4.8",
            "max_input_tokens": 1_000_000,
            "max_tokens": 128_000,
            "capabilities": {
                "image_input": {"supported": true},
                "structured_outputs": {"supported": true},
                "thinking": {"supported": true}
            }
        }))
        .unwrap();
        assert_eq!(model.context_window(), Some(1_000_000));
        assert_eq!(model.max_tokens, Some(128_000));
        assert!(model.supports_vision());
        assert_eq!(model.family(), ModelFamily::Claude4);
    }

    #[test]
    fn test_list_response_minimal_fields() {
        // Shape returned by GET /v1/models items: no capabilities, no updated_at.
        let model: Model = serde_json::from_value(json!({
            "id": "claude-haiku-4-5",
            "type": "model",
            "display_name": "Claude Haiku 4.5",
            "created_at": "2025-10-01T00:00:00Z"
        }))
        .unwrap();
        assert_eq!(model.id, "claude-haiku-4-5");
        assert_eq!(model.size(), ModelSize::Haiku);
    }

    #[test]
    fn test_capabilities_string_array_still_supported() {
        let model: Model = serde_json::from_value(json!({
            "id": "x",
            "capabilities": ["vision", "tool_use"]
        }))
        .unwrap();
        assert!(model.supports_vision());
        assert!(model.supports_tools());
    }
}
