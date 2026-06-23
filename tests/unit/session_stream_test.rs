//! Unit tests for the Managed Agents session event stream
//! (beta: managed-agents-2026-04-01).
//!
//! These exercise two layers:
//!  1. Direct `serde_json::from_str::<SessionEvent>(...)` for each documented
//!     event `type` (the internally-tagged enum that the stream decodes into).
//!  2. The end-to-end `SessionEventStream`: a `text/event-stream` body of
//!     concatenated `event:`/`data:` frames (mirroring `mock_message_stream` in
//!     `tests/common/mod.rs`) is served from a `wiremock` mock and driven to
//!     completion via `client.sessions().events(id).stream(...)`.

use futures::StreamExt;
use threatflux::{
    models::managed_agents::{SessionEvent, SessionStopReason},
    Client, Config,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Build a client pointed at the mock server (mirrors the integration helper).
fn mock_client(server: &MockServer) -> Client {
    let config = Config::new("sk-ant-test-key")
        .unwrap()
        .with_base_url(server.uri().parse().unwrap());
    Client::new(config)
}

/// Concatenate SSE frames into a single `text/event-stream` body, mirroring the
/// `event:`/`data:`/blank-line framing used by `mock_message_stream`.
fn sse_body(frames: &[&str]) -> String {
    let mut body = String::new();
    for frame in frames {
        body.push_str(frame);
        body.push_str("\n\n");
    }
    body
}

const FRAME_AGENT_MESSAGE: &str = "event: agent.message\ndata: {\"type\":\"agent.message\",\"id\":\"evt_1\",\"processed_at\":\"2026-04-01T00:00:00Z\",\"content\":[{\"type\":\"text\",\"text\":\"hi\"}]}";
const FRAME_TOOL_USE: &str = "event: agent.tool_use\ndata: {\"type\":\"agent.tool_use\",\"id\":\"evt_2\",\"processed_at\":\"2026-04-01T00:00:01Z\",\"tool_use_id\":\"tu_1\",\"name\":\"search\",\"input\":{\"q\":\"x\"}}";
const FRAME_STATUS_IDLE: &str = "event: session.status_idle\ndata: {\"type\":\"session.status_idle\",\"id\":\"evt_3\",\"processed_at\":\"2026-04-01T00:00:02Z\",\"stop_reason\":\"awaiting_input\"}";
const FRAME_SESSION_ERROR: &str = "event: session.error\ndata: {\"type\":\"session.error\",\"id\":\"evt_4\",\"processed_at\":\"2026-04-01T00:00:03Z\",\"error\":{\"message\":\"boom\"}}";
const FRAME_UNKNOWN: &str = "event: agent.future_event\ndata: {\"type\":\"agent.future_event_2027\",\"id\":\"evt_5\",\"processed_at\":\"2026-04-01T00:00:04Z\",\"surprise\":true}";

#[cfg(test)]
mod direct_event_parse_tests {
    use super::*;

    #[test]
    fn parse_agent_message() {
        let json = r#"{"type":"agent.message","id":"e","processed_at":"2026-04-01T00:00:00Z","content":[{"type":"text","text":"hi"}]}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentMessage { meta, content } => {
                assert_eq!(meta.id, "e");
                assert_eq!(content.len(), 1);
            }
            other => panic!("expected AgentMessage, got {other:?}"),
        }
    }

    #[test]
    fn parse_agent_thinking() {
        let json = r#"{"type":"agent.thinking","id":"e","processed_at":"2026-04-01T00:00:00Z","thinking":"hmm"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentThinking { thinking, .. } => assert_eq!(thinking, "hmm"),
            other => panic!("expected AgentThinking, got {other:?}"),
        }
    }

    #[test]
    fn parse_agent_tool_use() {
        let json = r#"{"type":"agent.tool_use","id":"e","processed_at":"2026-04-01T00:00:00Z","tool_use_id":"tu","name":"search","input":{"q":"x"}}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentToolUse {
                tool_use_id,
                name,
                input,
                ..
            } => {
                assert_eq!(tool_use_id, "tu");
                assert_eq!(name, "search");
                assert_eq!(input["q"], "x");
            }
            other => panic!("expected AgentToolUse, got {other:?}"),
        }
    }

    #[test]
    fn parse_agent_tool_result() {
        let json = r#"{"type":"agent.tool_result","id":"e","processed_at":"2026-04-01T00:00:00Z","tool_use_id":"tu","content":{"ok":true}}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentToolResult {
                tool_use_id,
                content,
                ..
            } => {
                assert_eq!(tool_use_id, "tu");
                assert_eq!(content["ok"], true);
            }
            other => panic!("expected AgentToolResult, got {other:?}"),
        }
    }

    #[test]
    fn parse_agent_mcp_tool_use() {
        let json = r#"{"type":"agent.mcp_tool_use","id":"e","processed_at":"2026-04-01T00:00:00Z","server":"forge"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::AgentMcpToolUse { extra, .. } => {
                assert_eq!(extra["server"], "forge");
            }
            other => panic!("expected AgentMcpToolUse, got {other:?}"),
        }
    }

    #[test]
    fn parse_agent_custom_tool_use() {
        let json =
            r#"{"type":"agent.custom_tool_use","id":"e","processed_at":"2026-04-01T00:00:00Z"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, SessionEvent::AgentCustomToolUse { .. }));
    }

    #[test]
    fn parse_status_running() {
        let json =
            r#"{"type":"session.status_running","id":"e","processed_at":"2026-04-01T00:00:00Z"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, SessionEvent::StatusRunning { .. }));
    }

    #[test]
    fn parse_status_idle_with_stop_reason() {
        let json = r#"{"type":"session.status_idle","id":"e","processed_at":"2026-04-01T00:00:00Z","stop_reason":"end_turn"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::StatusIdle { stop_reason, .. } => {
                assert_eq!(stop_reason, Some(SessionStopReason::EndTurn));
            }
            other => panic!("expected StatusIdle, got {other:?}"),
        }
    }

    #[test]
    fn parse_status_idle_without_stop_reason() {
        let json =
            r#"{"type":"session.status_idle","id":"e","processed_at":"2026-04-01T00:00:00Z"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::StatusIdle { stop_reason, .. } => assert!(stop_reason.is_none()),
            other => panic!("expected StatusIdle, got {other:?}"),
        }
    }

    #[test]
    fn parse_status_rescheduled() {
        let json = r#"{"type":"session.status_rescheduled","id":"e","processed_at":"2026-04-01T00:00:00Z"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, SessionEvent::StatusRescheduled { .. }));
    }

    #[test]
    fn parse_status_terminated() {
        let json = r#"{"type":"session.status_terminated","id":"e","processed_at":"2026-04-01T00:00:00Z"}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert!(matches!(event, SessionEvent::StatusTerminated { .. }));
    }

    #[test]
    fn parse_session_error() {
        let json = r#"{"type":"session.error","id":"e","processed_at":"2026-04-01T00:00:00Z","error":{"message":"boom"}}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        match event {
            SessionEvent::SessionError { error, .. } => assert_eq!(error["message"], "boom"),
            other => panic!("expected SessionError, got {other:?}"),
        }
    }

    #[test]
    fn parse_span_events() {
        for ty in [
            "span.model_request_start",
            "span.model_request_end",
            "span.outcome_evaluation_start",
            "span.outcome_evaluation_end",
        ] {
            let json = format!(
                r#"{{"type":"{ty}","id":"e","processed_at":"2026-04-01T00:00:00Z","extra_key":1}}"#
            );
            let event: SessionEvent = serde_json::from_str(&json).unwrap();
            let ok = matches!(
                event,
                SessionEvent::SpanModelRequestStart { .. }
                    | SessionEvent::SpanModelRequestEnd { .. }
                    | SessionEvent::SpanOutcomeEvalStart { .. }
                    | SessionEvent::SpanOutcomeEvalEnd { .. }
            );
            assert!(ok, "type {ty} did not map to a span variant");
        }
    }

    #[test]
    fn parse_thread_events() {
        let started = r#"{"type":"session.thread_started","id":"e","processed_at":"2026-04-01T00:00:00Z","thread_id":"th_1"}"#;
        match serde_json::from_str::<SessionEvent>(started).unwrap() {
            SessionEvent::ThreadStarted { thread_id, .. } => assert_eq!(thread_id, "th_1"),
            other => panic!("expected ThreadStarted, got {other:?}"),
        }
        let completed = r#"{"type":"session.thread_completed","id":"e","processed_at":"2026-04-01T00:00:00Z","thread_id":"th_1"}"#;
        match serde_json::from_str::<SessionEvent>(completed).unwrap() {
            SessionEvent::ThreadCompleted { thread_id, .. } => assert_eq!(thread_id, "th_1"),
            other => panic!("expected ThreadCompleted, got {other:?}"),
        }
    }

    #[test]
    fn parse_unknown_event_is_forward_compatible() {
        let json = r#"{"type":"agent.brand_new_event_2099","id":"e","processed_at":"2026-04-01T00:00:00Z","x":1}"#;
        let event: SessionEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event, SessionEvent::Unknown);
    }

    #[test]
    fn parse_malformed_json_errors() {
        assert!(serde_json::from_str::<SessionEvent>("{not json}").is_err());
    }
}

#[cfg(test)]
mod stream_e2e_tests {
    use super::*;

    /// Mount a streaming mock at the session-events stream path.
    async fn mount_stream(server: &MockServer, body: String) {
        Mock::given(method("GET"))
            .and(path("/v1/sessions/sess_1/events/stream"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn stream_decodes_full_event_sequence() {
        let server = MockServer::start().await;
        let body = sse_body(&[
            FRAME_AGENT_MESSAGE,
            FRAME_TOOL_USE,
            FRAME_STATUS_IDLE,
            FRAME_SESSION_ERROR,
            FRAME_UNKNOWN,
        ]);
        mount_stream(&server, body).await;

        let client = mock_client(&server);
        let mut stream = client
            .sessions()
            .events("sess_1")
            .stream(None)
            .await
            .expect("stream should open");

        let mut events = Vec::new();
        while let Some(item) = stream.next().await {
            events.push(item.expect("each frame should decode"));
        }

        assert_eq!(events.len(), 5);
        assert!(matches!(events[0], SessionEvent::AgentMessage { .. }));
        assert!(matches!(events[1], SessionEvent::AgentToolUse { .. }));
        match &events[2] {
            SessionEvent::StatusIdle { stop_reason, .. } => {
                assert_eq!(*stop_reason, Some(SessionStopReason::AwaitingInput));
            }
            other => panic!("expected StatusIdle, got {other:?}"),
        }
        assert!(matches!(events[3], SessionEvent::SessionError { .. }));
        // The unrecognized event type is preserved as `Unknown` rather than
        // hard-failing the stream.
        assert_eq!(events[4], SessionEvent::Unknown);
    }

    #[tokio::test]
    async fn stream_ignores_comment_and_done_frames() {
        let server = MockServer::start().await;
        let mut body = String::new();
        body.push_str(": keep-alive\n\n");
        body.push_str(FRAME_AGENT_MESSAGE);
        body.push_str("\n\n");
        body.push_str("data: [DONE]\n\n");
        mount_stream(&server, body).await;

        let client = mock_client(&server);
        let mut stream = client
            .sessions()
            .events("sess_1")
            .stream(None)
            .await
            .unwrap();

        let mut events = Vec::new();
        while let Some(item) = stream.next().await {
            events.push(item.unwrap());
        }
        // Only the single real frame survives; comment + [DONE] are skipped.
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], SessionEvent::AgentMessage { .. }));
    }

    #[tokio::test]
    async fn stream_yields_err_on_malformed_frame_then_ends() {
        let server = MockServer::start().await;
        let mut body = String::new();
        body.push_str(FRAME_AGENT_MESSAGE);
        body.push_str("\n\n");
        // A frame whose data payload is not valid JSON.
        body.push_str("event: agent.message\ndata: {not valid json}\n\n");
        body.push_str(FRAME_STATUS_IDLE);
        body.push_str("\n\n");
        mount_stream(&server, body).await;

        let client = mock_client(&server);
        let mut stream = client
            .sessions()
            .events("sess_1")
            .stream(None)
            .await
            .unwrap();

        // First frame decodes fine.
        let first = stream.next().await.expect("first item");
        assert!(first.is_ok());
        assert!(matches!(first.unwrap(), SessionEvent::AgentMessage { .. }));

        // Second frame is malformed -> the stream yields an Err.
        let second = stream.next().await.expect("second item");
        assert!(second.is_err());

        // After the parse error the stream terminates cleanly (no further items,
        // even though a well-formed frame followed the bad one).
        assert!(stream.next().await.is_none());
    }
}
