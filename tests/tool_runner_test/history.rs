use super::*;

#[tokio::test]
async fn multiple_turns_preserve_signed_history_and_never_execute_server_or_mcp_tools() {
    let server = MockServer::start().await;
    let first = vec![
        json!({"type":"thinking","thinking":"reasoning","signature":"original","signed_extra":{"kept":1}}),
        json!({"type":"server_tool_use","id":"server1","name":"search","input":{}}),
        json!({"type":"mcp_tool_use","id":"mcp1","name":"calculate","server_name":"remote","input":{"value":9}}),
        tool_call("client1", "calculate", json!({"value":2})),
    ];
    sequence(
        &server,
        vec![
            response(first.clone(), Some("tool_use")),
            response(
                vec![tool_call("client2", "calculate", json!({"value":3}))],
                Some("tool_use"),
            ),
            response(vec![json!({"type":"text","text":"done"})], Some("end_turn")),
        ],
    )
    .await;
    let count = Arc::new(AtomicUsize::new(0));
    let result = runner(
        &server,
        counting_registry(count.clone()),
        ToolRunnerOptions::default(),
    )
    .run(request(), None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
    assert_eq!(count.load(Ordering::SeqCst), 2);
    assert_eq!(result.started_calls, vec!["client1", "client2"]);
    assert_eq!(result.total_tokens(), 15);
    assert_eq!(result.messages.len(), 6);
    let requests = server.received_requests().await.unwrap();
    let second: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(second["messages"][1]["content"], json!(first));
    assert_eq!(second["messages"][2]["role"], "user");
    assert_eq!(
        second["messages"][2]["content"][0]["tool_use_id"],
        "client1"
    );
    assert_eq!(second["tool_choice"], json!({"type":"auto"}));
    assert_eq!(second["model"], "future-model");
    assert!(second.get("thinking").is_none());
    server.verify().await;
}

#[tokio::test]
async fn reused_call_id_is_not_executed_twice() {
    let server = MockServer::start().await;
    sequence(
        &server,
        vec![
            response(
                vec![tool_call("a", "calculate", json!({"value":1}))],
                Some("tool_use"),
            ),
            response(
                vec![tool_call("a", "calculate", json!({"value":2}))],
                Some("tool_use"),
            ),
        ],
    )
    .await;
    let count = Arc::new(AtomicUsize::new(0));
    let result = runner(
        &server,
        counting_registry(count.clone()),
        ToolRunnerOptions::default(),
    )
    .run(request(), None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::InvalidToolCall);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(result.started_calls, vec!["a"]);
}

#[tokio::test]
async fn stop_reasons_and_server_only_tool_turns_never_execute_callbacks() {
    for (reason, expected) in [
        (Some("end_turn"), ToolRunnerTermination::EndTurn),
        (Some("max_tokens"), ToolRunnerTermination::MaxTokens),
        (Some("stop_sequence"), ToolRunnerTermination::StopSequence),
        (Some("refusal"), ToolRunnerTermination::Refusal),
        (Some("pause_turn"), ToolRunnerTermination::PauseTurn),
        (
            Some("model_context_window_exceeded"),
            ToolRunnerTermination::ContextWindowExceeded,
        ),
        (
            Some("future_stop"),
            ToolRunnerTermination::UnknownStopReason("future_stop".into()),
        ),
        (None, ToolRunnerTermination::MissingStopReason),
    ] {
        let server = MockServer::start().await;
        sequence(
            &server,
            vec![response(
                vec![tool_call("a", "calculate", json!({"value":1}))],
                reason,
            )],
        )
        .await;
        let count = Arc::new(AtomicUsize::new(0));
        let result = runner(
            &server,
            counting_registry(count.clone()),
            ToolRunnerOptions::default(),
        )
        .run(request(), None)
        .await;
        assert_eq!(result.termination, expected);
        assert_eq!(count.load(Ordering::SeqCst), 0);
    }
    let server = MockServer::start().await;
    sequence(&server,vec![response(vec![json!({"type":"mcp_tool_use","id":"m","name":"calculate","server_name":"remote","input":{"value":9}})],Some("tool_use"))]).await;
    let count = Arc::new(AtomicUsize::new(0));
    let result = runner(
        &server,
        counting_registry(count.clone()),
        ToolRunnerOptions::default(),
    )
    .run(request(), None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::NoClientToolCalls);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unknown_replay_and_compaction_need_explicit_policies() {
    use threatflux_anthropic_sdk::{
        models::ReplayUnknownPolicy, tool_runner::CompactionReplayPolicy,
    };
    for block in [
        json!({"type":"future_block","opaque":true}),
        json!({"type":"compaction","content":"summary","signature":"signed","encrypted_content":"opaque"}),
    ] {
        for preserve in [false, true] {
            let server = MockServer::start().await;
            let mut responses = vec![response(
                vec![
                    block.clone(),
                    tool_call("a", "calculate", json!({"value":1})),
                ],
                Some("tool_use"),
            )];
            if preserve {
                responses.push(response(vec![], Some("end_turn")));
            }
            sequence(&server, responses).await;
            let count = Arc::new(AtomicUsize::new(0));
            let mut options = ToolRunnerOptions::default();
            if preserve {
                options.unknown_content = ReplayUnknownPolicy::Preserve;
                options.compaction = CompactionReplayPolicy::PreserveHistory;
            }
            let result = runner(&server, counting_registry(count.clone()), options)
                .run(request(), None)
                .await;
            assert_eq!(
                result.termination,
                if preserve {
                    ToolRunnerTermination::EndTurn
                } else {
                    ToolRunnerTermination::UnsupportedReplay
                }
            );
            assert_eq!(count.load(Ordering::SeqCst), usize::from(preserve));
            assert_eq!(
                serde_json::to_value(&result.messages[1].content).unwrap()[0],
                block
            );
        }
    }
}
