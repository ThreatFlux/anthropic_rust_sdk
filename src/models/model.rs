//! Model-related data models

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Information about an available model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Model {
    /// Unique identifier for the model
    pub id: String,
    /// Object type (always "model")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Human-readable display name
    pub display_name: String,
    /// Model description
    pub description: Option<String>,
    /// Maximum context length in tokens
    pub max_tokens: Option<u32>,
    /// Maximum output tokens
    pub max_output_tokens: Option<u32>,
    /// Input cost per token (in cents)
    pub input_cost_per_token: Option<f64>,
    /// Output cost per token (in cents)  
    pub output_cost_per_token: Option<f64>,
    /// Model capabilities
    pub capabilities: Option<Vec<String>>,
    /// When the model was created
    pub created_at: DateTime<Utc>,
    /// When the model was last updated
    pub updated_at: DateTime<Utc>,
    /// Whether the model is deprecated
    pub deprecated: Option<bool>,
    /// Deprecation date if applicable
    pub deprecation_date: Option<DateTime<Utc>>,
}

impl Model {
    /// Check if the model has a specific capability
    pub fn has_capability(&self, capability: &str) -> bool {
        self.capabilities
            .as_ref()
            .map(|caps| caps.contains(&capability.to_string()))
            .unwrap_or(false)
    }

    /// Check if the model supports vision
    pub fn supports_vision(&self) -> bool {
        self.has_capability("vision")
    }

    /// Check if the model supports tool use
    pub fn supports_tools(&self) -> bool {
        self.has_capability("tool_use")
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
    /// Claude 3 family
    Claude3,
    /// Claude 3.5 family
    Claude35,
    /// Legacy models
    Legacy,
    /// Unknown/custom family
    Unknown,
}

impl std::str::FromStr for ModelFamily {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("claude-3.5") {
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
