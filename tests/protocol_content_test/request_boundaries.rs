use super::*;

#[tokio::test]
async fn http_replay_keeps_complete_signed_history_and_matching_tool_id() {
    let server = MockServer::start().await;
    let client = fixture_client(&server);
    let content = vec![
        json!({"type":"thinking","thinking":"reasoning","signature":"original_signature","signed_metadata":{"opaque":true}}),
        json!({"type":"tool_use","id":"tool_original","name":"lookup","input":{"query":"x"},"additional":{"preserved":1}}),
        json!({"type":"future_response_block","opaque":{"nested":[1,null,3]}}),
    ];
    mount_initial_lookup_response(&server, &content).await;
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
    let replay = response
        .to_conversation_message(ReplayUnknownPolicy::Preserve)
        .unwrap();
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

#[tokio::test]
async fn all_message_request_boundaries_reject_invalid_known_roles_before_http() {
    let server = MockServer::start().await;
    let client = fixture_client(&server);
    for message in invalid_message_roles() {
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
    let server = MockServer::start().await;
    let client = fixture_client(&server);
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

async fn mount_initial_lookup_response(server: &MockServer, content: &[Value]) {
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(body_partial_json(
            json!({"messages":[{"role":"user","content":[{"type":"text","text":"look up x"}]}]}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(message_fixture(content.to_vec())))
        .expect(1)
        .up_to_n_times(1)
        .mount(server)
        .await;
}

fn invalid_message_roles() -> Vec<Message> {
    vec![
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
    ]
}
