//! Legacy text-completion models (`/v1/complete`).

use serde::{Deserialize, Serialize};

/// Default legacy completion model.
pub const DEFAULT_COMPLETION_MODEL: &str = "claude-2.1";

/// Legacy completion request.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Model that will complete the prompt.
    pub model: String,
    /// Prompt text to complete.
    pub prompt: String,
    /// Maximum tokens to sample in the completion.
    pub max_tokens_to_sample: u32,
    /// Optional stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,
    /// Sampling temperature.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p sampling value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Top-k sampling value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    /// Enable SSE streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
}

impl CompletionRequest {
    /// Create a completion request with prompt and max token budget.
    pub fn new(prompt: impl Into<String>, max_tokens_to_sample: u32) -> Self {
        Self {
            model: DEFAULT_COMPLETION_MODEL.to_string(),
            prompt: prompt.into(),
            max_tokens_to_sample,
            stop_sequences: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stream: None,
        }
    }

    /// Set model.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p.
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set top-k.
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.top_k = Some(top_k);
        self
    }

    /// Add stop sequence.
    pub fn add_stop_sequence(mut self, stop: impl Into<String>) -> Self {
        self.stop_sequences
            .get_or_insert_with(Vec::new)
            .push(stop.into());
        self
    }

    /// Enable/disable streaming.
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
}

/// Legacy completion stop reasons.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompletionStopReason {
    /// Completion stopped due to stop sequence.
    StopSequence,
    /// Completion stopped due to max tokens.
    MaxTokens,
}

/// Legacy completion response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Completion ID.
    pub id: String,
    /// Object type (`completion`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Generated text.
    pub completion: String,
    /// Model used for completion.
    pub model: String,
    /// Stop reason.
    pub stop_reason: Option<CompletionStopReason>,
    /// Stop sequence that ended generation.
    pub stop: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_request_serialization() {
        let request = CompletionRequest::new("\n\nHuman: Hi\n\nAssistant:", 64)
            .model("claude-2.1")
            .temperature(0.7)
            .add_stop_sequence("\n\nHuman:");

        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "claude-2.1");
        assert_eq!(json["max_tokens_to_sample"], 64);
        assert_eq!(json["stop_sequences"][0], "\n\nHuman:");
    }

    #[test]
    fn test_completion_response_deserialization() {
        let response: CompletionResponse = serde_json::from_str(
            r#"{
                "id": "compl_123",
                "type": "completion",
                "completion": "Hello!",
                "model": "claude-2.1",
                "stop_reason": "stop_sequence",
                "stop": "\n\nHuman:"
            }"#,
        )
        .unwrap();

        assert_eq!(response.object_type, "completion");
        assert_eq!(response.completion, "Hello!");
        assert_eq!(
            response.stop_reason,
            Some(CompletionStopReason::StopSequence)
        );
    }
}
