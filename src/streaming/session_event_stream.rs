//! Bounded, UTF-8-safe SSE delivery for Managed Agents session events.

use crate::{
    error::{AnthropicError, Result},
    models::{managed_agents::session_event::SessionEvent, message::StreamEvent},
    streaming::message_stream::{MessageStream, StreamLimits},
};
use futures::Stream;
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Stream of session events with shared bounded SSE framing and cancellation.
pub struct SessionEventStream {
    inner: MessageStream,
    finished: bool,
}

impl SessionEventStream {
    /// Create a stream with the same frame and queue defaults as message streams.
    pub async fn new(response: reqwest::Response) -> Result<Self> {
        Self::new_with_limits(response, StreamLimits::default()).await
    }

    /// Create a session stream with explicit frame and queue bounds.
    pub async fn new_with_limits(
        response: reqwest::Response,
        limits: StreamLimits,
    ) -> Result<Self> {
        Ok(Self {
            inner: MessageStream::new_with_limits(response, limits).await?,
            finished: false,
        })
    }

    /// Cancel HTTP work and discard buffered events.
    pub fn close(&mut self) {
        self.finished = true;
        self.inner.close();
    }

    /// Whether the producer is closed and buffered events have been consumed.
    pub fn is_done(&self) -> bool {
        self.finished || self.inner.is_done()
    }
}

impl Stream for SessionEventStream {
    type Item = Result<SessionEvent>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.finished {
            return Poll::Ready(None);
        }
        loop {
            let next = std::task::ready!(Pin::new(&mut self.inner).poll_next(cx));
            let data = match next {
                None => {
                    self.close();
                    return Poll::Ready(None);
                }
                Some(Err(error)) => {
                    self.close();
                    return Poll::Ready(Some(Err(error)));
                }
                Some(Ok(StreamEvent::Ping)) => continue,
                Some(Ok(StreamEvent::Unknown { data, .. })) => data,
                Some(Ok(StreamEvent::Error { .. })) => {
                    self.close();
                    return Poll::Ready(Some(Err(AnthropicError::stream(
                        "API error in session event stream",
                    ))));
                }
                Some(Ok(event)) => match serde_json::to_string(&event) {
                    Ok(data) => data,
                    Err(_) => {
                        self.close();
                        return Poll::Ready(Some(Err(AnthropicError::stream(
                            "Invalid session event schema",
                        ))));
                    }
                },
            };
            if data.trim() == "[DONE]" {
                self.close();
                return Poll::Ready(None);
            }
            if data.trim().is_empty() {
                continue;
            }
            let event = serde_json::from_str(&data)
                .map_err(|_| AnthropicError::stream("Invalid session event JSON/schema"));
            if event.is_err() {
                self.close();
            }
            return Poll::Ready(Some(event));
        }
    }
}
impl futures::stream::FusedStream for SessionEventStream {
    fn is_terminated(&self) -> bool {
        self.is_done()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::{stream, StreamExt};
    use serde_json::json;

    #[tokio::test]
    async fn split_utf8_and_mixed_line_endings_preserve_session_payloads() {
        let payload = json!({"type":"agent.message","id":"evt_1","processed_at":"2026-10-03T00:00:00Z","content":[{"type":"future_content","opaque":{"text":"é😀"}}]});
        let body=format!("event: agent.message\r\ndata: {payload}\r\n\r\nevent: agent.thinking\ndata: {{\"type\":\"agent.thinking\",\"id\":\"evt_2\",\"processed_at\":\"2026-10-03T00:00:00Z\",\"thinking\":\"é😀\"}}\n\ndata: [DONE]\n\n");
        let chunks: Vec<reqwest::Result<Vec<u8>>> =
            body.as_bytes().iter().map(|byte| Ok(vec![*byte])).collect();
        let mut session = SessionEventStream {
            inner: MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default()),
            finished: false,
        };
        let first = session.next().await.unwrap().unwrap();
        assert_eq!(
            serde_json::to_value(first).unwrap()["content"][0],
            payload["content"][0]
        );
        let second = session.next().await.unwrap().unwrap();
        assert_eq!(serde_json::to_value(second).unwrap()["thinking"], "é😀");
        assert!(session.next().await.is_none());
        assert!(session.is_done());
    }

    #[tokio::test]
    async fn malformed_trailing_session_frame_fails_once() {
        let chunks: Vec<reqwest::Result<Vec<u8>>> =
            vec![Ok(b"event: agent.message\ndata: {invalid-json".to_vec())];
        let mut session = SessionEventStream {
            inner: MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default()),
            finished: false,
        };
        assert!(session.next().await.unwrap().is_err());
        assert!(session.next().await.is_none());
    }
}
