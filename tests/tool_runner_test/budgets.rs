use super::*;

#[tokio::test]
async fn turn_call_and_result_budgets_bound_work() {
    for budget in ["turns", "calls", "bytes"] {
        let server = MockServer::start().await;
        let content = if budget == "calls" {
            vec![
                tool_call("a", "calculate", json!({"value":1})),
                tool_call("b", "calculate", json!({"value":2})),
            ]
        } else {
            vec![tool_call("a", "calculate", json!({"value":1000}))]
        };
        sequence(&server, vec![response(content, Some("tool_use"))]).await;
        let count = Arc::new(AtomicUsize::new(0));
        let mut options = ToolRunnerOptions::default();
        let expected = match budget {
            "turns" => {
                options.max_turns = 1;
                ToolRunnerTermination::TurnLimit
            }
            "calls" => {
                options.max_tool_calls = 1;
                ToolRunnerTermination::CallLimit
            }
            _ => {
                options.max_result_bytes = 1;
                ToolRunnerTermination::ResultLimit
            }
        };
        let result = runner(&server, counting_registry(count.clone()), options)
            .run(request(), None)
            .await;
        assert_eq!(result.termination, expected);
        assert_eq!(count.load(Ordering::SeqCst), usize::from(budget != "calls"));
    }
}

#[tokio::test]
async fn callback_and_whole_run_deadlines_drop_pending_work() {
    for whole_run in [false, true] {
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
        let counter = count.clone();
        let mut registry = ToolRegistry::new();
        registry
            .register_typed::<Input, _, _>(tool("calculate"), move |_| {
                let count = counter.clone();
                async move {
                    count.fetch_add(1, Ordering::SeqCst);
                    futures::future::pending::<()>().await;
                    Ok(ToolResultContent::Text("unreachable".into()))
                }
            })
            .unwrap();
        let mut options = ToolRunnerOptions::default();
        if whole_run {
            options.overall_timeout = Duration::from_millis(500);
        } else {
            options.call_timeout = Duration::from_millis(5);
        }
        let result = runner(&server, registry, options)
            .run(request(), None)
            .await;
        assert_eq!(
            result.termination,
            if whole_run {
                ToolRunnerTermination::OverallTimeout
            } else {
                ToolRunnerTermination::CallbackFailed
            }
        );
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(result.started_calls, vec!["a"]);
        assert!(result.completed_calls.is_empty());
    }
}

#[tokio::test]
async fn explicit_cancellation_stops_running_callback_and_future_calls() {
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
    let cancellation = ToolRunnerCancellation::new();
    let token = cancellation.clone();
    let count = Arc::new(AtomicUsize::new(0));
    let counter = count.clone();
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |_| {
            let token = token.clone();
            let count = counter.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                token.cancel();
                futures::future::pending::<()>().await;
                Ok(ToolResultContent::Text("never".into()))
            }
        })
        .unwrap();
    let result = runner(&server, registry, ToolRunnerOptions::default())
        .with_cancellation(cancellation)
        .run(request(), None)
        .await;
    assert_eq!(result.termination, ToolRunnerTermination::Cancelled);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    assert_eq!(result.started_calls, vec!["a"]);
    let server = MockServer::start().await;
    let token = ToolRunnerCancellation::new();
    token.cancel();
    let result = runner(&server, ToolRegistry::new(), ToolRunnerOptions::default())
        .with_cancellation(token)
        .run(request(), None)
        .await;
    assert_eq!(result.termination, ToolRunnerTermination::Cancelled);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_and_deadline_preserve_results_completed_before_pending_call() {
    for cancel in [false, true] {
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
        let token = ToolRunnerCancellation::new();
        let registry = pending_after_first_registry(token.clone(), cancel);
        let mut options = ToolRunnerOptions::default();
        options.overall_timeout = Duration::from_millis(500);
        let result = runner(&server, registry, options)
            .with_cancellation(token)
            .run(request(), None)
            .await;
        assert_eq!(
            result.termination,
            if cancel {
                ToolRunnerTermination::Cancelled
            } else {
                ToolRunnerTermination::OverallTimeout
            }
        );
        assert_eq!(result.started_calls, vec!["a", "b"]);
        assert_eq!(result.completed_calls, vec!["a"]);
        let completed: Value =
            serde_json::to_value(&result.messages.last().unwrap().content).unwrap();
        assert_eq!(completed[0]["tool_use_id"], "a");
        assert_eq!(completed[0]["content"], "completed external work");
    }
}

fn pending_after_first_registry(token: ToolRunnerCancellation, cancel: bool) -> ToolRegistry {
    let callback_token = token;
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |input| {
            let token = callback_token.clone();
            async move {
                if input.value == 1 {
                    return Ok(ToolResultContent::Text("completed external work".into()));
                }
                if cancel {
                    token.cancel();
                }
                futures::future::pending::<()>().await;
                Ok(ToolResultContent::Text("unreachable".into()))
            }
        })
        .unwrap();
    registry
}
