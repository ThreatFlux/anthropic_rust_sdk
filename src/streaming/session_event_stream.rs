//! Streaming session events for Managed Agents (beta: managed-agents-2026-04-01).
//!
//! Unlike [`MessageStream`](crate::streaming::MessageStream), session events are
//! delivered as complete JSON objects in each SSE `data:` frame — there is no
//! multi-line content-block reassembly. This stream is therefore self-contained:
//! it accumulates raw bytes, splits SSE frames on blank lines, and deserializes
//! each frame's concatenated `data:` payload straight into a [`SessionEvent`].

use crate::{
    error::{AnthropicError, Result},
    models::managed_agents::session_event::SessionEvent,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// Stream of [`SessionEvent`]s from a Managed Agents session.
pub struct SessionEventStream {
    receiver: mpsc::Receiver<Result<SessionEvent>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl SessionEventStream {
    /// Create a new session-event stream from an HTTP response.
    pub async fn new(response: reqwest::Response) -> Result<Self> {
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AnthropicError::api_error(status.as_u16(), error_text, None));
        }

        let (sender, receiver) = mpsc::channel(100);
        let mut bytes_stream = response.bytes_stream();

        let handle = tokio::spawn(async move {
            // Raw text buffer; SSE frames are separated by a blank line.
            let mut buffer = String::new();

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.push_str(&String::from_utf8_lossy(&chunk));

                        // Process every complete frame currently in the buffer. A
                        // frame boundary is a blank line: "\n\n" (or "\r\n\r\n").
                        while let Some(boundary) = find_frame_boundary(&buffer) {
                            let frame: String = buffer.drain(..boundary.end).collect();
                            let frame = &frame[..boundary.frame_len];

                            match parse_frame(frame) {
                                Ok(Some(event)) => {
                                    if sender.send(Ok(event)).await.is_err() {
                                        return; // Receiver dropped, exit cleanly.
                                    }
                                }
                                Ok(None) => {
                                    // Comment-only or `[DONE]` frame — skip.
                                }
                                Err(e) => {
                                    let _ = sender.send(Err(e)).await;
                                    return; // Exit on parse error.
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let error = AnthropicError::stream(format!("Stream chunk error: {}", e))
                            .with_context("Session event stream processing");
                        let _ = sender.send(Err(error)).await;
                        return;
                    }
                }
            }

            // Flush any trailing frame that was not terminated by a blank line.
            let trailing = buffer.trim();
            if !trailing.is_empty() {
                match parse_frame(trailing) {
                    Ok(Some(event)) => {
                        let _ = sender.send(Ok(event)).await;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        let _ = sender.send(Err(e)).await;
                    }
                }
            }
        });

        Ok(Self {
            receiver,
            _handle: handle,
        })
    }

    /// Check if the stream is done.
    pub fn is_done(&self) -> bool {
        self.receiver.is_closed()
    }
}

/// Location of the first complete SSE frame in `buffer`.
struct FrameBoundary {
    /// Byte length of the frame content (excluding the blank-line separator).
    frame_len: usize,
    /// Byte index just past the blank-line separator (the drain point).
    end: usize,
}

/// Find the first blank-line frame separator (`\n\n` or `\r\n\r\n`).
fn find_frame_boundary(buffer: &str) -> Option<FrameBoundary> {
    if let Some(pos) = buffer.find("\r\n\r\n") {
        return Some(FrameBoundary {
            frame_len: pos,
            end: pos + 4,
        });
    }
    if let Some(pos) = buffer.find("\n\n") {
        return Some(FrameBoundary {
            frame_len: pos,
            end: pos + 2,
        });
    }
    None
}

/// Parse a single SSE frame into a [`SessionEvent`].
///
/// Returns `Ok(None)` for comment-only frames or a `[DONE]` sentinel.
fn parse_frame(frame: &str) -> Result<Option<SessionEvent>> {
    let mut data = String::new();

    for line in frame.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue; // Blank line or SSE comment.
        }
        if let Some(rest) = line.strip_prefix("data:") {
            // SSE allows an optional leading space after the colon.
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        }
        // `event:` / `id:` / `retry:` lines are ignored; the JSON payload is
        // internally tagged on `type`, so the `event:` name is redundant.
    }

    let data = data.trim();
    if data.is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    let event = serde_json::from_str::<SessionEvent>(data).map_err(|e| {
        AnthropicError::stream(format!("Failed to parse session event: {}", e))
            .with_context("Session event deserialization")
    })?;
    Ok(Some(event))
}

impl Stream for SessionEventStream {
    type Item = Result<SessionEvent>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}

impl futures::stream::FusedStream for SessionEventStream {
    fn is_terminated(&self) -> bool {
        self.receiver.is_closed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_frame_agent_message() {
        let frame = "event: agent.message\ndata: {\"type\":\"agent.message\",\"id\":\"e1\",\"processed_at\":\"2026-04-01T00:00:00Z\",\"content\":[]}";
        let event = parse_frame(frame).unwrap().unwrap();
        assert!(matches!(event, SessionEvent::AgentMessage { .. }));
    }

    #[test]
    fn parse_frame_done_sentinel() {
        assert!(parse_frame("data: [DONE]").unwrap().is_none());
    }

    #[test]
    fn parse_frame_comment_only() {
        assert!(parse_frame(": keep-alive").unwrap().is_none());
    }

    #[test]
    fn parse_frame_malformed_errors() {
        let frame = "data: {not json}";
        assert!(parse_frame(frame).is_err());
    }

    #[test]
    fn find_boundary_lf() {
        let b = find_frame_boundary("data: x\n\ndata: y").unwrap();
        assert_eq!(b.frame_len, 7);
        assert_eq!(b.end, 9);
    }
}
