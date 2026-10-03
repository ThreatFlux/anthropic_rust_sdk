use super::*;

#[tokio::test]
async fn session_user_messages_use_their_own_block_union_and_preserve_raw_extensions() {
    let server = MockServer::start().await;
    let client = fixture_client(&server);
    for block in invalid_session_blocks() {
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
    let server = MockServer::start().await;
    let client = fixture_client(&server);
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

fn invalid_session_blocks() -> Vec<ContentBlock> {
    vec![
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
    ]
}
