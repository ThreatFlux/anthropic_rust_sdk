//! Shared SSE fixture construction for internal regressions.

use super::*;
use crate::streaming::event_parser::EventParser;
use futures::stream;
use serde_json::{json, Value};
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

mod framing;
mod lifecycle;
mod metadata;
