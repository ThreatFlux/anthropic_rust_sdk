//! Bounded message streaming and strict, presence-aware message accumulation.

use crate::{
    error::{AnthropicError, Result},
    models::common::ContentBlock,
    models::message::{MessageResponse, StreamEvent},
    streaming::event_parser::DEFAULT_MAX_EVENT_BYTES,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

mod accumulator;
mod content_delta;
mod framing;

use accumulator::MessageAccumulator;
use framing::pump_lines;

/// Bounds for raw SSE frames and collected content.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct StreamLimits {
    /// Maximum bytes in a single SSE event or line.
    pub max_event_bytes: usize,
    /// Maximum bytes in one collected block's streamed tool-input JSON.
    pub max_tool_input_bytes: usize,
    /// Maximum content blocks in a collected message.
    pub max_content_blocks: usize,
    /// Maximum queued events before applying HTTP backpressure.
    pub event_buffer_capacity: usize,
}

impl Default for StreamLimits {
    fn default() -> Self {
        Self {
            max_event_bytes: DEFAULT_MAX_EVENT_BYTES,
            max_tool_input_bytes: DEFAULT_MAX_EVENT_BYTES,
            max_content_blocks: 4096,
            event_buffer_capacity: 32,
        }
    }
}
impl StreamLimits {
    /// Validate nonzero limits before starting HTTP work.
    pub fn validate(&self) -> Result<()> {
        if self.max_event_bytes == 0
            || self.max_tool_input_bytes == 0
            || self.max_content_blocks == 0
            || self.event_buffer_capacity == 0
            || self.event_buffer_capacity > tokio::sync::Semaphore::MAX_PERMITS
        {
            return Err(AnthropicError::invalid_input(
                "Stream limits must be nonzero",
            ));
        }
        Ok(())
    }
}

/// Stream of message events. Dropping it cancels the HTTP producer task.
pub struct MessageStream {
    receiver: mpsc::Receiver<Result<StreamEvent>>,
    handle: tokio::task::JoinHandle<()>,
    limits: StreamLimits,
}

impl MessageStream {
    /// Create a stream with bounded defaults from a successful HTTP response.
    pub async fn new(response: reqwest::Response) -> Result<Self> {
        Self::new_with_limits(response, StreamLimits::default()).await
    }

    /// Create a stream with explicit frame, tool input, block, and queue bounds.
    pub async fn new_with_limits(
        response: reqwest::Response,
        limits: StreamLimits,
    ) -> Result<Self> {
        limits.validate()?;
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AnthropicError::api_error(status.as_u16(), error_text, None));
        }
        Ok(Self::from_bytes(response.bytes_stream(), limits))
    }

    pub(super) fn from_bytes<S, B>(bytes: S, limits: StreamLimits) -> Self
    where
        S: Stream<Item = reqwest::Result<B>> + Unpin + Send + 'static,
        B: AsRef<[u8]> + Send + 'static,
    {
        let (sender, receiver) = mpsc::channel(limits.event_buffer_capacity);
        let handle = tokio::spawn(pump_lines(bytes, sender, limits.max_event_bytes));
        Self {
            receiver,
            handle,
            limits,
        }
    }

    /// Collect through a valid terminal `message_stop`, then cancel HTTP work.
    /// EOF before that event, invalid lifecycles and unknown content deltas with
    /// no safe merge rule fail explicitly. Already buffered protocol events are
    /// validated; events arriving after cancellation cannot be observed.
    pub async fn collect_message(mut self) -> Result<MessageResponse> {
        let mut accumulator = MessageAccumulator::new(self.limits);
        while let Some(event) = self.next().await {
            let event = event?;
            let terminal = matches!(event, StreamEvent::MessageStop);
            accumulator.apply(event)?;
            if terminal {
                self.handle.abort();
                self.receiver.close();
                while let Ok(event) = self.receiver.try_recv() {
                    accumulator.apply(event?)?;
                }
                return accumulator.finish();
            }
        }
        accumulator.finish()
    }

    /// Collect text from a fully validated message lifecycle.
    pub async fn collect_text(self) -> Result<String> {
        let message = self.collect_message().await?;
        Ok(message
            .content
            .iter()
            .filter_map(ContentBlock::as_text)
            .collect::<Vec<_>>()
            .concat())
    }

    /// Cancel HTTP work immediately and discard buffered events.
    pub fn close(&mut self) {
        self.handle.abort();
        self.receiver.close();
        while self.receiver.try_recv().is_ok() {}
    }

    /// Whether the producer closed and all queued events have been consumed.
    pub fn is_done(&self) -> bool {
        self.receiver.is_closed() && self.receiver.is_empty()
    }
}

impl Drop for MessageStream {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
impl Stream for MessageStream {
    type Item = Result<StreamEvent>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx)
    }
}
impl futures::stream::FusedStream for MessageStream {
    fn is_terminated(&self) -> bool {
        self.is_done()
    }
}

#[cfg(test)]
mod tests;
