use super::*;
use threatflux_anthropic_sdk::tool_runner::ToolRunnerResult;

#[tokio::test]
async fn a_ready_execution_hook_cannot_start_callbacks_after_cancelling() {
    for parallel in [1, 2] {
        let (result, count) = ready_cancellation_case(parallel, true).await;
        assert_eq!(result.termination, ToolRunnerTermination::Cancelled);
        assert_eq!(count.load(Ordering::SeqCst), 0);
        assert!(result.started_calls.is_empty());
        assert!(result.completed_calls.is_empty());
        assert_eq!(result.messages.len(), 2);
        assert!(result.error.is_none());
    }
}

#[tokio::test]
async fn a_ready_callback_cannot_start_the_next_buffered_callback_after_cancelling() {
    for parallel in [1, 2] {
        let (result, count) = ready_cancellation_case(parallel, false).await;
        assert_eq!(result.termination, ToolRunnerTermination::Cancelled);
        assert_eq!(count.load(Ordering::SeqCst), 1);
        assert_eq!(result.started_calls, ["first"]);
        assert_eq!(result.completed_calls, ["first"]);
        let blocks = &result.messages.last().unwrap().content;
        assert_eq!(blocks.len(), 1);
        let completed = serde_json::to_value(blocks).unwrap();
        assert_eq!(completed[0]["tool_use_id"], "first");
        assert_eq!(completed[0]["content"], "completed external work");
        assert!(result.error.is_none());
    }
}

async fn ready_cancellation_case(
    parallel: usize,
    cancel_hook: bool,
) -> (ToolRunnerResult, Arc<AtomicUsize>) {
    let server = MockServer::start().await;
    sequence(
        &server,
        vec![response(
            vec![
                tool_call("first", "calculate", json!({"value":1})),
                tool_call("second", "calculate", json!({"value":2})),
            ],
            Some("tool_use"),
        )],
    )
    .await;
    let token = ToolRunnerCancellation::new();
    let count = Arc::new(AtomicUsize::new(0));
    let registry = ready_registry(token.clone(), count.clone(), cancel_hook);
    let mut options = ToolRunnerOptions::default();
    options.max_parallel_calls = parallel;
    let mut runner = runner(&server, registry, options).with_cancellation(token.clone());
    if cancel_hook {
        runner = runner.with_execution_hook(move |_| {
            token.cancel();
            async { Ok(()) }
        });
    }
    let result = runner.run(request(), None).await;
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
    (result, count)
}

fn ready_registry(
    token: ToolRunnerCancellation,
    count: Arc<AtomicUsize>,
    cancel_hook: bool,
) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |input| {
            let token = token.clone();
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                if !cancel_hook && input.value == 1 {
                    token.cancel();
                }
                Ok(ToolResultContent::Text("completed external work".into()))
            }
        })
        .unwrap();
    registry
}
