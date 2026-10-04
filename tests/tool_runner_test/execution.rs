use super::*;

#[tokio::test]
async fn callback_failures_return_partial_state_or_fixed_sanitized_error() {
    for sanitize in [false, true] {
        let server = MockServer::start().await;
        let mut responses = vec![response(
            vec![tool_call("a", "calculate", json!({"value":1}))],
            Some("tool_use"),
        )];
        if sanitize {
            responses.push(response(
                vec![json!({"type":"text","text":"handled"})],
                Some("end_turn"),
            ));
        }
        sequence(&server, responses).await;
        let mut registry = ToolRegistry::new();
        registry
            .register_typed::<Input, _, _>(tool("calculate"), |_| async {
                Err(AnthropicError::invalid_input("private callback detail"))
            })
            .unwrap();
        let mut options = ToolRunnerOptions::default();
        if sanitize {
            options.callback_errors = CallbackErrorPolicy::ReturnSanitizedError;
        }
        let result = runner(&server, registry, options)
            .run(request(), None)
            .await;
        assert_eq!(result.started_calls, vec!["a"]);
        if sanitize {
            assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
            let requests = server.received_requests().await.unwrap();
            let wire: Value = serde_json::from_slice(&requests[1].body).unwrap();
            assert_eq!(wire["messages"][2]["content"][0]["is_error"], true);
            assert_eq!(
                wire["messages"][2]["content"][0]["content"],
                "Tool execution failed"
            );
            assert!(!wire.to_string().contains("private callback detail"));
        } else {
            assert_eq!(result.termination, ToolRunnerTermination::CallbackFailed);
            assert!(result.error.is_some());
            assert_eq!(result.messages.len(), 2);
            assert!(result.completed_calls.is_empty());
        }
    }
}

#[tokio::test]
async fn hooks_approve_all_calls_before_side_effects_start() {
    let server = MockServer::start().await;
    sequence(
        &server,
        vec![response(
            vec![
                tool_call("a", "calculate", json!({"value":1})),
                tool_call("b", "calculate", json!({"value":2})),
            ],
            Some("tool_use"),
        )],
    )
    .await;
    let count = Arc::new(AtomicUsize::new(0));
    let approvals = Arc::new(AtomicUsize::new(0));
    let approved = approvals.clone();
    let result = runner(
        &server,
        counting_registry(count.clone()),
        ToolRunnerOptions::default(),
    )
    .with_execution_hook(move |call| {
        let approved = approved.clone();
        async move {
            approved.fetch_add(1, Ordering::SeqCst);
            if call.id == "b" {
                Err(AnthropicError::invalid_input("execution denied"))
            } else {
                Ok(())
            }
        }
    })
    .run(request(), None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::CallbackFailed);
    assert_eq!(approvals.load(Ordering::SeqCst), 2);
    assert_eq!(count.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn bounded_parallel_execution_preserves_wire_order_and_parallel_flag() {
    for disable in [false, true] {
        let server = MockServer::start().await;
        sequence(
            &server,
            vec![
                response(
                    vec![
                        tool_call("a", "calculate", json!({"value":1})),
                        tool_call("b", "calculate", json!({"value":2})),
                        tool_call("c", "calculate", json!({"value":3})),
                    ],
                    Some("tool_use"),
                ),
                response(vec![], Some("end_turn")),
            ],
        )
        .await;
        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let registry = parallel_registry(active, maximum.clone());
        let mut options = ToolRunnerOptions::default();
        options.max_parallel_calls = 2;
        let mut input = request();
        input.tool_choice = Some(
            ToolChoice::Auto
                .with_disable_parallel_tool_use(disable)
                .unwrap(),
        );
        let result = runner(&server, registry, options).run(input, None).await;
        assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
        assert_eq!(maximum.load(Ordering::SeqCst), if disable { 1 } else { 2 });
        let requests = server.received_requests().await.unwrap();
        let wire: Value = serde_json::from_slice(&requests[1].body).unwrap();
        let ids = wire["messages"][2]["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|block| block["tool_use_id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }
}

#[tokio::test]
async fn json_callback_values_are_encoded_as_text_and_explicit_none_is_honored() {
    let server = MockServer::start().await;
    sequence(
        &server,
        vec![
            response(
                vec![tool_call("a", "calculate", json!({"value":1}))],
                Some("tool_use"),
            ),
            response(vec![], Some("end_turn")),
        ],
    )
    .await;
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), |_| async {
            Ok(ToolResultContent::Json(json!({"structured":true})))
        })
        .unwrap();
    let result = runner(&server, registry, ToolRunnerOptions::default())
        .run(request(), None)
        .await;
    assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
    let requests = server.received_requests().await.unwrap();
    let wire: Value = serde_json::from_slice(&requests[1].body).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(
            wire["messages"][2]["content"][0]["content"]
                .as_str()
                .unwrap()
        )
        .unwrap(),
        json!({"structured":true})
    );
}

fn parallel_registry(active: Arc<AtomicUsize>, maximum: Arc<AtomicUsize>) -> ToolRegistry {
    let a = active;
    let m = maximum;
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |input| {
            let a = a.clone();
            let m = m.clone();
            async move {
                let current = a.fetch_add(1, Ordering::SeqCst) + 1;
                m.fetch_max(current, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(if input.value == 1 { 15 } else { 2 }))
                    .await;
                a.fetch_sub(1, Ordering::SeqCst);
                Ok(ToolResultContent::Text(input.value.to_string()))
            }
        })
        .unwrap();
    registry
}
