//! Streaming message responses

use crate::{
    error::{AnthropicError, Result},
    models::message::{MessageResponse, StreamEvent},
    streaming::event_parser::EventParser,
};
use futures::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

/// Stream of message events from the Anthropic API
pub struct MessageStream {
    receiver: mpsc::Receiver<Result<StreamEvent>>,
    _handle: tokio::task::JoinHandle<()>,
}

impl MessageStream {
    /// Create a new message stream from an HTTP response
    pub async fn new(response: reqwest::Response) -> Result<Self> {
        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(AnthropicError::api_error(status.as_u16(), error_text, None));
        }

        let (sender, receiver) = mpsc::channel(100);
        let mut bytes_stream = response.bytes_stream();
        let mut parser = EventParser::new();

        let handle = tokio::spawn(async move {
            let mut buffer = Vec::with_capacity(8192); // Pre-allocate buffer for better performance

            while let Some(chunk_result) = bytes_stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        buffer.extend_from_slice(&chunk);

                        // Process complete lines
                        while let Some(newline_pos) = buffer.iter().position(|&b| b == b'\n') {
                            let line = buffer.drain(..=newline_pos).collect::<Vec<_>>();
                            // Remove newline and handle both \r\n and \n line endings
                            let line_len = if line.len() >= 2 && line[line.len() - 2] == b'\r' {
                                line.len() - 2
                            } else {
                                line.len() - 1
                            };
                            let line_str = String::from_utf8_lossy(&line[..line_len]);

                            match parser.parse_line(&line_str) {
                                Ok(Some(event)) => {
                                    if sender.send(Ok(event)).await.is_err() {
                                        return; // Receiver dropped, exit cleanly
                                    }
                                }
                                Ok(None) => {
                                    // Continue processing (comment, empty line, or partial event)
                                }
                                Err(e) => {
                                    let _ = sender.send(Err(e)).await;
                                    return; // Exit on parse error
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let error = AnthropicError::stream(format!("Stream chunk error: {}", e))
                            .with_context("HTTP stream processing");
                        let _ = sender.send(Err(error)).await;
                        return; // Exit on stream error
                    }
                }
            }
        });

        Ok(Self {
            receiver,
            _handle: handle,
        })
    }

    /// Collect all events into a complete message response
    pub async fn collect_message(mut self) -> Result<MessageResponse> {
        let mut message_response = None;
        let mut content_blocks = Vec::new();

        while let Some(event_result) = self.next().await {
            let event = event_result?;

            match event {
                StreamEvent::MessageStart { message } => {
                    message_response = Some(message);
                }
                StreamEvent::ContentBlockStart {
                    index,
                    content_block,
                } => {
                    // Ensure we have enough space
                    while content_blocks.len() <= index {
                        content_blocks.push(None);
                    }
                    content_blocks[index] = Some(content_block);
                }
                StreamEvent::ContentBlockDelta { index, delta } => {
                    if let (
                        Some(Some(crate::models::common::ContentBlock::Text {
                            text: ref mut block_text,
                        })),
                        Some(text),
                    ) = (content_blocks.get_mut(index), &delta.text)
                    {
                        block_text.push_str(text);
                    }
                }
                StreamEvent::MessageDelta { delta, usage } => {
                    if let Some(ref mut message) = message_response {
                        message.usage = usage;
                        if let Some(stop_reason) = delta.stop_reason {
                            message.stop_reason = Some(stop_reason);
                        }
                        if let Some(stop_sequence) = delta.stop_sequence {
                            message.stop_sequence = Some(stop_sequence);
                        }
                    }
                }
                StreamEvent::MessageStop => {
                    break;
                }
                StreamEvent::ContentBlockStop { .. } => {
                    // Content block finished
                }
                StreamEvent::Ping => {
                    // Keep-alive ping, ignore
                }
                StreamEvent::Error { error } => {
                    return Err(AnthropicError::stream(format!("Stream error: {:?}", error))
                        .with_context("Message streaming"));
                }
            }
        }

        let mut message = message_response.ok_or_else(|| {
            AnthropicError::stream("No message_start event received")
                .with_context("Stream message collection")
        })?;

        // Update content with streamed content
        message.content = content_blocks.into_iter().flatten().collect();

        Ok(message)
    }

    /// Collect only text content from the stream
    pub async fn collect_text(mut self) -> Result<String> {
        let mut text = String::new();

        while let Some(event_result) = self.next().await {
            let event = event_result?;

            match event {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Some(delta_text) = delta.text {
                        text.push_str(&delta_text);
                    }
                }
                StreamEvent::MessageStop => {
                    break;
                }
                StreamEvent::Error { error } => {
                    return Err(AnthropicError::stream(format!("Stream error: {:?}", error))
                        .with_context("Message streaming"));
                }
                _ => {
                    // Ignore other event types
                }
            }
        }

        Ok(text)
    }

    /// Check if the stream is done
    pub fn is_done(&self) -> bool {
        self.receiver.is_closed()
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
        self.receiver.is_closed()
    }
}
