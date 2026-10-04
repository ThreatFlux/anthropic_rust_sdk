use super::*;

#[tokio::test]
async fn all_calls_are_validated_before_any_callback_runs() {
    for second in [
        tool_call("b", "unregistered", json!({"value":2})),
        tool_call("b", "calculate", json!({"value":"wrong"})),
        tool_call("a", "calculate", json!({"value":2})),
    ] {
        let server = MockServer::start().await;
        sequence(
            &server,
            vec![response(
                vec![tool_call("a", "calculate", json!({"value":1})), second],
                Some("tool_use"),
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
        assert_eq!(result.termination, ToolRunnerTermination::InvalidToolCall);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(result.started_calls.is_empty());
        assert_eq!(result.messages.len(), 2);
    }
}

#[tokio::test]
async fn invalid_registry_budgets_and_known_model_options_fail_before_http() {
    let server = MockServer::start().await;
    let mut options = ToolRunnerOptions::default();
    options.max_turns = 0;
    assert!(ToolRunner::new(client(&server), ToolRegistry::new(), options).is_err());
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), |_| async {
            Ok(ToolResultContent::Text("ok".into()))
        })
        .unwrap();
    assert!(registry
        .register_typed::<Input, _, _>(tool("calculate"), |_| async {
            Ok(ToolResultContent::Text("ok".into()))
        })
        .is_err());
    let count = Arc::new(AtomicUsize::new(0));
    let mut input = request().model("claude-sonnet-5-5");
    input.tool_choice = Some(ToolChoice::Any);
    let result = runner(
        &server,
        counting_registry(count),
        ToolRunnerOptions::default(),
    )
    .run(input, None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::InvalidRequest);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn empty_registry_does_not_inject_tool_only_fields() {
    let server = MockServer::start().await;
    sequence(&server, vec![response(vec![], Some("end_turn"))]).await;
    let result = runner(&server, ToolRegistry::new(), ToolRunnerOptions::default())
        .run(request(), None)
        .await;
    assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
    let requests = server.received_requests().await.unwrap();
    let wire: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert!(wire.get("tools").is_none());
    assert!(wire.get("tool_choice").is_none());
}

#[tokio::test]
async fn explicit_custom_type_is_registered_and_executed_as_a_client_tool() {
    let server = MockServer::start().await;
    sequence(
        &server,
        vec![
            response(
                vec![tool_call("custom_1", "calculate", json!({"value":4}))],
                Some("tool_use"),
            ),
            response(vec![], Some("end_turn")),
        ],
    )
    .await;
    let count = Arc::new(AtomicUsize::new(0));
    let callback_count = count.clone();
    let mut definition = tool("calculate");
    definition.tool_type = Some("custom".into());
    assert!(definition.is_client());
    assert!(tool("implicit").is_client());
    assert!(!Tool::web_search().is_client());
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(definition, move |input| {
            let count = callback_count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolResultContent::Text(input.value.to_string()))
            }
        })
        .unwrap();
    let result = runner(&server, registry, ToolRunnerOptions::default())
        .run(request(), None)
        .await;
    assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(result.started_calls, vec!["custom_1"]);
    let requests = server.received_requests().await.unwrap();
    let wire: Value = serde_json::from_slice(&requests[0].body).unwrap();
    assert_eq!(wire["tools"][0]["type"], "custom");
}

#[tokio::test]
async fn forced_tool_choice_rejects_a_different_registered_and_offered_tool() {
    for parallel_control in [None, Some(true)] {
        let server = MockServer::start().await;
        sequence(
            &server,
            vec![response(
                vec![
                    tool_call("valid_tool", "a", json!({"value":1})),
                    tool_call("wrong_tool", "b", json!({"value":1})),
                ],
                Some("tool_use"),
            )],
        )
        .await;
        let count = Arc::new(AtomicUsize::new(0));
        let mut registry = ToolRegistry::new();
        for name in ["a", "b"] {
            let callback_count = count.clone();
            registry
                .register_typed::<Input, _, _>(tool(name), move |_| {
                    let count = callback_count.clone();
                    async move {
                        count.fetch_add(1, Ordering::SeqCst);
                        Ok(ToolResultContent::Text("never".into()))
                    }
                })
                .unwrap();
        }
        let mut input = request();
        let choice = ToolChoice::Tool { name: "a".into() };
        input.tool_choice = Some(match parallel_control {
            Some(flag) => choice.with_disable_parallel_tool_use(flag).unwrap(),
            None => choice,
        });
        let result = runner(&server, registry, ToolRunnerOptions::default())
            .run(input, None)
            .await;
        assert_eq!(result.termination, ToolRunnerTermination::InvalidToolCall);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(result.started_calls.is_empty());
        assert_eq!(result.responses.len(), 1);
        let requests = server.received_requests().await.unwrap();
        let wire: Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(wire["tool_choice"]["name"], "a");
        assert_eq!(wire["tools"].as_array().unwrap().len(), 2);
    }
}

#[tokio::test]
async fn explicit_none_prevents_callbacks() {
    let server = MockServer::start().await;
    sequence(
        &server,
        vec![response(
            vec![tool_call("a", "calculate", json!({"value":1}))],
            Some("tool_use"),
        )],
    )
    .await;
    let count = Arc::new(AtomicUsize::new(0));
    let mut input = request();
    input.tool_choice = Some(ToolChoice::None);
    let result = runner(
        &server,
        counting_registry(count.clone()),
        ToolRunnerOptions::default(),
    )
    .run(input, None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::InvalidToolCall);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}
