//! Tool-runner tests use HTTP fixtures and explicit fake callbacks only.

use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use threatflux_anthropic_sdk::{
    models::{MessageRequest, Tool, ToolChoice, ToolResultContent},
    tool_runner::{
        CallbackErrorPolicy, ToolRegistry, ToolRunner, ToolRunnerCancellation, ToolRunnerOptions,
        ToolRunnerTermination,
    },
    AnthropicError, Client, Config,
};
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, Request, ResponseTemplate,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    value: u64,
}
fn tool(name: &str) -> Tool {
    Tool::new(
        name,
        "fixture callback",
        json!({"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}),
    )
}
fn request() -> MessageRequest {
    MessageRequest::new()
        .model("future-model")
        .max_tokens(64)
        .add_user_message("calculate")
}
fn tool_call(id: &str, name: &str, value: Value) -> Value {
    json!({"type":"tool_use","id":id,"name":name,"input":value})
}
fn response(content: Vec<Value>, reason: Option<&str>) -> Value {
    json!({"id":"msg_fixture","type":"message","role":"assistant","model":"future-model","content":content,"stop_reason":reason,"stop_sequence":null,"usage":{"input_tokens":2,"output_tokens":3}})
}
fn client(server: &MockServer) -> Client {
    Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    )
}
async fn sequence(server: &MockServer, responses: Vec<Value>) {
    let cursor = Arc::new(AtomicUsize::new(0));
    let expected = responses.len() as u64;
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(move |_: &Request| {
            let index = cursor.fetch_add(1, Ordering::SeqCst);
            responses.get(index).map_or_else(
                || ResponseTemplate::new(500),
                |body| ResponseTemplate::new(200).set_body_json(body),
            )
        })
        .expect(expected)
        .mount(server)
        .await;
}
fn counting_registry(count: Arc<AtomicUsize>) -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |input| {
            let count = count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                Ok(ToolResultContent::Text((input.value * 2).to_string()))
            }
        })
        .unwrap();
    registry
}
fn runner(server: &MockServer, registry: ToolRegistry, options: ToolRunnerOptions) -> ToolRunner {
    ToolRunner::new(client(server), registry, options).unwrap()
}

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
        let a = active.clone();
        let m = maximum.clone();
        let mut registry = ToolRegistry::new();
        registry
            .register_typed::<Input, _, _>(tool("calculate"), move |input| {
                let a = a.clone();
                let m = m.clone();
                async move {
                    let current = a.fetch_add(1, Ordering::SeqCst) + 1;
                    m.fetch_max(current, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(if input.value == 1 {
                        15
                    } else {
                        2
                    }))
                    .await;
                    a.fetch_sub(1, Ordering::SeqCst);
                    Ok(ToolResultContent::Text(input.value.to_string()))
                }
            })
            .unwrap();
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
        let callback_token = token.clone();
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

#[tokio::test]
async fn streaming_callback_waits_for_terminal_frame_under_controlled_delivery() {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
        sync::oneshot,
    };
    async fn read_request(socket: &mut TcpStream) {
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0u8; 2048];
            let size = socket.read(&mut chunk).await.unwrap();
            assert_ne!(size, 0);
            bytes.extend_from_slice(&chunk[..size]);
            if let Some(end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..end]);
                let length = headers
                    .lines()
                    .find_map(|line| {
                        line.split_once(':')
                            .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
                            .map(|(_, value)| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or_default();
                if bytes.len() >= end + 4 + length {
                    return;
                }
            }
        }
    }
    async fn chunk(socket: &mut TcpStream, body: &str) {
        socket
            .write_all(format!("{:x}\r\n", body.len()).as_bytes())
            .await
            .unwrap();
        socket.write_all(body.as_bytes()).await.unwrap();
        socket.write_all(b"\r\n").await.unwrap();
        socket.flush().await.unwrap();
    }
    fn event(name: &str, body: Value) -> String {
        format!("event: {name}\ndata: {body}\n\n")
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let (prefix_sent, prefix_ready) = oneshot::channel();
    let (release, terminal_allowed) = oneshot::channel();
    let (callback_fired, callback_observed) = oneshot::channel();
    let callback_signal = Arc::new(std::sync::Mutex::new(Some(callback_fired)));
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        let prefix = event(
            "message_start",
            json!({"type":"message_start","message":response(vec![],None)}),
        ) + &event(
            "content_block_start",
            json!({"type":"content_block_start","index":0,"content_block":tool_call("a","calculate",json!({}))}),
        ) + &event(
            "content_block_delta",
            json!({"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"value\":1}"}}),
        ) + &event(
            "content_block_stop",
            json!({"type":"content_block_stop","index":0}),
        ) + &event(
            "message_delta",
            json!({"type":"message_delta","delta":{"stop_reason":"tool_use"},"usage":{"output_tokens":3}}),
        );
        chunk(&mut socket, &prefix).await;
        prefix_sent.send(()).unwrap();
        terminal_allowed.await.unwrap();
        chunk(
            &mut socket,
            &event("message_stop", json!({"type":"message_stop"})),
        )
        .await;
        // Do not finish the HTTP body until the callback has run. A collector
        // waiting for HTTP EOF instead of message_stop deadlocks this fixture.
        tokio::time::timeout(Duration::from_secs(5), callback_observed)
            .await
            .unwrap()
            .unwrap();
        drop(socket);
        let (mut socket, _) = listener.accept().await.unwrap();
        read_request(&mut socket).await;
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await.unwrap();
        let final_turn = event(
            "message_start",
            json!({"type":"message_start","message":response(vec![],None)}),
        ) + &event(
            "message_delta",
            json!({"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":3}}),
        ) + &event("message_stop", json!({"type":"message_stop"}));
        chunk(&mut socket, &final_turn).await;
        // The terminal SSE collector may already have closed the transport.
        let _ = socket.write_all(b"0\r\n\r\n").await;
    });
    let count = Arc::new(AtomicUsize::new(0));
    let client = Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(format!("http://{address}").parse().unwrap()),
    );
    let mut registry = ToolRegistry::new();
    let callback_count = count.clone();
    registry
        .register_typed::<Input, _, _>(tool("calculate"), move |input| {
            let signal = callback_signal.clone();
            let count = callback_count.clone();
            async move {
                count.fetch_add(1, Ordering::SeqCst);
                signal.lock().unwrap().take().unwrap().send(()).unwrap();
                Ok(ToolResultContent::Text(input.value.to_string()))
            }
        })
        .unwrap();
    let runner = ToolRunner::new(client, registry, ToolRunnerOptions::default()).unwrap();
    let run = tokio::spawn(async move { runner.run_streaming(request(), None).await });
    tokio::time::timeout(Duration::from_secs(5), prefix_ready)
        .await
        .unwrap()
        .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    assert_eq!(count.load(Ordering::SeqCst), 0);
    release.send(()).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(5), run)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(result.termination, ToolRunnerTermination::EndTurn);
    assert_eq!(count.load(Ordering::SeqCst), 1);
    server.await.unwrap();
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
