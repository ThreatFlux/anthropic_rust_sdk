//! Server-Sent Events (SSE) parser for streaming responses

use crate::error::{AnthropicError, Result};
use std::collections::HashMap;

/// Parser for Server-Sent Events (SSE) streams
#[derive(Debug)]
pub struct EventParser {
    current_event: Option<ParsedEvent>,
}

#[derive(Debug)]
struct ParsedEvent {
    event_type: Option<String>,
    data: Vec<String>,
    id: Option<String>,
    retry: Option<u32>,
}

impl EventParser {
    /// Create a new event parser
    pub fn new() -> Self {
        Self {
            current_event: None,
        }
    }

    /// Parse a line from the SSE stream
    pub fn parse_line(
        &mut self,
        line: &str,
    ) -> Result<Option<crate::models::message::StreamEvent>> {
        let line = line.trim();

        // Empty line indicates end of event
        if line.is_empty() {
            return self.finish_event();
        }

        // Comments start with ':'
        if line.starts_with(':') {
            return Ok(None); // Ignore comments
        }

        // Ensure we have a current event
        if self.current_event.is_none() {
            self.current_event = Some(ParsedEvent {
                event_type: None,
                data: Vec::new(),
                id: None,
                retry: None,
            });
        }

        let event = self.current_event.as_mut().unwrap();

        // Parse field
        if let Some((field, value)) = line.split_once(':') {
            let field = field.trim();
            let value = value.trim();

            match field {
                "event" => {
                    event.event_type = Some(value.to_string());
                }
                "data" => {
                    event.data.push(value.to_string());
                }
                "id" => {
                    event.id = Some(value.to_string());
                }
                "retry" => {
                    if let Ok(retry_ms) = value.parse() {
                        event.retry = Some(retry_ms);
                    }
                }
                _ => {
                    // Unknown field, ignore
                }
            }
        } else {
            // Line without colon is treated as data
            event.data.push(line.to_string());
        }

        Ok(None)
    }

    /// Parse JSON data with error handling
    fn parse_json_data<T>(&self, data: &str, event_type: &str) -> Result<T>
    where
        T: serde::de::DeserializeOwned,
    {
        serde_json::from_str(data).map_err(|e| {
            AnthropicError::stream(format!("Failed to parse {} event: {}", event_type, e))
        })
    }

    /// Parse a single event (for testing purposes)
    pub fn parse_event(
        &self,
        event_type: &str,
        data: &str,
    ) -> Result<crate::models::message::StreamEvent> {
        match event_type {
            "ping" => Ok(crate::models::message::StreamEvent::Ping),
            "error" => {
                let error_data: HashMap<String, serde_json::Value> =
                    self.parse_json_data(data, event_type)?;
                Ok(crate::models::message::StreamEvent::Error { error: error_data })
            }
            "message_start" => {
                let parsed: MessageStartData = self.parse_json_data(data, event_type)?;
                Ok(crate::models::message::StreamEvent::MessageStart {
                    message: parsed.message,
                })
            }
            "message_delta" => {
                let parsed: MessageDeltaData = self.parse_json_data(data, event_type)?;
                Ok(crate::models::message::StreamEvent::MessageDelta {
                    delta: parsed.delta,
                    usage: parsed.usage,
                })
            }
            "message_stop" => Ok(crate::models::message::StreamEvent::MessageStop),
            "content_block_start" => {
                let parsed: ContentBlockStartData = self.parse_json_data(data, event_type)?;
                Ok(crate::models::message::StreamEvent::ContentBlockStart {
                    index: parsed.index,
                    content_block: parsed.content_block,
                })
            }
            "content_block_delta" => {
                let parsed: ContentBlockDeltaData = self.parse_json_data(data, event_type)?;
                Ok(crate::models::message::StreamEvent::ContentBlockDelta {
                    index: parsed.index,
                    delta: parsed.delta,
                })
            }
            "content_block_stop" => {
                let parsed: ContentBlockStopData = self.parse_json_data(data, event_type)?;
                Ok(crate::models::message::StreamEvent::ContentBlockStop {
                    index: parsed.index,
                })
            }
            _ => Err(AnthropicError::stream(format!(
                "Unknown event type: {}",
                event_type
            ))),
        }
    }

    /// Finish parsing the current event
    fn finish_event(&mut self) -> Result<Option<crate::models::message::StreamEvent>> {
        let event = match self.current_event.take() {
            Some(event) => event,
            None => return Ok(None), // No event to finish
        };

        // Join data lines with newlines
        let data = event.data.join("\n");
        if data.is_empty() {
            return Ok(None);
        }

        let event_type = event.event_type.as_deref().unwrap_or("message");

        match event_type {
            "ping" => Ok(Some(crate::models::message::StreamEvent::Ping)),
            "error" => {
                let error_data: HashMap<String, serde_json::Value> =
                    self.parse_json_data(&data, event_type)?;
                Ok(Some(crate::models::message::StreamEvent::Error {
                    error: error_data,
                }))
            }
            "message_start" => {
                let parsed: MessageStartData = self.parse_json_data(&data, event_type)?;
                Ok(Some(crate::models::message::StreamEvent::MessageStart {
                    message: parsed.message,
                }))
            }
            "message_delta" => {
                let parsed: MessageDeltaData = self.parse_json_data(&data, event_type)?;
                Ok(Some(crate::models::message::StreamEvent::MessageDelta {
                    delta: parsed.delta,
                    usage: parsed.usage,
                }))
            }
            "message_stop" => Ok(Some(crate::models::message::StreamEvent::MessageStop)),
            "content_block_start" => {
                let parsed: ContentBlockStartData = self.parse_json_data(&data, event_type)?;
                Ok(Some(
                    crate::models::message::StreamEvent::ContentBlockStart {
                        index: parsed.index,
                        content_block: parsed.content_block,
                    },
                ))
            }
            "content_block_delta" => {
                let parsed: ContentBlockDeltaData = self.parse_json_data(&data, event_type)?;
                Ok(Some(
                    crate::models::message::StreamEvent::ContentBlockDelta {
                        index: parsed.index,
                        delta: parsed.delta,
                    },
                ))
            }
            "content_block_stop" => {
                let parsed: ContentBlockStopData = self.parse_json_data(&data, event_type)?;
                Ok(Some(
                    crate::models::message::StreamEvent::ContentBlockStop {
                        index: parsed.index,
                    },
                ))
            }
            _ => {
                // Unknown event type, ignore or log
                tracing::warn!("Unknown event type: {}", event_type);
                Ok(None)
            }
        }
    }
}

impl Default for EventParser {
    fn default() -> Self {
        Self::new()
    }
}

// Helper structs for parsing specific event data

#[derive(serde::Deserialize)]
struct MessageStartData {
    #[serde(rename = "type")]
    _type: String,
    message: crate::models::message::MessageResponse,
}

#[derive(serde::Deserialize)]
struct MessageDeltaData {
    #[serde(rename = "type")]
    _type: String,
    delta: crate::models::message::MessageDelta,
    #[serde(default)]
    usage: crate::models::common::Usage,
}

#[derive(serde::Deserialize)]
struct ContentBlockStartData {
    #[serde(rename = "type")]
    _type: String,
    index: usize,
    content_block: crate::models::common::ContentBlock,
}

#[derive(serde::Deserialize)]
struct ContentBlockDeltaData {
    #[serde(rename = "type")]
    _type: String,
    index: usize,
    delta: crate::models::message::ContentBlockDelta,
}

#[derive(serde::Deserialize)]
struct ContentBlockStopData {
    #[serde(rename = "type")]
    _type: String,
    index: usize,
}

// Type alias for convenience
pub use crate::models::message::StreamEvent;

#[cfg(test)]
mod tests {
    use super::EventParser;
    use crate::models::{StreamEvent, TextCitation};

    #[test]
    fn test_parse_content_block_delta_with_citation() {
        let parser = EventParser::new();
        let event = parser
            .parse_event(
                "content_block_delta",
                r#"{
                    "type":"content_block_delta",
                    "index":0,
                    "delta":{
                        "type":"citations_delta",
                        "citation":{
                            "type":"search_result_location",
                            "search_result_index":1,
                            "source":"web_search",
                            "title":"Result"
                        }
                    }
                }"#,
            )
            .unwrap();

        match event {
            StreamEvent::ContentBlockDelta { delta, .. } => {
                assert!(matches!(
                    delta.citation,
                    Some(TextCitation::SearchResultLocation { .. })
                ));
            }
            _ => panic!("Expected ContentBlockDelta"),
        }
    }

    #[test]
    fn test_parse_message_delta_with_extended_usage() {
        let parser = EventParser::new();
        let event = parser
            .parse_event(
                "message_delta",
                r#"{
                    "type":"message_delta",
                    "delta":{"stop_reason":"end_turn"},
                    "usage":{
                        "output_tokens":5,
                        "cache_creation_input_tokens":3,
                        "cache_read_input_tokens":7,
                        "server_tool_use":{"web_search_requests":2},
                        "service_tier":"standard"
                    }
                }"#,
            )
            .unwrap();

        match event {
            StreamEvent::MessageDelta { usage, .. } => {
                assert_eq!(usage.output_tokens, 5);
                assert_eq!(usage.cache_creation_input_tokens, 3);
                assert_eq!(usage.cache_read_input_tokens, 7);
                assert_eq!(usage.server_tool_use.unwrap().web_search_requests, 2);
                assert_eq!(usage.service_tier.as_deref(), Some("standard"));
            }
            _ => panic!("Expected MessageDelta"),
        }
    }
}
