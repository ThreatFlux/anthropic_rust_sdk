//! Bounded byte framing with same-chunk terminal-event validation.

use crate::{
    error::{AnthropicError, Result},
    models::StreamEvent,
    streaming::event_parser::EventParser,
};
use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

/// Preserve UTF-8 across chunks and recognize LF, CRLF and CR line endings.
pub(super) async fn pump_lines<S, B>(
    mut bytes: S,
    sender: mpsc::Sender<Result<StreamEvent>>,
    max_bytes: usize,
) where
    S: Stream<Item = reqwest::Result<B>> + Unpin + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
{
    let mut framer = match LineFramer::new(max_bytes) {
        Ok(framer) => framer,
        Err(error) => {
            send_error(&sender, error).await;
            return;
        }
    };
    loop {
        let chunk = tokio::select! { _ = sender.closed() => return, chunk = bytes.next() => chunk };
        let Some(chunk) = chunk else { break };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => {
                send_error(
                    &sender,
                    AnthropicError::stream("HTTP stream transport failed"),
                )
                .await;
                return;
            }
        };
        if !framer.process_chunk(chunk.as_ref(), &sender).await {
            return;
        }
    }
    framer.finish(&sender).await;
}

struct LineFramer {
    parser: EventParser,
    max_bytes: usize,
    line: Vec<u8>,
    skip_lf: bool,
    first_line: bool,
    terminal_seen: bool,
    terminal_batch: Vec<StreamEvent>,
}

impl LineFramer {
    fn new(max_bytes: usize) -> Result<Self> {
        Ok(Self {
            parser: EventParser::with_max_event_bytes(max_bytes)?,
            max_bytes,
            line: Vec::new(),
            skip_lf: false,
            first_line: true,
            terminal_seen: false,
            terminal_batch: Vec::new(),
        })
    }

    async fn process_chunk(
        &mut self,
        chunk: &[u8],
        sender: &mpsc::Sender<Result<StreamEvent>>,
    ) -> bool {
        for &byte in chunk {
            if self.skip_lf {
                self.skip_lf = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte == b'\r' || byte == b'\n' {
                self.skip_lf = byte == b'\r';
                if !self.dispatch_line(sender).await {
                    return false;
                }
            } else {
                if self.line.len() >= self.max_bytes {
                    return send_error(
                        sender,
                        AnthropicError::stream("SSE line exceeds byte limit"),
                    )
                    .await;
                }
                self.line.push(byte);
            }
        }
        // Validate this whole chunk before exposing its terminal event. Retain
        // only the stop and a bounded tail of ignorable events during look-ahead.
        self.flush_terminal(sender).await
    }

    async fn dispatch_line(&mut self, sender: &mpsc::Sender<Result<StreamEvent>>) -> bool {
        let text = match std::str::from_utf8(&self.line) {
            Ok(text) => text,
            Err(_) => {
                return send_error(sender, AnthropicError::stream("Invalid UTF-8 in SSE line"))
                    .await;
            }
        };
        let text = if self.first_line {
            self.first_line = false;
            text.strip_prefix('\u{feff}').unwrap_or(text)
        } else {
            text
        };
        let event = self.parser.parse_line(text);
        self.line.clear();
        match event {
            Ok(Some(event)) => self.queue_event(event, sender).await,
            Ok(None) => true,
            Err(error) => send_error(sender, error).await,
        }
    }

    async fn queue_event(
        &mut self,
        event: StreamEvent,
        sender: &mpsc::Sender<Result<StreamEvent>>,
    ) -> bool {
        if self.terminal_seen && !matches!(event, StreamEvent::Ping | StreamEvent::Unknown { .. }) {
            return send_error(
                sender,
                AnthropicError::stream("Event received after message_stop"),
            )
            .await;
        }
        if matches!(event, StreamEvent::MessageStop) {
            self.terminal_seen = true;
            self.terminal_batch.push(event);
            true
        } else if !self.terminal_batch.is_empty() {
            if self.terminal_batch.len() >= sender.max_capacity() {
                return send_error(
                    sender,
                    AnthropicError::stream("SSE terminal tail exceeds event limit"),
                )
                .await;
            }
            self.terminal_batch.push(event);
            true
        } else {
            sender.send(Ok(event)).await.is_ok()
        }
    }

    async fn flush_terminal(&mut self, sender: &mpsc::Sender<Result<StreamEvent>>) -> bool {
        for event in self.terminal_batch.drain(..) {
            if sender.send(Ok(event)).await.is_err() {
                return false;
            }
        }
        true
    }

    async fn finish(&mut self, sender: &mpsc::Sender<Result<StreamEvent>>) -> bool {
        if !self.line.is_empty() && !self.dispatch_line(sender).await {
            return false;
        }
        match self.parser.finish() {
            Ok(Some(event)) => {
                if !self.queue_event(event, sender).await {
                    return false;
                }
            }
            Ok(None) => {}
            Err(error) => return send_error(sender, error).await,
        }
        self.flush_terminal(sender).await
    }
}

async fn send_error(sender: &mpsc::Sender<Result<StreamEvent>>, error: AnthropicError) -> bool {
    let _ = sender.send(Err(error)).await;
    false
}
