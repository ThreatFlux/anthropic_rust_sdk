use super::*;

/// Stop reason enumeration
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum StopReason {
    /// Hit maximum tokens limit
    MaxTokens,
    /// Natural end of message
    EndTurn,
    /// Stop sequence encountered
    StopSequence,
    /// Tool use requested
    ToolUse,
    /// Model paused and expects the conversation to continue
    PauseTurn,
    /// Response was declined for safety/policy reasons
    Refusal,
    /// Model context window was exhausted.
    ModelContextWindowExceeded,
    /// An unfamiliar API reason, preserved exactly.
    Unknown(String),
}

/// Structured detail accompanying a `refusal` (and other) stop reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StopDetails {
    /// Detail type (e.g. `"refusal"`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub detail_type: Option<String>,
    /// Policy category, e.g. `"cyber"`, `"bio"`, `"reasoning_extraction"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// Human-readable explanation, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    /// Recommended model to retry with, when provided.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_model: Option<String>,
    /// Opaque token used to redeem fallback credit on a retry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_credit_token: Option<String>,
    /// Whether the refusal token can be redeemed with assistant-prefill form.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_has_prefill_claim: Option<bool>,
    /// Forward-compatible extra fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}
impl StopReason {
    /// The exact API wire value.
    pub fn as_str(&self) -> &str {
        match self {
            Self::MaxTokens => "max_tokens",
            Self::EndTurn => "end_turn",
            Self::StopSequence => "stop_sequence",
            Self::ToolUse => "tool_use",
            Self::PauseTurn => "pause_turn",
            Self::Refusal => "refusal",
            Self::ModelContextWindowExceeded => "model_context_window_exceeded",
            Self::Unknown(value) => value,
        }
    }
}
impl Serialize for StopReason {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for StopReason {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)?.as_str() {
            "max_tokens" => Self::MaxTokens,
            "end_turn" => Self::EndTurn,
            "stop_sequence" => Self::StopSequence,
            "tool_use" => Self::ToolUse,
            "pause_turn" => Self::PauseTurn,
            "refusal" => Self::Refusal,
            "model_context_window_exceeded" => Self::ModelContextWindowExceeded,
            value => Self::Unknown(value.to_owned()),
        })
    }
}
