//! Bounded message streaming and strict, presence-aware message accumulation.

use crate::{
    error::{AnthropicError, Result},
    models::common::ContentBlock,
    models::message::{ContentBlockDelta, FieldUpdate, MessageResponse, StreamEvent},
    streaming::event_parser::{EventParser, DEFAULT_MAX_EVENT_BYTES},
};
use futures::{Stream, StreamExt};
use serde_json::{json, Value};
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

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

/// Preserve UTF-8 across chunks and recognize LF, CRLF and CR line endings.
/// At most one bounded line and one bounded parsed event are retained.
async fn pump_lines<S, B>(mut bytes: S, sender: mpsc::Sender<Result<StreamEvent>>, max_bytes: usize)
where
    S: Stream<Item = reqwest::Result<B>> + Unpin + Send + 'static,
    B: AsRef<[u8]> + Send + 'static,
{
    let mut parser = match EventParser::with_max_event_bytes(max_bytes) {
        Ok(parser) => parser,
        Err(error) => {
            let _ = sender.send(Err(error)).await;
            return;
        }
    };
    let mut line = Vec::new();
    let mut skip_lf = false;
    let mut first_line = true;
    let mut terminal_seen = false;
    let mut terminal_batch = Vec::new();
    loop {
        let chunk = tokio::select! { _ = sender.closed() => return, chunk = bytes.next() => chunk };
        let Some(chunk) = chunk else {
            break;
        };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => {
                let _ = sender
                    .send(Err(AnthropicError::stream("HTTP stream transport failed")))
                    .await;
                return;
            }
        };
        for &byte in chunk.as_ref() {
            if skip_lf {
                skip_lf = false;
                if byte == b'\n' {
                    continue;
                }
            }
            if byte == b'\r' || byte == b'\n' {
                skip_lf = byte == b'\r';
                if !dispatch_line(
                    &mut line,
                    &mut first_line,
                    &mut parser,
                    &sender,
                    &mut terminal_seen,
                    &mut terminal_batch,
                )
                .await
                {
                    return;
                }
            } else {
                if line.len() >= max_bytes {
                    let _ = sender
                        .send(Err(AnthropicError::stream("SSE line exceeds byte limit")))
                        .await;
                    return;
                }
                line.push(byte);
            }
        }
        // Validate the rest of this HTTP chunk before publishing a terminal
        // event, so a same-chunk duplicate cannot race collector cancellation.
        // The look-ahead contains only the stop and bounded ignorable events.
        for event in terminal_batch.drain(..) {
            if sender.send(Ok(event)).await.is_err() {
                return;
            }
        }
    }
    if !line.is_empty()
        && !dispatch_line(
            &mut line,
            &mut first_line,
            &mut parser,
            &sender,
            &mut terminal_seen,
            &mut terminal_batch,
        )
        .await
    {
        return;
    }
    match parser.finish() {
        Ok(Some(event)) => {
            if !queue_event(event, &sender, &mut terminal_seen, &mut terminal_batch).await {
                return;
            }
        }
        Ok(None) => {}
        Err(error) => {
            let _ = sender.send(Err(error)).await;
            return;
        }
    }
    for event in terminal_batch {
        if sender.send(Ok(event)).await.is_err() {
            return;
        }
    }
}

async fn dispatch_line(
    line: &mut Vec<u8>,
    first_line: &mut bool,
    parser: &mut EventParser,
    sender: &mpsc::Sender<Result<StreamEvent>>,
    terminal_seen: &mut bool,
    terminal_batch: &mut Vec<StreamEvent>,
) -> bool {
    let text = match std::str::from_utf8(line) {
        Ok(text) => text,
        Err(_) => {
            let _ = sender
                .send(Err(AnthropicError::stream("Invalid UTF-8 in SSE line")))
                .await;
            return false;
        }
    };
    let text = if *first_line {
        *first_line = false;
        text.strip_prefix('\u{feff}').unwrap_or(text)
    } else {
        text
    };
    let event = parser.parse_line(text);
    line.clear();
    match event {
        Ok(Some(event)) => queue_event(event, sender, terminal_seen, terminal_batch).await,
        Ok(None) => true,
        Err(error) => {
            let _ = sender.send(Err(error)).await;
            false
        }
    }
}

async fn queue_event(
    event: StreamEvent,
    sender: &mpsc::Sender<Result<StreamEvent>>,
    terminal_seen: &mut bool,
    terminal_batch: &mut Vec<StreamEvent>,
) -> bool {
    if *terminal_seen && !matches!(event, StreamEvent::Ping | StreamEvent::Unknown { .. }) {
        let _ = sender
            .send(Err(AnthropicError::stream(
                "Event received after message_stop",
            )))
            .await;
        return false;
    }
    if matches!(event, StreamEvent::MessageStop) {
        *terminal_seen = true;
        terminal_batch.push(event);
        true
    } else if !terminal_batch.is_empty() {
        if terminal_batch.len() >= sender.max_capacity() {
            let _ = sender
                .send(Err(AnthropicError::stream(
                    "SSE terminal tail exceeds event limit",
                )))
                .await;
            return false;
        }
        terminal_batch.push(event);
        true
    } else {
        sender.send(Ok(event)).await.is_ok()
    }
}

struct MessageAccumulator {
    message: Option<MessageResponse>,
    blocks: Vec<Value>,
    open: Vec<bool>,
    inputs: Vec<Option<String>>,
    stopped: bool,
    limits: StreamLimits,
}

impl MessageAccumulator {
    fn new(limits: StreamLimits) -> Self {
        Self {
            message: None,
            blocks: Vec::new(),
            open: Vec::new(),
            inputs: Vec::new(),
            stopped: false,
            limits,
        }
    }
    fn apply(&mut self, event: StreamEvent) -> Result<()> {
        if matches!(event, StreamEvent::Ping | StreamEvent::Unknown { .. }) {
            return Ok(());
        }
        if self.stopped {
            return Err(AnthropicError::stream("Event received after message_stop"));
        }
        match event {
            StreamEvent::Ping | StreamEvent::Unknown { .. } => {}
            StreamEvent::Error { .. } => {
                return Err(AnthropicError::stream(
                    "API returned an error event during message streaming",
                ))
            }
            StreamEvent::MessageStart { message } => {
                if self.message.is_some() {
                    return Err(AnthropicError::stream("Duplicate message_start"));
                }
                if message.content.len() > self.limits.max_content_blocks {
                    return Err(AnthropicError::stream("Content block limit exceeded"));
                }
                self.blocks = message
                    .content
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<std::result::Result<_, _>>()?;
                self.open = vec![false; self.blocks.len()];
                self.inputs = vec![None; self.blocks.len()];
                self.message = Some(message);
            }
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                self.require_start()?;
                if index != self.blocks.len() {
                    return Err(AnthropicError::stream(
                        "Content block start index must be the next contiguous index",
                    ));
                }
                if index >= self.limits.max_content_blocks {
                    return Err(AnthropicError::stream("Content block limit exceeded"));
                }
                let block = serde_json::to_value(content_block)?;
                if block["type"] == "fallback" {
                    let model = block
                        .get("to")
                        .and_then(|to| to.get("model"))
                        .and_then(Value::as_str)
                        .ok_or_else(|| {
                            AnthropicError::stream("Fallback block is missing to.model")
                        })?;
                    self.message.as_mut().expect("start checked").model = model.to_owned();
                }
                self.blocks.push(block);
                self.open.push(true);
                self.inputs.push(None);
            }
            StreamEvent::ContentBlockDelta { index, delta } => {
                self.require_open(index)?;
                delta.validate()?;
                apply_content_delta(
                    &mut self.blocks[index],
                    &mut self.inputs[index],
                    delta,
                    self.limits.max_tool_input_bytes,
                )?;
            }
            StreamEvent::ContentBlockStop { index } => {
                self.require_open(index)?;
                if let Some(input) = self.inputs[index].take() {
                    let value: Value = serde_json::from_str(&input)
                        .map_err(|_| AnthropicError::stream("Invalid completed tool input JSON"))?;
                    if !value.is_object() {
                        return Err(AnthropicError::stream(
                            "Completed tool input JSON must be an object",
                        ));
                    }
                    self.blocks[index]["input"] = value;
                }
                // Validate the completed block without coercing malformed known payloads.
                serde_json::from_value::<ContentBlock>(self.blocks[index].clone())
                    .map_err(|_| AnthropicError::stream("Invalid completed content block"))?;
                self.open[index] = false;
            }
            StreamEvent::MessageDelta {
                delta,
                usage,
                context_management,
                input_transformations,
                ..
            } => {
                self.require_start()?;
                let message = self.message.as_mut().expect("start checked");
                delta.stop_reason.apply(&mut message.stop_reason);
                delta.stop_sequence.apply(&mut message.stop_sequence);
                delta.stop_details.apply(&mut message.stop_details);
                delta.container.apply_non_null(&mut message.container);
                context_management.apply_non_null(&mut message.context_management);
                input_transformations.apply_non_null(&mut message.input_transformations);
                usage.apply(&mut message.usage);
            }
            StreamEvent::MessageStop => {
                self.require_start()?;
                if self.open.iter().any(|open| *open) || self.inputs.iter().any(Option::is_some) {
                    return Err(AnthropicError::stream(
                        "message_stop received with unfinished content blocks",
                    ));
                }
                self.stopped = true;
            }
        }
        Ok(())
    }
    fn require_start(&self) -> Result<()> {
        if self.message.is_none() {
            return Err(AnthropicError::stream(
                "Event received before message_start",
            ));
        }
        Ok(())
    }
    fn require_open(&self, index: usize) -> Result<()> {
        self.require_start()?;
        if !self.open.get(index).copied().unwrap_or(false) {
            return Err(AnthropicError::stream(
                "Content delta/stop index does not refer to an open block",
            ));
        }
        Ok(())
    }
    fn finish(self) -> Result<MessageResponse> {
        if !self.stopped {
            return Err(AnthropicError::stream("Stream ended before message_stop"));
        }
        let mut message = self
            .message
            .ok_or_else(|| AnthropicError::stream("No message_start received"))?;
        message.content = self
            .blocks
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()?;
        Ok(message)
    }
}

fn apply_content_delta(
    block: &mut Value,
    input: &mut Option<String>,
    delta: ContentBlockDelta,
    max_input: usize,
) -> Result<()> {
    let kind = block["type"].as_str().unwrap_or("");
    match delta.block_type.as_str() {
        "text_delta" if kind == "text" => {
            append_string(block, "text", delta.text.as_deref().expect("validated"))?
        }
        "thinking_delta" if kind == "thinking" => append_string(
            block,
            "thinking",
            delta.thinking.as_deref().expect("validated"),
        )?,
        "signature_delta" if kind == "thinking" => {
            block["signature"] = json!(delta.signature.expect("validated"))
        }
        "citations_delta" if kind == "text" => {
            if block.get("citations").is_none_or(Value::is_null) {
                block["citations"] = json!([]);
            }
            block["citations"]
                .as_array_mut()
                .ok_or_else(|| AnthropicError::stream("Invalid text citation snapshot"))?
                .push(serde_json::to_value(delta.citation.expect("validated"))?);
        }
        "input_json_delta" if matches!(kind, "tool_use" | "server_tool_use" | "mcp_tool_use") => {
            let buffer = input.get_or_insert_with(String::new);
            let fragment = delta.partial_json.expect("validated");
            if fragment.len() > max_input.saturating_sub(buffer.len()) {
                return Err(AnthropicError::stream("Tool input JSON exceeds byte limit"));
            }
            buffer.push_str(&fragment);
        }
        "compaction_delta" if kind == "compaction" => {
            // These are whole snapshots. The official beta schema normalizes omitted
            // optional compaction fields to null before replacing both fields.
            block["content"] = field_value(delta.content);
            block["encrypted_content"] = field_value(delta.encrypted_content);
        }
        _ => {
            return Err(AnthropicError::stream(format!(
                "No safe {} merge rule for content type {kind}",
                delta.block_type
            )))
        }
    }
    Ok(())
}
fn field_value<T: serde::Serialize>(field: FieldUpdate<T>) -> Value {
    match field {
        FieldUpdate::Value(value) => json!(value),
        _ => Value::Null,
    }
}
fn append_string(block: &mut Value, key: &str, fragment: &str) -> Result<()> {
    match block.get_mut(key) {
        Some(Value::String(value)) => {
            value.push_str(fragment);
            Ok(())
        }
        _ => Err(AnthropicError::stream(format!("Invalid {key} snapshot"))),
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
mod tests {
    use super::*;
    use futures::stream;
    use tokio::sync::oneshot;
    use tokio::time::{timeout, Duration};

    fn start() -> Value {
        json!({"type":"message_start","message":{
            "id":"msg_fixture","type":"message","role":"assistant","model":"claude-fable-5-1",
            "content":[],"stop_reason":null,"stop_sequence":null,
            "usage":{"input_tokens":15,"output_tokens":1,"cache_creation_input_tokens":3,"cache_read_input_tokens":7}
        }})
    }
    fn block(index: usize, content: Value) -> Value {
        json!({"type":"content_block_start","index":index,"content_block":content})
    }
    fn delta(index: usize, delta: Value) -> Value {
        json!({"type":"content_block_delta","index":index,"delta":delta})
    }
    fn stop_block(index: usize) -> Value {
        json!({"type":"content_block_stop","index":index})
    }
    fn stop() -> Value {
        json!({"type":"message_stop"})
    }
    fn frame(event: &Value) -> Vec<u8> {
        format!(
            "event: {}\ndata: {}\n\n",
            event["type"].as_str().unwrap(),
            event
        )
        .into_bytes()
    }
    fn fixture_stream(events: &[Value]) -> MessageStream {
        let chunks: Vec<reqwest::Result<Vec<u8>>> =
            events.iter().map(|event| Ok(frame(event))).collect();
        MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default())
    }
    fn accumulator(events: &[Value]) -> Result<MessageResponse> {
        let mut acc = MessageAccumulator::new(StreamLimits::default());
        let parser = EventParser::new();
        for event in events {
            acc.apply(parser.parse_event(event["type"].as_str().unwrap(), &event.to_string())?)?;
        }
        acc.finish()
    }

    #[tokio::test]
    async fn every_lifecycle_truncation_fails_for_both_collectors() {
        let events = [
            start(),
            block(0, json!({"type":"text","text":""})),
            delta(0, json!({"type":"text_delta","text":"hello"})),
            stop_block(0),
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":4}}),
            stop(),
        ];
        for truncated in 0..events.len() {
            assert!(
                fixture_stream(&events[..truncated])
                    .collect_message()
                    .await
                    .is_err(),
                "truncated at {truncated}"
            );
            assert!(
                fixture_stream(&events[..truncated])
                    .collect_text()
                    .await
                    .is_err(),
                "text truncated at {truncated}"
            );
        }
        assert_eq!(
            fixture_stream(&events).collect_text().await.unwrap(),
            "hello"
        );
        assert!(fixture_stream(&[start(), stop()])
            .collect_text()
            .await
            .unwrap()
            .is_empty());
    }

    #[test]
    fn invalid_order_indices_duplicate_events_and_open_blocks_fail() {
        for events in [
            vec![stop()],
            vec![start(), start(), stop()],
            vec![start(), stop(), stop()],
            vec![start(), block(1, json!({"type":"text","text":""})), stop()],
            vec![
                start(),
                delta(0, json!({"type":"text_delta","text":"x"})),
                stop(),
            ],
            vec![start(), block(0, json!({"type":"text","text":""})), stop()],
            vec![
                start(),
                block(0, json!({"type":"text","text":""})),
                stop_block(0),
                stop_block(0),
                stop(),
            ],
            vec![
                start(),
                block(0, json!({"type":"text","text":""})),
                stop_block(0),
                delta(0, json!({"type":"text_delta","text":"x"})),
                stop(),
            ],
        ] {
            assert!(accumulator(&events).is_err(), "invalid fixture {events:?}");
        }
    }

    #[test]
    fn cumulative_usage_zero_null_and_replacement_metadata_are_exact() {
        let mut beginning = start();
        beginning["message"]["stop_reason"] = json!("stop_sequence");
        beginning["message"]["stop_sequence"] = json!("old");
        beginning["message"]["stop_details"] = json!({"type":"refusal","category":"old"});
        beginning["message"]["container"] = json!({"id":"container-start"});
        beginning["message"]["diagnostics"] = json!({"trace":"retained"});
        beginning["message"]["context_management"] = json!({"applied_edits":["old"]});
        beginning["message"]["input_transformations"] = json!([{"type":"old"}]);
        beginning["message"]["future_message"] = json!({"nested":1});
        beginning["message"]["usage"]["speed"] = json!("fast");
        beginning["message"]["usage"]["service_tier"] = json!("priority");
        beginning["message"]["usage"]["cache_creation"] =
            json!({"ephemeral_5m_input_tokens":3,"ephemeral_1h_input_tokens":0,"future":1});
        beginning["message"]["usage"]["server_tool_use"] =
            json!({"web_search_requests":9,"web_fetch_requests":3});
        let first = json!({"type":"message_delta","delta":{"stop_reason":null,"stop_sequence":null,"stop_details":null,"container":null},"usage":{"output_tokens":9,"input_tokens":20,"cache_read_input_tokens":null,"server_tool_use":{"web_search_requests":2},"iterations":[{"type":"message","input_tokens":20},{"type":"compaction","input_tokens":2},{"type":"advisor_message","input_tokens":3},{"type":"fallback_message","input_tokens":4},{"type":"future_iteration","opaque":[1,2]}],"fallback_credit":{"tokens":9},"output_tokens_details":{"thinking_tokens":7}},"context_management":{"applied_edits":[]},"input_transformations":[]});
        let second = json!({"type":"message_delta","delta":{"stop_reason":"end_turn","container":{"id":"container-final"}},"usage":{"output_tokens":0,"input_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"server_tool_use":null,"iterations":null,"fallback_credit":null,"service_tier":"ignored-delta","cache_creation":{"ephemeral_5m_input_tokens":99}},"context_management":null,"input_transformations":null});
        let result = accumulator(&[beginning, first, second, stop()]).unwrap();
        assert_eq!(result.usage.output_tokens, 0);
        assert_eq!(result.usage.input_tokens, 0);
        assert_eq!(result.usage.cache_creation_input_tokens, 0);
        assert_eq!(result.usage.cache_read_input_tokens, 0);
        assert_eq!(result.usage.server_tool_use.unwrap().web_search_requests, 2);
        assert_eq!(result.usage.service_tier.as_deref(), Some("priority"));
        assert_eq!(result.usage.speed.as_deref(), Some("fast"));
        assert_eq!(
            result
                .usage
                .cache_creation
                .unwrap()
                .ephemeral_5m_input_tokens,
            3
        );
        assert_eq!(
            result.usage.output_tokens_details.unwrap().thinking_tokens,
            7
        );
        assert_eq!(
            result.usage.iterations.unwrap()[4]["type"],
            "future_iteration"
        );
        assert_eq!(result.usage.fallback_credit.unwrap()["tokens"], 9);
        assert_eq!(
            result.stop_reason,
            Some(crate::models::common::StopReason::EndTurn)
        );
        assert!(result.stop_sequence.is_none());
        assert!(result.stop_details.is_none());
        assert_eq!(result.container.unwrap()["id"], "container-final");
        assert_eq!(result.diagnostics.unwrap()["trace"], "retained");
        assert_eq!(
            result.context_management.unwrap(),
            json!({"applied_edits":[]})
        );
        assert_eq!(result.input_transformations.unwrap(), json!([]));
        assert_eq!(result.extra["future_message"], json!({"nested":1}));
    }

    #[test]
    fn signed_thinking_compaction_fallback_and_unknown_blocks_remain_lossless() {
        let events = vec![
            start(),
            block(
                0,
                json!({"type":"thinking","thinking":"first ","signature":"initial","future":true}),
            ),
            delta(0, json!({"type":"thinking_delta","thinking":"last"})),
            delta(
                0,
                json!({"type":"signature_delta","signature":"signed-one"}),
            ),
            delta(
                0,
                json!({"type":"signature_delta","signature":"signed-final"}),
            ),
            stop_block(0),
            block(
                1,
                json!({"type":"compaction","content":"old","encrypted_content":"old-opaque","future":"retained"}),
            ),
            delta(
                1,
                json!({"type":"compaction_delta","content":"snapshot-one","encrypted_content":"opaque-one"}),
            ),
            delta(
                1,
                json!({"type":"compaction_delta","content":null,"encrypted_content":"opaque-final"}),
            ),
            stop_block(1),
            block(
                2,
                json!({"type":"fallback","from":{"model":"claude-fable-5-1"},"to":{"model":"claude-opus-5-5"},"trigger":{"type":"refusal"}}),
            ),
            stop_block(2),
            block(
                3,
                json!({"type":"future_block","opaque":{"type":"nested","x":[1,2]}}),
            ),
            stop_block(3),
            stop(),
        ];
        let result = accumulator(&events).unwrap();
        let value = serde_json::to_value(&result).unwrap();
        assert_eq!(result.model, "claude-opus-5-5");
        assert_eq!(value["content"][0]["thinking"], "first last");
        assert_eq!(value["content"][0]["signature"], "signed-final");
        assert_eq!(value["content"][0]["future"], true);
        assert_eq!(value["content"][1]["content"], Value::Null);
        assert_eq!(value["content"][1]["encrypted_content"], "opaque-final");
        assert_eq!(value["content"][3], events[12]["content_block"]);
    }

    #[test]
    fn unknown_content_deltas_and_malformed_tool_json_never_become_success() {
        for input in ["{\"x\":", "\"string\"", "null", "[]", "true"] {
            let events = vec![
                start(),
                block(
                    0,
                    json!({"type":"tool_use","id":"tool_1","name":"lookup","input":{}}),
                ),
                delta(0, json!({"type":"input_json_delta","partial_json":input})),
                stop_block(0),
                stop(),
            ];
            assert!(accumulator(&events).is_err());
        }
        let events = vec![
            start(),
            block(0, json!({"type":"future_block","opaque":1})),
            delta(0, json!({"type":"future_delta","opaque":2})),
            stop_block(0),
            stop(),
        ];
        assert!(accumulator(&events).is_err());
        let events = vec![
            start(),
            block(
                0,
                json!({"type":"mcp_tool_use","id":"tool_1","name":"lookup","server_name":"srv","input":{}}),
            ),
            delta(
                0,
                json!({"type":"input_json_delta","partial_json":"{\"x\":"}),
            ),
            delta(
                0,
                json!({"type":"input_json_delta","partial_json":"\"é\"}"}),
            ),
            stop_block(0),
            stop(),
        ];
        let result = accumulator(&events).unwrap();
        assert_eq!(
            serde_json::to_value(result.content).unwrap()[0]["input"],
            json!({"x":"é"})
        );
    }

    #[tokio::test]
    async fn all_chunk_splits_utf8_crlf_multiline_and_final_frame_are_supported() {
        let events = [
            start(),
            block(0, json!({"type":"text","text":""})),
            delta(0, json!({"type":"text_delta","text":"é😀"})),
            stop_block(0),
            stop(),
        ];
        let mut bytes = String::from_utf8(events.iter().flat_map(frame).collect())
            .unwrap()
            .replace('\n', "\r\n")
            .into_bytes();
        // The terminal JSON is complete, but no final line or blank-line delimiter.
        bytes.truncate(bytes.len() - 4);
        for split in 0..=bytes.len() {
            let chunks = vec![Ok(bytes[..split].to_vec()), Ok(bytes[split..].to_vec())];
            let response = MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default())
                .collect_message()
                .await
                .unwrap();
            assert_eq!(response.text(), "é😀", "split at {split}");
        }
        let chunks: Vec<reqwest::Result<Vec<u8>>> =
            bytes.iter().map(|byte| Ok(vec![*byte])).collect();
        assert_eq!(
            MessageStream::from_bytes(stream::iter(chunks), StreamLimits::default())
                .collect_text()
                .await
                .unwrap(),
            "é😀"
        );
        let multiline=b"event: message_start\ndata: {\ndata: \"type\":\"message_start\",\ndata: \"message\":{\"id\":\"m\",\"type\":\"message\",\"role\":\"assistant\",\"model\":\"custom\",\"content\":[],\"stop_reason\":null,\"stop_sequence\":null,\"usage\":{}}\ndata: }\n\nevent: message_stop\ndata: {\"type\":\"message_stop\"}";
        assert!(MessageStream::from_bytes(
            stream::iter(vec![Ok(multiline.to_vec())]),
            StreamLimits::default()
        )
        .collect_message()
        .await
        .is_ok());
    }

    #[tokio::test]
    async fn invalid_utf8_oversized_lines_and_tool_input_are_bounded() {
        let invalid = vec![Ok(b"data: \xff\n\n".to_vec())];
        let mut raw = MessageStream::from_bytes(stream::iter(invalid), StreamLimits::default());
        assert!(raw.next().await.unwrap().is_err());
        assert!(raw.next().await.is_none());
        let limits = StreamLimits {
            max_event_bytes: 8,
            ..StreamLimits::default()
        };
        let mut raw =
            MessageStream::from_bytes(stream::iter(vec![Ok(b"data: 123456789".to_vec())]), limits);
        assert!(raw.next().await.unwrap().is_err());
        let mut acc = MessageAccumulator::new(StreamLimits {
            max_tool_input_bytes: 2,
            ..StreamLimits::default()
        });
        let parser = EventParser::new();
        for event in [
            start(),
            block(
                0,
                json!({"type":"tool_use","id":"t","name":"test","input":{}}),
            ),
        ] {
            acc.apply(
                parser
                    .parse_event(event["type"].as_str().unwrap(), &event.to_string())
                    .unwrap(),
            )
            .unwrap();
        }
        assert!(acc
            .apply(
                parser
                    .parse_event(
                        "content_block_delta",
                        &delta(
                            0,
                            json!({"type":"input_json_delta","partial_json":"{\"large\":1}"})
                        )
                        .to_string()
                    )
                    .unwrap()
            )
            .is_err());
    }

    #[tokio::test]
    async fn events_arrive_incrementally_and_unknown_events_remain_raw() {
        let (sender, receiver) = mpsc::channel::<reqwest::Result<Vec<u8>>>(2);
        let mut stream = MessageStream::from_bytes(
            tokio_stream::wrappers::ReceiverStream::new(receiver),
            StreamLimits::default(),
        );
        sender.send(Ok(frame(&start()))).await.unwrap();
        assert!(matches!(
            timeout(Duration::from_secs(1), stream.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            StreamEvent::MessageStart { .. }
        ));
        sender
            .send(Ok(b"event: future_event\ndata: opaque text\n\n".to_vec()))
            .await
            .unwrap();
        assert!(
            matches!(stream.next().await.unwrap().unwrap(),StreamEvent::Unknown{event_type,data} if event_type=="future_event" && data=="opaque text")
        );
        drop(sender);
        assert!(stream.next().await.is_none());
    }

    #[tokio::test]
    async fn terminal_collection_cancels_a_body_that_never_reaches_http_eof() {
        for text_only in [false, true] {
            let (sender, receiver) = mpsc::channel::<reqwest::Result<Vec<u8>>>(1);
            let stream = MessageStream::from_bytes(
                tokio_stream::wrappers::ReceiverStream::new(receiver),
                StreamLimits::default(),
            );
            let body = [
                start(),
                block(0, json!({"type":"text","text":""})),
                delta(0, json!({"type":"text_delta","text":"completed"})),
                stop_block(0),
                stop(),
            ]
            .iter()
            .flat_map(frame)
            .collect();
            sender.send(Ok(body)).await.unwrap();
            // Keeping the sender alive deliberately withholds HTTP EOF.
            let collected = timeout(Duration::from_secs(1), async move {
                if text_only {
                    stream.collect_text().await
                } else {
                    stream.collect_message().await.map(|message| {
                        message
                            .content
                            .iter()
                            .filter_map(ContentBlock::as_text)
                            .collect()
                    })
                }
            })
            .await
            .expect("message_stop must complete collection without HTTP EOF")
            .unwrap();
            assert_eq!(collected, "completed");
            timeout(Duration::from_secs(1), sender.closed())
                .await
                .expect("collection must cancel the body producer");
        }
    }

    #[tokio::test]
    async fn same_chunk_terminal_duplicates_fail_before_terminal_publication() {
        let body = [start(), stop(), stop()]
            .iter()
            .flat_map(frame)
            .collect::<Vec<_>>();
        let error =
            MessageStream::from_bytes(stream::iter(vec![Ok(body)]), StreamLimits::default())
                .collect_message()
                .await
                .unwrap_err();
        assert!(matches!(error, AnthropicError::Stream(_)));
    }

    #[tokio::test]
    async fn raw_events_keep_the_http_body_alive_after_message_stop() {
        let (sender, receiver) = mpsc::channel::<reqwest::Result<Vec<u8>>>(1);
        let mut stream = MessageStream::from_bytes(
            tokio_stream::wrappers::ReceiverStream::new(receiver),
            StreamLimits::default(),
        );
        sender.send(Ok(frame(&stop()))).await.unwrap();
        assert!(matches!(
            stream.next().await.unwrap().unwrap(),
            StreamEvent::MessageStop
        ));
        assert!(timeout(Duration::from_millis(20), stream.next())
            .await
            .is_err());
        sender
            .send(Ok(b"event: future_event\ndata: retained\n\n".to_vec()))
            .await
            .unwrap();
        assert!(matches!(
            stream.next().await.unwrap().unwrap(),
            StreamEvent::Unknown { .. }
        ));
        stream.close();
    }

    struct PendingBytes {
        dropped: Option<oneshot::Sender<()>>,
    }
    impl Stream for PendingBytes {
        type Item = reqwest::Result<Vec<u8>>;
        fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }
    impl Drop for PendingBytes {
        fn drop(&mut self) {
            if let Some(sender) = self.dropped.take() {
                let _ = sender.send(());
            }
        }
    }
    #[tokio::test]
    async fn dropping_consumer_cancels_even_a_pending_http_body() {
        let (sender, receiver) = oneshot::channel();
        let stream = MessageStream::from_bytes(
            PendingBytes {
                dropped: Some(sender),
            },
            StreamLimits::default(),
        );
        tokio::task::yield_now().await;
        drop(stream);
        timeout(Duration::from_secs(1), receiver)
            .await
            .unwrap()
            .unwrap();
    }
}
