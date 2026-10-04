use super::*;

pub(super) fn tool(name: &str) -> Tool {
    Tool::new(
        name,
        "fixture callback",
        json!({"type":"object","properties":{"value":{"type":"integer"}},"required":["value"],"additionalProperties":false}),
    )
}

pub(super) fn request() -> MessageRequest {
    MessageRequest::new()
        .model("future-model")
        .max_tokens(64)
        .add_user_message("calculate")
}

pub(super) fn tool_call(id: &str, name: &str, value: Value) -> Value {
    json!({"type":"tool_use","id":id,"name":name,"input":value})
}

pub(super) fn response(content: Vec<Value>, reason: Option<&str>) -> Value {
    json!({"id":"msg_fixture","type":"message","role":"assistant","model":"future-model","content":content,"stop_reason":reason,"stop_sequence":null,"usage":{"input_tokens":2,"output_tokens":3}})
}

pub(super) fn client(server: &MockServer) -> Client {
    Client::new(
        Config::new("sk-ant-fixture")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    )
}

pub(super) async fn sequence(server: &MockServer, responses: Vec<Value>) {
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

pub(super) fn counting_registry(count: Arc<AtomicUsize>) -> ToolRegistry {
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

pub(super) fn runner(
    server: &MockServer,
    registry: ToolRegistry,
    options: ToolRunnerOptions,
) -> ToolRunner {
    ToolRunner::new(client(server), registry, options).unwrap()
}
