//! Managed Agents — Session event models (beta: managed-agents-2026-04-01)

use crate::models::common::ContentBlock;
use crate::models::managed_agents::session::SessionStopReason;
use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Common fields present on every agent-originated session event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEventMeta {
    /// Unique event identifier.
    pub id: String,
    /// When the event was processed.
    pub processed_at: DateTime<Utc>,
}

/// A heterogeneous session event delivered over the events stream or list.
///
/// The wire `type` field selects the variant. Dotted names (`agent.message`,
/// `session.status_idle`) cannot be produced by `rename_all`, so each variant
/// carries an explicit `#[serde(rename = "...")]`. Unknown event types
/// deserialize to [`SessionEvent::Unknown`] so the stream never hard-fails.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SessionEvent {
    // ---- received (agent-originated) ----
    /// The agent emitted a message.
    #[serde(rename = "agent.message")]
    AgentMessage {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Message content blocks.
        #[serde(default)]
        content: Vec<ContentBlock>,
    },
    /// The agent emitted thinking text.
    #[serde(rename = "agent.thinking")]
    AgentThinking {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Thinking text.
        #[serde(default)]
        thinking: String,
    },
    /// The agent invoked a tool.
    #[serde(rename = "agent.tool_use")]
    AgentToolUse {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Tool-use identifier.
        tool_use_id: String,
        /// Tool name.
        name: String,
        /// Tool input.
        #[serde(default)]
        input: serde_json::Value,
    },
    /// A tool produced a result.
    #[serde(rename = "agent.tool_result")]
    AgentToolResult {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Tool-use identifier this result corresponds to.
        tool_use_id: String,
        /// Tool result content.
        #[serde(default)]
        content: serde_json::Value,
    },
    /// The agent invoked an MCP tool.
    #[serde(rename = "agent.mcp_tool_use")]
    AgentMcpToolUse {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// The agent invoked a custom (client) tool.
    #[serde(rename = "agent.custom_tool_use")]
    AgentCustomToolUse {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// The session entered the running state.
    #[serde(rename = "session.status_running")]
    StatusRunning {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
    },
    /// The session went idle.
    #[serde(rename = "session.status_idle")]
    StatusIdle {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Why the session went idle.
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_reason: Option<SessionStopReason>,
    },
    /// The session was rescheduled onto compute.
    #[serde(rename = "session.status_rescheduled")]
    StatusRescheduled {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
    },
    /// The session terminated.
    #[serde(rename = "session.status_terminated")]
    StatusTerminated {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
    },
    /// The session reported an error.
    #[serde(rename = "session.error")]
    SessionError {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Error payload.
        #[serde(default)]
        error: serde_json::Value,
    },
    /// A model request span started.
    #[serde(rename = "span.model_request_start")]
    SpanModelRequestStart {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// A model request span ended.
    #[serde(rename = "span.model_request_end")]
    SpanModelRequestEnd {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// An outcome-evaluation span started.
    #[serde(rename = "span.outcome_evaluation_start")]
    SpanOutcomeEvalStart {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// An outcome-evaluation span ended.
    #[serde(rename = "span.outcome_evaluation_end")]
    SpanOutcomeEvalEnd {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Additional fields not yet modeled explicitly.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// A multiagent thread started.
    #[serde(rename = "session.thread_started")]
    ThreadStarted {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Thread identifier.
        thread_id: String,
    },
    /// A multiagent thread completed.
    #[serde(rename = "session.thread_completed")]
    ThreadCompleted {
        /// Common event metadata.
        #[serde(flatten)]
        meta: SessionEventMeta,
        /// Thread identifier.
        thread_id: String,
    },

    // ---- sent (client-originated) — echoed back on the stream ----
    /// A user message.
    #[serde(rename = "user.message")]
    UserMessage {
        /// Message content blocks.
        #[serde(default)]
        content: Vec<ContentBlock>,
    },
    /// A user interrupt.
    #[serde(rename = "user.interrupt")]
    UserInterrupt {},
    /// A user tool-use confirmation.
    #[serde(rename = "user.tool_confirmation")]
    UserToolConfirmation {
        /// Tool-use identifier being confirmed.
        tool_use_id: String,
        /// Whether the tool use is approved.
        approve: bool,
    },
    /// A user-provided custom tool result.
    #[serde(rename = "user.custom_tool_result")]
    UserCustomToolResult {
        /// Tool-use identifier this result corresponds to.
        tool_use_id: String,
        /// Tool result content.
        content: serde_json::Value,
    },
    /// A user-defined outcome.
    #[serde(rename = "user.define_outcome")]
    UserDefineOutcome {
        /// Outcome definition payload.
        outcome: serde_json::Value,
    },
    /// A system message.
    #[serde(rename = "system.message")]
    SystemMessage {
        /// Message text.
        content: String,
    },

    /// Forward-compatible catch-all for event types added after this release.
    #[serde(other)]
    Unknown,
}

/// A lean client-originated event accepted by the `send` endpoint.
///
/// This is intentionally a subset of [`SessionEvent`] so callers cannot "send"
/// an agent-originated event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SendEvent {
    /// Send a user message.
    #[serde(rename = "user.message")]
    UserMessage {
        /// Message content blocks.
        content: Vec<ContentBlock>,
    },
    /// Interrupt the running agent.
    #[serde(rename = "user.interrupt")]
    UserInterrupt {},
    /// Confirm or reject a pending tool use.
    #[serde(rename = "user.tool_confirmation")]
    UserToolConfirmation {
        /// Tool-use identifier being confirmed.
        tool_use_id: String,
        /// Whether the tool use is approved.
        approve: bool,
    },
    /// Provide a custom tool result.
    #[serde(rename = "user.custom_tool_result")]
    UserCustomToolResult {
        /// Tool-use identifier this result corresponds to.
        tool_use_id: String,
        /// Tool result content.
        content: serde_json::Value,
    },
    /// Define a session outcome.
    #[serde(rename = "user.define_outcome")]
    UserDefineOutcome {
        /// Outcome definition payload.
        outcome: serde_json::Value,
    },
    /// Inject a system message mid-session.
    #[serde(rename = "system.message")]
    SystemMessage {
        /// Message text.
        content: String,
    },
}

impl SendEvent {
    /// Build a user text message event.
    pub fn user_text(text: impl Into<String>) -> Self {
        Self::UserMessage {
            content: vec![ContentBlock::Text {
                text: text.into(),
                citations: None,
                cache_control: None,
            }],
        }
    }

    /// Build an interrupt event.
    pub fn interrupt() -> Self {
        Self::UserInterrupt {}
    }

    /// Build a tool-confirmation event.
    pub fn confirm_tool(tool_use_id: impl Into<String>, approve: bool) -> Self {
        Self::UserToolConfirmation {
            tool_use_id: tool_use_id.into(),
            approve,
        }
    }

    /// Build a system-message event.
    pub fn system(text: impl Into<String>) -> Self {
        Self::SystemMessage {
            content: text.into(),
        }
    }
}

/// Response when listing session events (cursor-style pagination).
pub type SessionEventListResponse = PaginatedResponse<SessionEvent>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_message_event() {
        let json = r#"{
            "type": "agent.message",
            "id": "evt_1",
            "processed_at": "2026-04-01T00:00:00Z",
            "content": [{"type":"text","text":"hi"}]
        }"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentMessage { meta, content } => {
                assert_eq!(meta.id, "evt_1");
                assert_eq!(content.len(), 1);
            }
            other => panic!("expected AgentMessage, got {other:?}"),
        }
    }

    #[test]
    fn agent_tool_use_event() {
        let json = r#"{
            "type": "agent.tool_use",
            "id": "evt_2",
            "processed_at": "2026-04-01T00:00:00Z",
            "tool_use_id": "tu_1",
            "name": "search",
            "input": {"q": "x"}
        }"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentToolUse {
                tool_use_id, name, ..
            } => {
                assert_eq!(tool_use_id, "tu_1");
                assert_eq!(name, "search");
            }
            other => panic!("expected AgentToolUse, got {other:?}"),
        }
    }

    #[test]
    fn status_idle_event_with_stop_reason() {
        let json = r#"{
            "type": "session.status_idle",
            "id": "evt_3",
            "processed_at": "2026-04-01T00:00:00Z",
            "stop_reason": "awaiting_input"
        }"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::StatusIdle { stop_reason, .. } => {
                assert_eq!(stop_reason, Some(SessionStopReason::AwaitingInput));
            }
            other => panic!("expected StatusIdle, got {other:?}"),
        }
    }

    #[test]
    fn session_error_event() {
        let json = r#"{
            "type": "session.error",
            "id": "evt_4",
            "processed_at": "2026-04-01T00:00:00Z",
            "error": {"message": "boom"}
        }"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, SessionEvent::SessionError { .. }));
    }

    #[test]
    fn unknown_event_forward_compat() {
        let json = r#"{
            "type": "agent.brand_new_event_2027",
            "id": "evt_5",
            "processed_at": "2026-04-01T00:00:00Z"
        }"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event, SessionEvent::Unknown);
    }

    #[test]
    fn send_event_user_text_serialization() {
        let event = SendEvent::user_text("hello");
        let value = serde_json::to_value(&event).unwrap();
        assert_eq!(value["type"], "user.message");
        assert_eq!(value["content"][0]["text"], "hello");
    }
}
