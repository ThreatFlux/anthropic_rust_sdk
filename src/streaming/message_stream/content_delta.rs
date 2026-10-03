//! Field-specific content delta merging.

use crate::{
    error::{AnthropicError, Result},
    models::message::{ContentBlockDelta, FieldUpdate},
};
use serde_json::{json, Value};

pub(super) fn apply_content_delta(
    block: &mut Value,
    input: &mut Option<String>,
    delta: ContentBlockDelta,
    max_input: usize,
) -> Result<()> {
    let kind = block["type"].as_str().unwrap_or("");
    require_compatible_content(kind, &delta.block_type)?;
    match delta.block_type.as_str() {
        "text_delta" => append_string(block, "text", delta.text.as_deref().expect("validated"))?,
        "thinking_delta" => append_string(
            block,
            "thinking",
            delta.thinking.as_deref().expect("validated"),
        )?,
        "signature_delta" => block["signature"] = json!(delta.signature.expect("validated")),
        "citations_delta" => {
            if block.get("citations").is_none_or(Value::is_null) {
                block["citations"] = json!([]);
            }
            block["citations"]
                .as_array_mut()
                .ok_or_else(|| AnthropicError::stream("Invalid text citation snapshot"))?
                .push(serde_json::to_value(delta.citation.expect("validated"))?);
        }
        "input_json_delta" => {
            let buffer = input.get_or_insert_with(String::new);
            let fragment = delta.partial_json.expect("validated");
            if fragment.len() > max_input.saturating_sub(buffer.len()) {
                return Err(AnthropicError::stream("Tool input JSON exceeds byte limit"));
            }
            buffer.push_str(&fragment);
        }
        "compaction_delta" => {
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
fn require_compatible_content(kind: &str, delta_type: &str) -> Result<()> {
    if expected_content_kinds(delta_type).is_some_and(|expected| expected.contains(&kind)) {
        return Ok(());
    }
    Err(AnthropicError::stream(format!(
        "No safe {delta_type} merge rule for content type {kind}",
    )))
}

fn expected_content_kinds(delta_type: &str) -> Option<&'static [&'static str]> {
    match delta_type {
        "text_delta" | "citations_delta" => Some(&["text"]),
        "thinking_delta" | "signature_delta" => Some(&["thinking"]),
        "input_json_delta" => Some(&["tool_use", "server_tool_use", "mcp_tool_use"]),
        "compaction_delta" => Some(&["compaction"]),
        _ => None,
    }
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
