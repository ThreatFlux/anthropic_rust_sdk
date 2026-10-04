use super::*;
use controlled_delivery::*;

#[tokio::test]
async fn incomplete_stream_does_not_execute_partial_tool_input() {
    let server = MockServer::start().await;
    let sse=format!("event: message_start\ndata: {}\n\nevent: content_block_start\ndata: {}\n\nevent: content_block_delta\ndata: {}\n\nevent: content_block_stop\ndata: {}\n\nevent: message_delta\ndata: {}\n\n",
        json!({"type":"message_start","message":response(vec![],None)}),json!({"type":"content_block_start","index":0,"content_block":tool_call("a","calculate",json!({}))}),json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"value\":1}"}}),json!({"type":"content_block_stop","index":0}),json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":3}}));
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(sse, "text/event-stream"))
        .expect(1)
        .mount(&server)
        .await;
    let count = Arc::new(AtomicUsize::new(0));
    let result = runner(
        &server,
        counting_registry(count.clone()),
        ToolRunnerOptions::default(),
    )
    .run_streaming(request(), None)
    .await;
    assert_eq!(result.termination, ToolRunnerTermination::TransportFailed);
    assert_eq!(count.load(Ordering::SeqCst), 0);
    assert!(result.started_calls.is_empty());
}

#[tokio::test]
async fn streaming_callback_waits_for_terminal_frame_under_controlled_delivery() {
    let delivery = controlled_delivery().await;
    let count = Arc::new(AtomicUsize::new(0));
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(format!("http://{}", delivery.address).parse().unwrap()),
    );
    let registry = signalling_registry(delivery.callback_signal, count.clone());
    let runner = ToolRunner::new(client, registry, ToolRunnerOptions::default()).unwrap();
    let run = tokio::spawn(async move { runner.run_streaming(request(), None).await });
    tokio::time::timeout(Duration::from_secs(5), delivery.prefix_ready)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
    delivery.release.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    delivery.server.await.unwrap();
}
