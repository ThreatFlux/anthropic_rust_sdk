//! Offline wire fixtures verified against Anthropic's Python SDK revision
//! 18f25547f20cf5f01da69ac611e700e3bc9ebf21 on 2026-10-03.

use serde_json::{json, Value};
use threatflux_anthropic_sdk::models::{
    ContentBlock, MessageBatchResult, MessageBatchResultEntry, MessageResponse, RawContentBlock,
    ReplayUnknownPolicy, Role, SendEvent, SessionEvent, StopReason, ToolChoice, Usage,
};

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

fn current_blocks() -> Vec<Value> {
    vec![
        json!({"type":"search_result","source":"https://example.org/research","title":"Research","content":[{"type":"text","text":"Evidence","revision":3}],"citations":{"enabled":true,"policy":{"next":true}},"cache_control":{"type":"ephemeral","ttl":"1h","new_cache":[]},"score":0.75}),
        json!({"type":"container_upload","file_id":"file_123","cache_control":{"type":"ephemeral","future":"retained"},"upload_status":"ready"}),
        json!({"type":"mcp_tool_use","id":"mcp_123","name":"lookup","server_name":"research","input":{"query":"example","nested":[1,2]},"server_revision":42}),
        json!({"type":"compaction","content":"summary","encrypted_content":"opaque","signature":"signed","tool_changes":[{"type":"tool_addition","tool":{"name":"lookup","new":null}}],"cache_control":{"type":"ephemeral","future":{}},"boundary":{"from":4,"to":9}}),
    ]
}

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

fn message_fixture(content: Vec<Value>) -> Value {
    json!({"id":"msg_1","type":"message","role":"assistant","model":"future-model","content":content,"stop_reason":"future_stop","stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":4}})
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

#[tokio::test]
async fn http_replay_keeps_complete_signed_history_and_matching_tool_id() {
    use threatflux_anthropic_sdk::{models::Message, models::MessageRequest, Client, Config};
    use wiremock::{
        matchers::{body_partial_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    let server = MockServer::start().await;
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    );
    let content = vec![
        json!({"type":"thinking","thinking":"reasoning","signature":"original_signature","signed_metadata":{"opaque":true}}),
        json!({"type":"tool_use","id":"tool_original","name":"lookup","input":{"query":"x"},"additional":{"preserved":1}}),
        json!({"type":"future_response_block","opaque":{"nested":[1,null,3]}}),
    ];
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(
            json!({"messages":[{"role":"user","content":[{"type":"text","text":"look up x"}]}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(message_fixture(content.clone())))
        .expect(1)
        .up_to_n_times(1)
        .mount(&server)
        .await;
    let response = client
        .messages()
        .create(
            MessageRequest::new()
                .model("future-model")
                .max_tokens(64)
                .add_user_message("look up x"),
            None,
        )
        .await
        .unwrap();
    let replay = Message::new(
        Role::Assistant,
        response
            .content
            .iter()
            .map(|block| {
                block
                    .checked_replay(&Role::Assistant, ReplayUnknownPolicy::Preserve)
                    .unwrap()
            })
            .collect(),
    );
    let result = Message::new(
        Role::User,
        vec![ContentBlock::tool_result(
            "tool_original",
            Some("result".into()),
        )],
    );
    Mock::given(method("POST")).and(path("/v1/messages"))
        .and(body_partial_json(json!({"messages":[
            {"role":"user","content":[{"type":"text","text":"look up x"}]},
            {"role":"assistant","content":content},
            {"role":"user","content":[{"type":"tool_result","tool_use_id":"tool_original","content":"result","is_error":false}]}
        ]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(message_fixture(vec![json!({"type":"text","text":"done"})])))
        .expect(1).mount(&server).await;
    let mut request = MessageRequest::new()
        .model("future-model")
        .max_tokens(64)
        .add_user_message("look up x");
    request.messages.extend([replay, result]);
    let final_response = client.messages().create(request, None).await.unwrap();
    assert_eq!(final_response.text(), "done");
    server.verify().await;
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

#[tokio::test]
async fn all_message_request_boundaries_reject_invalid_known_roles_before_http() {
    use threatflux_anthropic_sdk::{
        models::{
            batch::BatchRequestItem, Message, MessageBatchCreateRequest, MessageRequest,
            TokenCountRequest,
        },
        Client, Config,
    };
    use wiremock::MockServer;
    let server = MockServer::start().await;
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    );
    for message in [
        Message::new(
            Role::Assistant,
            vec![ContentBlock::tool_result("call_1", Some("result".into()))],
        ),
        Message::new(
            Role::User,
            vec![ContentBlock::tool_use("call_1", "lookup", json!({}))],
        ),
        Message::new(
            Role::Assistant,
            vec![ContentBlock::tool_use("call_1", "lookup", json!([]))],
        ),
        Message::new(
            Role::Assistant,
            vec![ContentBlock::Compaction {
                content: Some("".into()),
                encrypted_content: None,
                signature: None,
                tool_changes: None,
                cache_control: None,
                extra: Default::default(),
            }],
        ),
    ] {
        let mut request = MessageRequest::new().model("future-model").max_tokens(16);
        request.messages.push(message.clone());
        assert!(client
            .messages()
            .create(request.clone(), None)
            .await
            .is_err());
        assert!(client
            .messages()
            .create_stream(request.clone(), None)
            .await
            .is_err());
        let count = TokenCountRequest::new()
            .model("future-model")
            .add_message(message);
        assert!(client.messages().count_tokens(count, None).await.is_err());
        let batch = MessageBatchCreateRequest::new()
            .add_request_item(BatchRequestItem::new("request_1", request));
        assert!(client.message_batches().create(batch, None).await.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn count_and_batch_boundaries_send_explicit_future_blocks_unchanged() {
    use threatflux_anthropic_sdk::{
        models::{
            batch::BatchRequestItem, Message, MessageBatchCreateRequest, MessageRequest,
            TokenCountRequest,
        },
        Client, Config,
    };
    use wiremock::{
        matchers::{body_partial_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    let server = MockServer::start().await;
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    );
    let raw = json!({"type":"future_prompt_content","opaque":{"nested":[1,null,3]},"retain":"all"});
    let message = Message::new(Role::User, vec![ContentBlock::raw(raw.clone()).unwrap()]);
    Mock::given(method("POST"))
        .and(path("/v1/messages/count_tokens"))
        .and(body_partial_json(
            json!({"messages":[{"role":"user","content":[raw]}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"input_tokens":3})))
        .expect(1)
        .mount(&server)
        .await;
    let count = client
        .messages()
        .count_tokens(
            TokenCountRequest::new()
                .model("future-model")
                .add_message(message.clone()),
            None,
        )
        .await
        .unwrap();
    assert_eq!(count.input_tokens, 3);
    Mock::given(method("POST")).and(path("/v1/messages/batches"))
        .and(body_partial_json(json!({"requests":[{"custom_id":"request_1","params":{"messages":[{"role":"user","content":[raw]}]}}]})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id":"batch_1","type":"message_batch","processing_status":"in_progress",
            "request_counts":{"processing":1},"created_at":"2026-10-03T00:00:00Z","expires_at":"2026-10-04T00:00:00Z"
        }))).expect(1).mount(&server).await;
    let mut request = MessageRequest::new().model("future-model").max_tokens(16);
    request.messages.push(message);
    let batch = MessageBatchCreateRequest::new()
        .add_request_item(BatchRequestItem::new("request_1", request));
    assert_eq!(
        client
            .message_batches()
            .create(batch, None)
            .await
            .unwrap()
            .id,
        "batch_1"
    );
    server.verify().await;
}

#[tokio::test]
async fn session_user_messages_use_their_own_block_union_and_preserve_raw_extensions() {
    use threatflux_anthropic_sdk::{
        models::{DocumentSource, ImageSource},
        Client, Config,
    };
    use wiremock::{
        matchers::{body_partial_json, header, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    let server = MockServer::start().await;
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    );
    for block in [
        ContentBlock::tool_result("call_1", Some("result".into())),
        ContentBlock::tool_use("call_1", "lookup", json!({})),
        ContentBlock::search_result("source", "title", ["text".into()]),
        ContentBlock::container_upload("file_1"),
        ContentBlock::document(DocumentSource::content(vec![
            json!({"type":"text","text":"inline"}),
        ])),
        ContentBlock::Image {
            source: ImageSource::Url {
                url: "https://example.org/image".into(),
                extra: std::collections::HashMap::from([("url".into(), json!("collision"))]),
            },
            extra: Default::default(),
        },
    ] {
        assert!(client
            .sessions()
            .events("session_1")
            .send(
                SendEvent::UserMessage {
                    content: vec![block]
                },
                None
            )
            .await
            .is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    let content = vec![
        json!({"type":"text","text":"message","future_metadata":true}),
        json!({"type":"image","source":{"type":"file","file_id":"file_1","future_source":3}}),
        json!({"type":"document","source":{"type":"text","media_type":"text/plain","data":"document"},"title":"title","future_document":null}),
        json!({"type":"redacted","future_policy":"kept"}),
        json!({"type":"future_session_content","payload":{"opaque":true}}),
    ];
    Mock::given(method("POST")).and(path("/v1/sessions/session_1/events"))
        .and(header("anthropic-beta","managed-agents-2026-04-01"))
        .and(body_partial_json(json!({"type":"user.message","content":content})))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"type":"user.message","id":"event_1","processed_at":"2026-10-03T00:00:00Z","content":content})))
        .expect(1).mount(&server).await;
    let blocks = content
        .into_iter()
        .map(|value| ContentBlock::raw(value).unwrap())
        .collect();
    let event = client
        .sessions()
        .events("session_1")
        .send(SendEvent::UserMessage { content: blocks }, None)
        .await
        .unwrap();
    let SessionEvent::UserMessage { content, .. } = event else {
        panic!("expected user event")
    };
    assert_eq!(
        serde_json::to_value(content).unwrap()[4],
        json!({"type":"future_session_content","payload":{"opaque":true}})
    );
    server.verify().await;
}

#[tokio::test]
async fn initial_session_events_validate_known_user_messages_without_rewriting_extensions() {
    use threatflux_anthropic_sdk::{models::SessionCreateRequest, Client, Config};
    use wiremock::{
        matchers::{body_partial_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };
    let server = MockServer::start().await;
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    );
    for event in [
        json!({"type":"user.message","content":[{"type":"tool_result","tool_use_id":"c","content":"invalid surface"}]}),
        json!({"type":"user.message","content":[{"type":"search_result","source":"s","title":"t","content":[{"type":"text","text":"t"}]}]}),
        json!({"type":"user.message","content":[{"type":"text"}]}),
        json!({"type":"user.message","content":"not an array"}),
        json!({"type":"user.message"}),
        json!({"content":[]}),
        json!(null),
    ] {
        let request = SessionCreateRequest::new("agent_1").initial_event(event);
        assert!(client.sessions().create(request, None).await.is_err());
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    let events = vec![
        json!({"type":"user.message","content":[{"type":"redacted","future":{"preserved":true}},{"type":"text","text":"hello","source_extra":null}],"future_event_metadata":[1,null,3]}),
        json!({"type":"future_initial_event","payload":{"unchanged":true}}),
    ];
    Mock::given(method("POST"))
        .and(path("/v1/sessions"))
        .and(body_partial_json(json!({"initial_events":events})))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            json!({"id":"session_1","type":"session","status":"idle","agent":"agent_1"}),
        ))
        .expect(1)
        .mount(&server)
        .await;
    let mut request = SessionCreateRequest::new("agent_1");
    for event in events {
        request = request.initial_event(event);
    }
    assert_eq!(
        client.sessions().create(request, None).await.unwrap().id,
        "session_1"
    );
    server.verify().await;
}
