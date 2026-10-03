//! Bounded Server-Sent Events parsing with one consistent typed event path.

use crate::error::{AnthropicError, Result};
use std::collections::HashMap;

/// Default maximum bytes in a single SSE event (one MiB).
pub const DEFAULT_MAX_EVENT_BYTES: usize = 1024 * 1024;

/// Parser for Server-Sent Events. Unknown event names preserve their data.
#[derive(Debug)]
pub struct EventParser {
    current_event: ParsedEvent,
    max_event_bytes: usize,
}

#[derive(Debug, Default)]
struct ParsedEvent {
    event_type: Option<String>,
    data: Vec<String>,
    bytes: usize,
}

impl EventParser {
    /// Construct a parser with a one MiB event bound.
    pub fn new() -> Self {
        Self {
            current_event: ParsedEvent::default(),
            max_event_bytes: DEFAULT_MAX_EVENT_BYTES,
        }
    }

    /// Construct a parser with an explicit nonzero event bound.
    pub fn with_max_event_bytes(max_event_bytes: usize) -> Result<Self> {
        if max_event_bytes == 0 {
            return Err(AnthropicError::invalid_input(
                "SSE event byte limit must be nonzero",
            ));
        }
        Ok(Self {
            current_event: ParsedEvent::default(),
            max_event_bytes,
        })
    }

    /// Parse one complete SSE line, excluding its line ending.
    ///
    /// SSE removes at most one space after the colon; payload whitespace is not
    /// trimmed. Blank lines dispatch, comments and unsupported fields are ignored.
    pub fn parse_line(&mut self, line: &str) -> Result<Option<StreamEvent>> {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.is_empty() {
            return self.finish();
        }
        self.current_event.bytes = self
            .current_event
            .bytes
            .checked_add(line.len() + 1)
            .ok_or_else(|| AnthropicError::stream("SSE event exceeds byte limit"))?;
        if self.current_event.bytes > self.max_event_bytes {
            return Err(AnthropicError::stream("SSE event exceeds byte limit"));
        }
        if line.starts_with(':') {
            return Ok(None);
        }
        let (field, value) = line.split_once(':').unwrap_or((line, ""));
        let value = value.strip_prefix(' ').unwrap_or(value);
        match field {
            "event" => self.current_event.event_type = Some(value.to_owned()),
            "data" => self.current_event.data.push(value.to_owned()),
            _ => {}
        }
        Ok(None)
    }

    /// Dispatch a final event whose data line did not end in a blank line.
    /// Incomplete JSON in a known event is an error, never silently discarded.
    pub fn finish(&mut self) -> Result<Option<StreamEvent>> {
        let event = std::mem::take(&mut self.current_event);
        if event.data.is_empty() {
            return Ok(None);
        }
        let data = event.data.join("\n");
        self.parse_event(event.event_type.as_deref().unwrap_or("message"), &data)
            .map(Some)
    }

    /// Parse a complete event through the same path used by line framing.
    pub fn parse_event(&self, event_type: &str, data: &str) -> Result<StreamEvent> {
        if data.len() > self.max_event_bytes {
            return Err(AnthropicError::stream("SSE event exceeds byte limit"));
        }
        let known = matches!(
            event_type,
            "message_start"
                | "message_delta"
                | "message_stop"
                | "content_block_start"
                | "content_block_delta"
                | "content_block_stop"
                | "ping"
                | "error"
        );
        if !known {
            return Ok(StreamEvent::Unknown {
                event_type: event_type.into(),
                data: data.into(),
            });
        }
        let value: serde_json::Value = serde_json::from_str(data)
            .map_err(|_| AnthropicError::stream(format!("Invalid JSON in {event_type} event")))?;
        if value.get("type").and_then(serde_json::Value::as_str) != Some(event_type) {
            return Err(AnthropicError::stream(format!(
                "SSE event name and payload type differ for {event_type}"
            )));
        }
        // Keep the complete error envelope, as in the previous public contract.
        if event_type == "error" {
            let valid_error = value.get("error").is_some_and(|error| {
                error.is_object()
                    && error.get("type").is_some_and(serde_json::Value::is_string)
                    && error
                        .get("message")
                        .is_some_and(serde_json::Value::is_string)
            });
            if !valid_error {
                return Err(AnthropicError::stream("Invalid error event schema"));
            }
            let error: HashMap<String, serde_json::Value> = serde_json::from_value(value)
                .map_err(|_| AnthropicError::stream("Invalid error event object"))?;
            return Ok(StreamEvent::Error { error });
        }
        let event: StreamEvent = serde_json::from_value(value)
            .map_err(|_| AnthropicError::stream(format!("Invalid {event_type} event schema")))?;
        if let StreamEvent::ContentBlockDelta { delta, .. } = &event {
            delta.validate()?;
        }
        Ok(event)
    }
}

impl Default for EventParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Typed message stream event.
pub use crate::models::message::StreamEvent;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::message::FieldUpdate;

    #[test]
    fn unknown_events_take_the_same_path_and_preserve_whitespace() {
        let mut parser = EventParser::new();
        parser.parse_line("event: future_event").unwrap();
        parser.parse_line("data:  leading ").unwrap();
        parser.parse_line("data: second ").unwrap();
        let event = parser.parse_line("").unwrap().unwrap();
        assert_eq!(
            event,
            parser
                .parse_event("future_event", " leading \nsecond ")
                .unwrap()
        );
    }

    #[test]
    fn delta_presence_and_beta_metadata_survive() {
        let event = EventParser::new().parse_event("message_delta", r#"{"type":"message_delta","delta":{"stop_reason":null},"usage":{"output_tokens":0,"input_tokens":null},"input_transformations":[],"future":true}"#).unwrap();
        match event {
            StreamEvent::MessageDelta {
                delta,
                usage,
                input_transformations,
                extra,
                ..
            } => {
                assert_eq!(delta.stop_reason, FieldUpdate::Null);
                assert_eq!(delta.stop_sequence, FieldUpdate::Missing);
                assert_eq!(usage.output_tokens, Some(0));
                assert_eq!(usage.input_tokens, None);
                assert_eq!(
                    input_transformations,
                    FieldUpdate::Value(serde_json::json!([]))
                );
                assert_eq!(extra["future"], true);
            }
            _ => panic!("wrong event"),
        }
    }

    #[test]
    fn unknown_delta_names_preserve_fields_with_future_types() {
        let parser = EventParser::new();
        let payload = serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"future_delta","text":{"nested":[1,2]},"partial_json":false,"thinking":null,"content":["opaque"]}});
        let event = parser
            .parse_event("content_block_delta", &payload.to_string())
            .unwrap();
        assert_eq!(serde_json::to_value(&event).unwrap(), payload);
        if let StreamEvent::ContentBlockDelta { delta, .. } = event {
            assert_eq!(delta.extra["partial_json"], false);
        } else {
            panic!("wrong event");
        }
    }

    #[test]
    fn recognized_delta_retains_future_fields_with_other_known_names() {
        let payload = serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"valid","signature":{"future":1},"content":["opaque"]}});
        let event = EventParser::new()
            .parse_event("content_block_delta", &payload.to_string())
            .unwrap();
        assert_eq!(serde_json::to_value(event).unwrap(), payload);
    }

    #[test]
    fn malformed_known_events_and_oversized_frames_fail() {
        let parser = EventParser::new();
        assert!(parser.parse_event("message_stop", "{}").is_err());
        assert!(parser
            .parse_event("message_stop", r#"{"type":"ping"}"#)
            .is_err());
        assert!(parser.parse_event("error", r#"{"type":"error"}"#).is_err());
        assert!(parser
            .parse_event(
                "error",
                r#"{"type":"error","error":{"type":"future","message":42}}"#
            )
            .is_err());
        assert!(parser
            .parse_event(
                "content_block_delta",
                r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta"}}"#
            )
            .is_err());
        let mut bounded = EventParser::with_max_event_bytes(8).unwrap();
        assert!(bounded.parse_line("data: abcdef").is_err());
    }
}
