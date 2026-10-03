//! Counting projection uses the official countable field allowlist (SDK 18f25547).

use serde_json::{json, Value};
use threatflux_anthropic_sdk::{
    models::{
        message::{
            MessageRequest, OutputConfig, OutputEffort, OutputFormat, PromptOptions, SystemBlock,
            ThinkingConfig, TokenCountRequest,
        },
        CacheControl, Tool, ToolChoice,
    },
    types::RequestOptions,
    Client, Config,
};
use wiremock::{
    matchers::{header, method, path},
    Mock, MockServer, ResponseTemplate,
};

fn configured_prompt() -> MessageRequest {
    let thinking: ThinkingConfig = serde_json::from_value(json!({"type":"adaptive","block_binding":{"prefix_mismatch_behavior":"error","future":{"nested":1}},"future_setting":true})).unwrap();
    MessageRequest::new()
        .model("future-model")
        .add_user_message("Count this")
        .max_tokens(321)
        .stream(true)
        .temperature(0.2)
        .system_blocks(vec![SystemBlock::cached("cached system")])
        .add_tool(Tool::new("lookup", "Lookup", json!({"type":"object"})))
        .tool_choice(ToolChoice::Auto)
        .thinking_config(thinking)
        .output_config(
            OutputConfig::new()
                .with_effort(OutputEffort::High)
                .with_format(OutputFormat::json_schema(json!({"type":"object"}))),
        )
        .cache_control(CacheControl::ephemeral())
        .user_profile_id("profile_123")
        .diagnostics(json!({"controls":true}))
        .service_tier("standard_only")
}

#[test]
fn projection_preserves_all_prompt_fields_and_excludes_generation_controls() {
    let request = configured_prompt();
    let count = TokenCountRequest::from_message(&request).unwrap();
    let count_json = serde_json::to_value(&count).unwrap();
    let request_json = serde_json::to_value(&request).unwrap();
    for key in [
        "model",
        "messages",
        "system",
        "tools",
        "thinking",
        "tool_choice",
        "output_config",
        "cache_control",
    ] {
        assert_eq!(count_json[key], request_json[key], "{key}");
    }
    assert_eq!(count_json.as_object().unwrap().len(), 8);
    for key in [
        "max_tokens",
        "stream",
        "temperature",
        "service_tier",
        "diagnostics",
        "user_profile_id",
        "container",
    ] {
        assert!(count_json.get(key).is_none(), "{key}");
    }
    assert_eq!(count.user_profile_id.as_deref(), Some("profile_123"));
    assert_eq!(count_json["thinking"]["future_setting"], true);
    assert_eq!(
        count_json["thinking"]["block_binding"]["future"],
        json!({"nested":1})
    );

    let mut shared = MessageRequest::new();
    PromptOptions::from_message(&request).apply_to_message(&mut shared);
    assert_eq!(shared.thinking, request.thinking);
    assert_eq!(
        TokenCountRequest::from_message(&shared).unwrap().thinking,
        request.thinking
    );
}

#[test]
fn unsupported_prompt_routing_never_claims_counting_parity() {
    let plain = MessageRequest::new().add_user_message("hello");
    for request in [
        plain.clone().container(json!({"id":"c"})),
        plain.clone().context_management(json!({"edits":[]})),
        plain.clone().mcp_servers(vec![]),
        plain.clone().add_fallback("other-model"),
        plain.fallback_credit_token("opaque"),
    ] {
        assert!(TokenCountRequest::from_message(&request).is_err());
    }
    let request =
        MessageRequest::new().output_format(OutputFormat::json_schema(json!({"type":"object"})));
    assert_eq!(
        serde_json::to_value(TokenCountRequest::from_message(&request).unwrap()).unwrap()
            ["output_config"]["format"]["schema"],
        json!({"type":"object"})
    );
    assert!(
        TokenCountRequest::from_message(&request.output_json_schema(json!({"type":"string"})))
            .is_err()
    );
}

#[test]
fn count_setters_stay_flat_and_between_tools_has_exact_schema() {
    let count = TokenCountRequest::new()
        .add_user_message("hello")
        .thinking(ThinkingConfig::between_tools())
        .tool_choice(ToolChoice::Auto)
        .cache_control(CacheControl::ephemeral())
        .output_config(OutputConfig::new().with_effort(OutputEffort::Low));
    let json = serde_json::to_value(count).unwrap();
    assert_eq!(json["thinking"], json!({"type":"between_tools"}));
    assert_eq!(json["tool_choice"], json!({"type":"auto"}));
    assert!(json.get("prompt_options").is_none());
    assert!(ThinkingConfig::between_tools()
        .validate_between_tools()
        .is_ok());
    assert!(ThinkingConfig::between_tools()
        .with_display("summarized")
        .validate_between_tools()
        .is_err());
    assert!(ThinkingConfig::between_tools()
        .with_block_binding(json!({}))
        .validate_between_tools()
        .is_err());
}

async fn mount_prompt_endpoints(server: &MockServer) {
    Mock::given(method("POST")).and(path("/v1/messages"))
        .and(header("anthropic-user-profile-id","profile_test")).and(header("anthropic-workspace-id","workspace_test")).and(header("anthropic-beta","thinking-binding-controls-2026-08-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id":"msg_test","type":"message","role":"assistant","model":"future-model","content":[],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":10,"output_tokens":1}}))).expect(1).mount(server).await;
    Mock::given(method("POST"))
        .and(path("/v1/messages/count_tokens"))
        .and(header("anthropic-user-profile-id", "profile_test"))
        .and(header("anthropic-workspace-id", "workspace_test"))
        .and(header(
            "anthropic-beta",
            "thinking-binding-controls-2026-08-01",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"input_tokens":10})))
        .expect(1)
        .mount(server)
        .await;
}

fn attribution_prompt() -> MessageRequest {
    MessageRequest::new()
        .model("future-model")
        .add_user_message("hi")
        .system_cached("system")
        .add_tool(Tool::new("lookup", "Lookup", json!({"type":"object"})))
        .tool_choice(ToolChoice::Auto)
        .thinking_config(ThinkingConfig::adaptive().with_block_binding(json!({})))
        .output_json_schema(json!({"type":"object"}))
        .auto_cache()
        .user_profile_id("profile_test")
}

#[tokio::test]
async fn messages_and_counting_send_identical_prompt_fields_and_attribution_headers() {
    let server = MockServer::start().await;
    let client = Client::new(
        Config::new("sk-ant-test-key")
            .unwrap()
            .with_base_url(server.uri().parse().unwrap()),
    );
    mount_prompt_endpoints(&server).await;
    let request = attribution_prompt();
    let options = RequestOptions::new()
        .with_header("anthropic-workspace-id", "workspace_test")
        .with_beta_feature("thinking-binding-controls-2026-08-01");
    client
        .messages()
        .count_tokens(
            TokenCountRequest::from_message(&request).unwrap(),
            Some(options.clone()),
        )
        .await
        .unwrap();
    client
        .messages()
        .create(request, Some(options))
        .await
        .unwrap();
    let requests = server.received_requests().await.unwrap();
    let count: Value = serde_json::from_slice(&requests[0].body).unwrap();
    let message: Value = serde_json::from_slice(&requests[1].body).unwrap();
    for key in [
        "model",
        "messages",
        "system",
        "tools",
        "thinking",
        "tool_choice",
        "output_config",
        "cache_control",
    ] {
        assert_eq!(count[key], message[key], "{key}");
    }
    assert!(count.get("user_profile_id").is_none());
    assert!(message.get("user_profile_id").is_none());
}
