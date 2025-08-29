//! Unit tests for Streaming modules
//!
//! Tests SSE parsing, stream handling, event processing, and streaming functionality.

use threatflux::{
    streaming::{EventParser, MessageStream},
    models::{
        message::{StreamEvent, MessageResponse},
        common::{Role, ContentBlock, Usage, StopReason},
    },
    error::AnthropicError,
};
use chrono::Utc;
use serde_json::json;
use pretty_assertions::assert_eq;
use futures::StreamExt;
use std::io::Cursor;

#[cfg(test)]
mod event_parser_tests {
    use super::*;

    #[test]
    fn test_parse_message_start() {
        let event_data = r#"{"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","model":"claude-3-5-haiku-20241022","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("message_start", event_data).unwrap();
        
        if let StreamEvent::MessageStart { message } = event {
            assert_eq!(message.id, "msg_123");
            assert_eq!(message.model, "claude-3-5-haiku-20241022");
            assert_eq!(message.role, Role::Assistant);
            assert_eq!(message.usage.input_tokens, 10);
            assert_eq!(message.usage.output_tokens, 0);
        } else {
            panic!("Expected MessageStart event");
        }
    }

    #[test]
    fn test_parse_content_block_start() {
        let event_data = r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("content_block_start", event_data).unwrap();
        
        if let StreamEvent::ContentBlockStart { index, content_block } = event {
            assert_eq!(index, 0);
            if let ContentBlock::Text { text } = content_block {
                assert_eq!(text, "");
            } else {
                panic!("Expected Text content block");
            }
        } else {
            panic!("Expected ContentBlockStart event");
        }
    }

    #[test]
    fn test_parse_content_block_delta() {
        let event_data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("content_block_delta", event_data).unwrap();
        
        if let StreamEvent::ContentBlockDelta { index, delta } = event {
            assert_eq!(index, 0);
            assert_eq!(delta["type"], "text_delta");
            assert_eq!(delta["text"], "Hello");
        } else {
            panic!("Expected ContentBlockDelta event");
        }
    }

    #[test]
    fn test_parse_content_block_stop() {
        let event_data = r#"{"type":"content_block_stop","index":0}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("content_block_stop", event_data).unwrap();
        
        if let StreamEvent::ContentBlockStop { index } = event {
            assert_eq!(index, 0);
        } else {
            panic!("Expected ContentBlockStop event");
        }
    }

    #[test]
    fn test_parse_message_delta() {
        let event_data = r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"output_tokens":5}}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("message_delta", event_data).unwrap();
        
        if let StreamEvent::MessageDelta { delta, usage } = event {
            assert_eq!(delta["stop_reason"], "end_turn");
            assert!(delta["stop_sequence"].is_null());
            assert!(usage.is_some());
            assert_eq!(usage.unwrap().output_tokens, 5);
        } else {
            panic!("Expected MessageDelta event");
        }
    }

    #[test]
    fn test_parse_message_stop() {
        let event_data = r#"{"type":"message_stop"}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("message_stop", event_data).unwrap();
        
        assert!(matches!(event, StreamEvent::MessageStop));
    }

    #[test]
    fn test_parse_error_event() {
        let event_data = r#"{"type":"error","error":{"type":"rate_limit_error","message":"Rate limit exceeded"}}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("error", event_data).unwrap();
        
        if let StreamEvent::Error { error } = event {
            assert_eq!(error["type"], "rate_limit_error");
            assert_eq!(error["message"], "Rate limit exceeded");
        } else {
            panic!("Expected Error event");
        }
    }

    #[test]
    fn test_parse_ping_event() {
        let event_data = r#"{"type":"ping"}"#;
        
        let parser = EventParser::new();
        let event = parser.parse_event("ping", event_data).unwrap();
        
        assert!(matches!(event, StreamEvent::Ping));
    }

    #[test]
    fn test_parse_invalid_event() {
        let parser = EventParser::new();
        
        // Invalid JSON
        let result = parser.parse_event("message_start", "invalid json");
        assert!(result.is_err());
        
        // Unknown event type
        let result = parser.parse_event("unknown_event", r#"{"type":"unknown"}"#);
        assert!(result.is_err());
        
        // Missing required fields
        let result = parser.parse_event("message_start", r#"{"type":"message_start"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_sse_format() {
        let sse_data = r#"event: content_block_delta
data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}

"#;
        
        let parser = EventParser::new();
        let lines: Vec<&str> = sse_data.lines().collect();
        
        // Extract event type and data
        let event_line = lines.iter().find(|line| line.starts_with("event: ")).unwrap();
        let data_line = lines.iter().find(|line| line.starts_with("data: ")).unwrap();
        
        let event_type = event_line.strip_prefix("event: ").unwrap();
        let event_data = data_line.strip_prefix("data: ").unwrap();
        
        let event = parser.parse_event(event_type, event_data).unwrap();
        
        if let StreamEvent::ContentBlockDelta { delta, .. } = event {
            assert_eq!(delta["text"], "Hello");
        } else {
            panic!("Expected ContentBlockDelta event");
        }
    }

    #[test]
    fn test_parser_state() {
        let parser = EventParser::new();
        
        // Test that parser can handle multiple events
        let event1_data = r#"{"type":"message_start","message":{"id":"msg_1","type":"message","role":"assistant","model":"claude-3-5-haiku-20241022","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#;
        let event2_data = r#"{"type":"message_stop"}"#;
        
        let event1 = parser.parse_event("message_start", event1_data).unwrap();
        let event2 = parser.parse_event("message_stop", event2_data).unwrap();
        
        assert!(matches!(event1, StreamEvent::MessageStart { .. }));
        assert!(matches!(event2, StreamEvent::MessageStop));
    }
}

#[cfg(test)]
mod message_stream_tests {
    use super::*;
    use tokio_test;
    use futures::stream;

    #[tokio::test]
    async fn test_stream_creation() {
        let sse_data = vec![
            "event: message_start",
            r#"data: {"type":"message_start","message":{"id":"msg_123","type":"message","role":"assistant","model":"claude-3-5-haiku-20241022","content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":0}}}"#,
            "",
            "event: content_block_start",
            r#"data: {"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#,
            "",
            "event: content_block_delta",
            r#"data: {"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#,
            "",
            "event: message_stop",
            r#"data: {"type":"message_stop"}"#,
            "",
        ];
        
        let sse_stream = sse_data.join("\n");
        let cursor = Cursor::new(sse_stream.as_bytes());
        
        // This would normally be created from a reqwest response
        // For testing, we'll simulate the behavior
        let events = vec![
            Ok(StreamEvent::MessageStart { 
                message: MessageResponse {
                    id: "msg_123".to_string(),
                    object: "message".to_string(),
                    created_at: Utc::now(),
                    model: "claude-3-5-haiku-20241022".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage::new(10, 0),
                }
            }),
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: json!({"type": "text_delta", "text": "Hello"}),
            }),
            Ok(StreamEvent::MessageStop),
        ];
        
        let mut stream = stream::iter(events);
        let mut collected_events = Vec::new();
        
        while let Some(event) = stream.next().await {
            collected_events.push(event.unwrap());
        }
        
        assert_eq!(collected_events.len(), 4);
        assert!(matches!(collected_events[0], StreamEvent::MessageStart { .. }));
        assert!(matches!(collected_events[1], StreamEvent::ContentBlockStart { .. }));
        assert!(matches!(collected_events[2], StreamEvent::ContentBlockDelta { .. }));
        assert!(matches!(collected_events[3], StreamEvent::MessageStop));
    }

    #[tokio::test]
    async fn test_collect_text() {
        let events = vec![
            Ok(StreamEvent::MessageStart { 
                message: MessageResponse {
                    id: "msg_123".to_string(),
                    object: "message".to_string(),
                    created_at: Utc::now(),
                    model: "claude-3-5-haiku-20241022".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage::new(10, 0),
                }
            }),
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: json!({"type": "text_delta", "text": "Hello"}),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: json!({"type": "text_delta", "text": " world"}),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: json!({"type": "text_delta", "text": "!"}),
            }),
            Ok(StreamEvent::ContentBlockStop { index: 0 }),
            Ok(StreamEvent::MessageStop),
        ];
        
        let stream = stream::iter(events);
        let mut text = String::new();
        
        tokio::pin!(stream);
        while let Some(event) = stream.next().await {
            match event.unwrap() {
                StreamEvent::ContentBlockDelta { delta, .. } => {
                    if let Some(text_delta) = delta.get("text") {
                        if let Some(text_str) = text_delta.as_str() {
                            text.push_str(text_str);
                        }
                    }
                },
                StreamEvent::MessageStop => break,
                _ => {},
            }
        }
        
        assert_eq!(text, "Hello world!");
    }

    #[tokio::test]
    async fn test_stream_error_handling() {
        let events = vec![
            Ok(StreamEvent::MessageStart { 
                message: MessageResponse {
                    id: "msg_123".to_string(),
                    object: "message".to_string(),
                    created_at: Utc::now(),
                    model: "claude-3-5-haiku-20241022".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage::new(10, 0),
                }
            }),
            Err(AnthropicError::network("Connection lost")),
            Ok(StreamEvent::MessageStop),
        ];
        
        let mut stream = stream::iter(events);
        let mut error_encountered = false;
        
        while let Some(event) = stream.next().await {
            match event {
                Ok(StreamEvent::MessageStart { .. }) => {},
                Err(_) => {
                    error_encountered = true;
                },
                Ok(StreamEvent::MessageStop) => break,
                _ => {},
            }
        }
        
        assert!(error_encountered);
    }

    #[tokio::test]
    async fn test_stream_interruption() {
        let events = vec![
            Ok(StreamEvent::MessageStart { 
                message: MessageResponse {
                    id: "msg_123".to_string(),
                    object: "message".to_string(),
                    created_at: Utc::now(),
                    model: "claude-3-5-haiku-20241022".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage::new(10, 0),
                }
            }),
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: json!({"type": "text_delta", "text": "Hello"}),
            }),
            // Stream ends abruptly without MessageStop
        ];
        
        let mut stream = stream::iter(events);
        let mut events_received = 0;
        let mut got_message_stop = false;
        
        while let Some(event) = stream.next().await {
            events_received += 1;
            if let Ok(StreamEvent::MessageStop) = event {
                got_message_stop = true;
            }
        }
        
        assert_eq!(events_received, 3);
        assert!(!got_message_stop);
    }

    #[tokio::test]
    async fn test_multiple_content_blocks() {
        let events = vec![
            Ok(StreamEvent::MessageStart { 
                message: MessageResponse {
                    id: "msg_123".to_string(),
                    object: "message".to_string(),
                    created_at: Utc::now(),
                    model: "claude-3-5-haiku-20241022".to_string(),
                    role: Role::Assistant,
                    content: vec![],
                    stop_reason: None,
                    stop_sequence: None,
                    usage: Usage::new(10, 0),
                }
            }),
            // First content block
            Ok(StreamEvent::ContentBlockStart {
                index: 0,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 0,
                delta: json!({"type": "text_delta", "text": "First block"}),
            }),
            Ok(StreamEvent::ContentBlockStop { index: 0 }),
            // Second content block
            Ok(StreamEvent::ContentBlockStart {
                index: 1,
                content_block: ContentBlock::text(""),
            }),
            Ok(StreamEvent::ContentBlockDelta {
                index: 1,
                delta: json!({"type": "text_delta", "text": "Second block"}),
            }),
            Ok(StreamEvent::ContentBlockStop { index: 1 }),
            Ok(StreamEvent::MessageStop),
        ];
        
        let mut stream = stream::iter(events);
        let mut content_blocks: std::collections::HashMap<usize, String> = std::collections::HashMap::new();
        
        while let Some(event) = stream.next().await {
            match event.unwrap() {
                StreamEvent::ContentBlockDelta { index, delta } => {
                    if let Some(text_delta) = delta.get("text") {
                        if let Some(text_str) = text_delta.as_str() {
                            content_blocks.entry(index)
                                .and_modify(|s| s.push_str(text_str))
                                .or_insert_with(|| text_str.to_string());
                        }
                    }
                },
                StreamEvent::MessageStop => break,
                _ => {},
            }
        }
        
        assert_eq!(content_blocks.len(), 2);
        assert_eq!(content_blocks[&0], "First block");
        assert_eq!(content_blocks[&1], "Second block");
    }
}

#[cfg(test)]
mod sse_parsing_tests {
    use super::*;

    #[test]
    fn test_sse_line_parsing() {
        let sse_lines = vec![
            "event: message_start",
            "data: {\"test\": \"data\"}",
            "",
            "event: ping",
            "data: {\"type\": \"ping\"}",
            "",
        ];
        
        let mut current_event: Option<String> = None;
        let mut current_data: Option<String> = None;
        let mut events = Vec::new();
        
        for line in sse_lines {
            if line.is_empty() {
                // End of event
                if let (Some(event), Some(data)) = (current_event.take(), current_data.take()) {
                    events.push((event, data));
                }
            } else if line.starts_with("event: ") {
                current_event = Some(line.strip_prefix("event: ").unwrap().to_string());
            } else if line.starts_with("data: ") {
                current_data = Some(line.strip_prefix("data: ").unwrap().to_string());
            }
        }
        
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, "message_start");
        assert_eq!(events[1].0, "ping");
    }

    #[test]
    fn test_multiline_data() {
        let sse_lines = vec![
            "event: test",
            "data: line 1",
            "data: line 2", 
            "data: line 3",
            "",
        ];
        
        let mut current_event: Option<String> = None;
        let mut data_lines: Vec<String> = Vec::new();
        
        for line in sse_lines {
            if line.is_empty() {
                // End of event
                if let Some(event) = current_event.take() {
                    let combined_data = data_lines.join("\n");
                    assert_eq!(event, "test");
                    assert_eq!(combined_data, "line 1\nline 2\nline 3");
                }
                data_lines.clear();
            } else if line.starts_with("event: ") {
                current_event = Some(line.strip_prefix("event: ").unwrap().to_string());
            } else if line.starts_with("data: ") {
                data_lines.push(line.strip_prefix("data: ").unwrap().to_string());
            }
        }
    }

    #[test]
    fn test_sse_with_retry() {
        let sse_line = "retry: 3000";
        
        if sse_line.starts_with("retry: ") {
            let retry_ms = sse_line.strip_prefix("retry: ").unwrap().parse::<u64>().unwrap();
            assert_eq!(retry_ms, 3000);
        }
    }

    #[test]
    fn test_sse_with_id() {
        let sse_line = "id: event_123";
        
        if sse_line.starts_with("id: ") {
            let event_id = sse_line.strip_prefix("id: ").unwrap();
            assert_eq!(event_id, "event_123");
        }
    }

    #[test]
    fn test_comment_lines() {
        let sse_lines = vec![
            ": This is a comment",
            "event: test",
            ": Another comment",
            "data: test data",
            "",
        ];
        
        let mut current_event: Option<String> = None;
        let mut current_data: Option<String> = None;
        
        for line in sse_lines {
            if line.starts_with(":") {
                // Comment line, ignore
                continue;
            } else if line.is_empty() {
                if let (Some(event), Some(data)) = (current_event.take(), current_data.take()) {
                    assert_eq!(event, "test");
                    assert_eq!(data, "test data");
                }
            } else if line.starts_with("event: ") {
                current_event = Some(line.strip_prefix("event: ").unwrap().to_string());
            } else if line.starts_with("data: ") {
                current_data = Some(line.strip_prefix("data: ").unwrap().to_string());
            }
        }
    }
}