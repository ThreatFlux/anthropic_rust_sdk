use super::*;

#[test]
fn choices_have_unambiguous_tagged_wire_shapes() {
    for (choice, wire) in [
        (ToolChoice::Auto, json!({"type":"auto"})),
        (ToolChoice::Any, json!({"type":"any"})),
        (
            ToolChoice::Tool {
                name: "weather".into(),
            },
            json!({"type":"tool","name":"weather"}),
        ),
        (ToolChoice::None, json!({"type":"none"})),
        (
            ToolChoice::Auto
                .with_disable_parallel_tool_use(false)
                .unwrap(),
            json!({"type":"auto","disable_parallel_tool_use":false}),
        ),
        (
            ToolChoice::Any
                .with_disable_parallel_tool_use(true)
                .unwrap(),
            json!({"type":"any","disable_parallel_tool_use":true}),
        ),
        (
            ToolChoice::Tool {
                name: "weather".into(),
            }
            .with_disable_parallel_tool_use(true)
            .unwrap(),
            json!({"type":"tool","name":"weather","disable_parallel_tool_use":true}),
        ),
    ] {
        assert_eq!(serde_json::to_value(&choice).unwrap(), wire);
        assert_eq!(serde_json::from_value::<ToolChoice>(wire).unwrap(), choice);
    }
}

#[test]
fn choices_reject_invalid_fields_and_decode_legacy_shapes() {
    assert!(ToolChoice::None
        .with_disable_parallel_tool_use(false)
        .is_err());
    for invalid in [
        json!({"type":"none","disable_parallel_tool_use":false}),
        json!({"type":"auto","disable_parallel_tool_use":null}),
        json!({"type":"tool"}),
        json!({"type":"tool","name":""}),
        json!({"type":"any","name":"weather"}),
        json!({"type":"future"}),
        json!({"type":"auto","extra":1}),
    ] {
        assert!(serde_json::from_value::<ToolChoice>(invalid).is_err());
    }
    assert_eq!(
        serde_json::from_value::<ToolChoice>(Value::Null).unwrap(),
        ToolChoice::Auto
    );
    assert_eq!(
        serde_json::from_value::<ToolChoice>(json!({"name":"weather"}))
            .unwrap()
            .kind(),
        "tool"
    );
}

#[test]
fn stop_reasons_preserve_current_and_future_strings() {
    for reason in [
        "end_turn",
        "max_tokens",
        "stop_sequence",
        "tool_use",
        "pause_turn",
        "refusal",
        "model_context_window_exceeded",
        "future_stop.v2",
    ] {
        let parsed: StopReason = serde_json::from_value(json!(reason)).unwrap();
        assert_eq!(parsed.as_str(), reason);
        assert_eq!(serde_json::to_value(parsed).unwrap(), json!(reason));
    }
    assert!(matches!(
        serde_json::from_value::<StopReason>(json!("future_stop.v2")).unwrap(),
        StopReason::Unknown(_)
    ));
    assert!(serde_json::from_value::<StopReason>(json!(1)).is_err());
}
