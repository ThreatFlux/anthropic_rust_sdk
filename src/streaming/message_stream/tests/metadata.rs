//! Presence, signed content and tool-input accumulation regressions.

use super::*;

fn metadata_start() -> Value {
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
    beginning
}

#[test]
fn cumulative_usage_zero_null_and_replacement_metadata_are_exact() {
    let beginning = metadata_start();
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

fn signed_snapshot_events() -> Vec<Value> {
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
    events
}

#[test]
fn signed_thinking_compaction_fallback_and_unknown_blocks_remain_lossless() {
    let events = signed_snapshot_events();
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
