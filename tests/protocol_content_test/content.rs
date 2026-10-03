use super::*;

#[test]
fn current_and_unknown_content_round_trip_without_payload_loss() {
    for value in current_blocks().into_iter().chain([
        json!({"type":"future_content","nested":{"unknown":{"type":"even_newer","value":null}},"array":[null,1,"x"],"payload":"opaque"}),
        json!({"type":"compaction","content":null,"encrypted_content":"opaque","signature":"signed","tool_changes":[]}),
        json!({"type":"image","source":{"type":"future_source","opaque":{"x":1}},"future":{"nested":true}}),
        json!({"type":"document","source":{"type":"file","file_id":"file_1","checksum":"abc"},"citations":{"enabled":true,"style":"new"},"revision":1}),
        json!({"type":"text","text":"citations","citations":[{"type":"future_citation","payload":{"text":"intact"}},{"type":"char_location","cited_text":"c","document_index":0,"start_char_index":0,"end_char_index":1,"confidence":0.9}],"cache_control":{"type":"ephemeral","ttl":"1h","scope":{"x":1}},"extra_text":{"x":null}}),
        json!({"type":"tool_result","tool_use_id":"tool_1","content":[{"type":"text","text":"ok","unknown":"nested"},{"type":"future_tool_content","opaque":{"x":1}}],"is_error":false,"future_result":"kept"}),
    ]) {
        let block = ContentBlock::raw(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(block).unwrap(), value);
    }
}

#[test]
fn malformed_recognized_blocks_do_not_become_unknown() {
    for value in [
        json!({"type":"text"}),
        json!({"type":"text","text":3}),
        json!({"type":"image","source":{"type":"url"}}),
        json!({"type":"document","source":{"type":"file","file_id":1}}),
        json!({"type":"tool_use","id":"a","name":"b","input":[]}),
        json!({"type":"mcp_tool_use","id":"a","name":"b","input":{}}),
        json!({"type":"search_result","source":"url","title":"t","content":[{"type":"image","source":{"type":"url","url":"u"}}]}),
        json!({"type":"container_upload","file_id":null}),
        json!({"type":"fallback"}),
        json!({"type":"fallback","from":{"model":1},"to":{"model":"other"},"trigger":{"type":"refusal"}}),
        json!({"type":"fallback","from":{"model":"a"},"to":{"model":"b"}}),
        json!({"type":"fallback","from":{"model":"a"},"to":{"model":"b"},"trigger":{"type":"refusal","category":5}}),
        json!({"type":"text","text":"t","citations":[{"type":"char_location","cited_text":"bad"}]}),
        json!({"type":"tool_result","tool_use_id":"a","content":[{"type":"text"}]}),
        json!({"type":1}),
        json!({"payload":true}),
        json!(null),
        json!([]),
    ] {
        assert!(
            ContentBlock::raw(value.clone()).is_err(),
            "accepted {value}"
        );
    }
}

#[test]
fn extras_cannot_override_typed_or_optional_protocol_fields() {
    let mut text = ContentBlock::text("intact");
    if let ContentBlock::Text { extra, .. } = &mut text {
        extra.insert("text".into(), json!("overwrite"));
    }
    assert!(serde_json::to_value(&text).is_err());
    if let ContentBlock::Text { extra, .. } = &mut text {
        extra.clear();
        extra.insert("citations".into(), json!([]));
    }
    assert!(serde_json::to_value(text).is_err());
    let mut cache = threatflux_anthropic_sdk::models::CacheControl::ephemeral();
    cache.extra.insert("type".into(), json!("changed"));
    assert!(serde_json::to_value(cache).is_err());
    let mut usage = Usage::new(1, 2);
    usage.extra.insert("input_tokens".into(), json!(100));
    assert!(serde_json::to_value(usage).is_err());
    let raw = RawContentBlock::new(json!({"type":"text","text":"bypass"})).unwrap();
    assert!(serde_json::to_value(ContentBlock::Unknown(raw)).is_err());
}

#[test]
fn checked_replay_keeps_signatures_and_requires_explicit_unknown_policy() {
    let signed = ContentBlock::raw(json!({"type":"thinking","thinking":"signed reasoning","signature":"original","server_state":{"sealed":true}})).unwrap();
    assert_eq!(
        signed
            .checked_replay(&Role::Assistant, ReplayUnknownPolicy::Reject)
            .unwrap(),
        signed
    );
    assert!(signed
        .checked_replay(&Role::User, ReplayUnknownPolicy::Preserve)
        .is_err());
    let unsigned = ContentBlock::raw(json!({"type":"thinking","thinking":"partial"})).unwrap();
    assert!(unsigned
        .checked_replay(&Role::Assistant, ReplayUnknownPolicy::Preserve)
        .is_err());
    let future =
        ContentBlock::raw(json!({"type":"future_block","payload":{"opaque":true}})).unwrap();
    assert!(future
        .checked_replay(&Role::Assistant, ReplayUnknownPolicy::Reject)
        .is_err());
    assert_eq!(
        future
            .checked_replay(&Role::Assistant, ReplayUnknownPolicy::Preserve)
            .unwrap(),
        future
    );
    let unknown_nested = ContentBlock::raw(
        json!({"type":"image","source":{"type":"new_source","payload":"opaque"}}),
    )
    .unwrap();
    assert!(unknown_nested
        .checked_replay(&Role::User, ReplayUnknownPolicy::Reject)
        .is_err());
    assert_eq!(
        unknown_nested
            .checked_replay(&Role::User, ReplayUnknownPolicy::Preserve)
            .unwrap(),
        unknown_nested
    );
}

#[test]
fn shared_union_retains_messages_batches_and_session_blocks() {
    let blocks = vec![
        json!({"type":"thinking","thinking":"reasoning","signature":"s","opaque_state":true}),
        json!({"type":"mcp_tool_use","id":"mcp1","name":"lookup","server_name":"s","input":{},"extra":1}),
        json!({"type":"compaction","content":null,"encrypted_content":"sealed","signature":"sig"}),
        json!({"type":"new_response_block","payload":{"keep":"all"}}),
    ];
    let response: MessageResponse =
        serde_json::from_value(message_fixture(blocks.clone())).unwrap();
    assert_eq!(
        serde_json::to_value(&response.content).unwrap(),
        json!(blocks)
    );
    assert_eq!(response.stop_reason.unwrap().as_str(), "future_stop");
    let batch: MessageBatchResultEntry = serde_json::from_value(json!({"custom_id":"request1","result":{"type":"succeeded","message":message_fixture(blocks.clone())}})).unwrap();
    let MessageBatchResult::Succeeded { message } = batch.result else {
        panic!("expected succeeded batch")
    };
    assert_eq!(
        serde_json::to_value(message.content).unwrap(),
        json!(blocks)
    );
    let session: SessionEvent = serde_json::from_value(json!({"type":"agent.message","id":"event1","processed_at":"2026-10-03T00:00:00Z","content":blocks})).unwrap();
    let SessionEvent::AgentMessage { content, .. } = session else {
        panic!("expected session message")
    };
    assert_eq!(serde_json::to_value(&content).unwrap(), json!(blocks));
    let send = SendEvent::UserMessage { content };
    assert_eq!(
        serde_json::to_value(send).unwrap()["content"],
        json!(blocks)
    );
}

#[test]
fn usage_preserves_nested_metadata_and_future_iteration_shapes() {
    let fixture = json!({
        "input_tokens":3,"output_tokens":7,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,
        "cache_creation":{"ephemeral_5m_input_tokens":1,"ephemeral_1h_input_tokens":2,"extra":9},
        "server_tool_use":{"web_search_requests":1,"web_fetch_requests":2,"new_requests":3},
        "output_tokens_details":{"thinking_tokens":5,"new_category":2},
        "iterations":[{"type":"message","input_tokens":3},{"type":"future_iteration","encrypted":{"bytes":"opaque"}}],
        "fallback_credit":{"type":"redeemed","future":null},"speed":"fast","future_usage":{"foo":null}
    });
    let usage: Usage = serde_json::from_value(fixture.clone()).unwrap();
    assert_eq!(usage.total_tokens(), 10);
    assert_eq!(serde_json::to_value(usage).unwrap(), fixture);
    let nullable: Usage = serde_json::from_value(json!({"input_tokens":1,"output_tokens":2,"cache_creation_input_tokens":null,"cache_read_input_tokens":null})).unwrap();
    assert_eq!(nullable.total_tokens(), 3);
}

#[test]
fn empty_compaction_response_is_preserved_but_cannot_be_replayed() {
    let fixture = json!({"type":"compaction","content":"","signature":"opaque"});
    let block = ContentBlock::raw(fixture.clone()).unwrap();
    assert_eq!(serde_json::to_value(&block).unwrap(), fixture);
    assert!(block
        .checked_replay(&Role::Assistant, ReplayUnknownPolicy::Preserve)
        .is_err());
}

#[test]
fn inline_documents_validate_known_shapes_and_preserve_unfamiliar_nested_blocks() {
    let fixture = json!({"type":"document","source":{"type":"content","content":[
        {"type":"text","text":"known","extra":true},
        {"type":"future_inline_block","sealed":{"x":null}}
    ],"source_extra":9}});
    let block = ContentBlock::raw(fixture.clone()).unwrap();
    assert_eq!(serde_json::to_value(&block).unwrap(), fixture);
    assert!(block
        .checked_replay(&Role::User, ReplayUnknownPolicy::Reject)
        .is_err());
    assert_eq!(
        block
            .checked_replay(&Role::User, ReplayUnknownPolicy::Preserve)
            .unwrap(),
        block
    );
    for nested in [
        json!({"type":"text"}),
        json!({"type":"image","source":{"type":"url"}}),
        json!({"type":"tool_use","id":"i","name":"n","input":{}}),
    ] {
        assert!(ContentBlock::raw(
            json!({"type":"document","source":{"type":"content","content":[nested]}})
        )
        .is_err());
    }
    let search = ContentBlock::raw(json!({"type":"search_result","source":"s","title":"t","content":[{"type":"future_text","opaque":true}]})).unwrap();
    assert!(search
        .checked_replay(&Role::User, ReplayUnknownPolicy::Reject)
        .is_err());
    assert!(search
        .checked_replay(&Role::User, ReplayUnknownPolicy::Preserve)
        .is_ok());
}
