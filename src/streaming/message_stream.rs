//! Streaming message responses

use crate::{
    error::{AnthropicError, Result},
    models::common::{CacheCreationUsage, ContentBlock, ServerToolUsage, ToolResultContent},
    models::message::{MessageResponse, StreamEvent},
    streaming::event_parser::EventParser,
};
use futures::{Stream, StreamExt};
use std::collections::HashMap;
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
        let mut input_json_buffers: HashMap<usize, String> = HashMap::new();

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
                    if let Some(text) = delta.text {
                        if let Some(Some(ContentBlock::Text {
                            text: ref mut block_text,
                            ..
                        })) = content_blocks.get_mut(index)
                        {
                            block_text.push_str(&text);
                        }
                    }

                    if let Some(thinking_delta) = delta.thinking {
                        if let Some(Some(ContentBlock::Thinking {
                            thinking: ref mut block_thinking,
                            ..
                        })) = content_blocks.get_mut(index)
                        {
                            block_thinking.push_str(&thinking_delta);
                        }
                    }

                    if let Some(signature_delta) = delta.signature {
                        if let Some(Some(ContentBlock::Thinking { signature, .. })) =
                            content_blocks.get_mut(index)
                        {
                            signature
                                .get_or_insert_with(String::new)
                                .push_str(&signature_delta);
                        }
                    }

                    if let Some(partial_json) = delta.partial_json {
                        input_json_buffers
                            .entry(index)
                            .and_modify(|buffer| buffer.push_str(&partial_json))
                            .or_insert(partial_json);
                    }

                    if let Some(citation_delta) = delta.citation {
                        if let Some(Some(ContentBlock::Text { citations, .. })) =
                            content_blocks.get_mut(index)
                        {
                            citations.get_or_insert_with(Vec::new).push(citation_delta);
                        }
                    }
                }
                StreamEvent::MessageDelta { delta, usage } => {
                    if let Some(ref mut message) = message_response {
                        // Streaming usage payloads can be partial; keep the max observed values.
                        message.usage.input_tokens =
                            message.usage.input_tokens.max(usage.input_tokens);
                        message.usage.output_tokens =
                            message.usage.output_tokens.max(usage.output_tokens);
                        message.usage.cache_creation_input_tokens = message
                            .usage
                            .cache_creation_input_tokens
                            .max(usage.cache_creation_input_tokens);
                        message.usage.cache_read_input_tokens = message
                            .usage
                            .cache_read_input_tokens
                            .max(usage.cache_read_input_tokens);

                        if let Some(incoming_cache_creation) = usage.cache_creation {
                            let cache_creation = message
                                .usage
                                .cache_creation
                                .get_or_insert_with(CacheCreationUsage::default);
                            cache_creation.ephemeral_5m_input_tokens = cache_creation
                                .ephemeral_5m_input_tokens
                                .max(incoming_cache_creation.ephemeral_5m_input_tokens);
                            cache_creation.ephemeral_1h_input_tokens = cache_creation
                                .ephemeral_1h_input_tokens
                                .max(incoming_cache_creation.ephemeral_1h_input_tokens);
                        }

                        if let Some(incoming_server_tool_use) = usage.server_tool_use {
                            let server_tool_use = message
                                .usage
                                .server_tool_use
                                .get_or_insert_with(ServerToolUsage::default);
                            server_tool_use.web_search_requests = server_tool_use
                                .web_search_requests
                                .max(incoming_server_tool_use.web_search_requests);
                        }

                        if usage.inference_geo.is_some() {
                            message.usage.inference_geo = usage.inference_geo;
                        }
                        if usage.service_tier.is_some() {
                            message.usage.service_tier = usage.service_tier;
                        }

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
                StreamEvent::ContentBlockStop { index } => {
                    if let Some(partial_json) = input_json_buffers.remove(&index) {
                        let parsed = serde_json::from_str::<serde_json::Value>(&partial_json)
                            .unwrap_or(serde_json::Value::String(partial_json));

                        if let Some(Some(ContentBlock::ToolUse { input, .. })) =
                            content_blocks.get_mut(index)
                        {
                            *input = parsed.clone();
                        } else if let Some(Some(ContentBlock::ServerToolUse { input, .. })) =
                            content_blocks.get_mut(index)
                        {
                            *input = Some(parsed.clone());
                        } else if let Some(Some(ContentBlock::ToolResult { content, .. })) =
                            content_blocks.get_mut(index)
                        {
                            *content = Some(ToolResultContent::Json(parsed));
                        }
                    }
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
