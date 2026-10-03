use serde_json::json;
use threatflux_anthropic_sdk::config::models::{self, Feature, Support};
use threatflux_anthropic_sdk::{MessageBuilder, Model, OutputEffort, ThinkingConfig, ToolChoice};

#[test]
fn current_catalog_is_conservative_and_future_ids_remain_usable() {
    for id in [
        models::OPUS_5_5,
        models::SONNET_5_5,
        models::FABLE_5_1,
        models::MYTHOS_5_1,
    ] {
        assert!(models::is_valid_model(id));
        assert_eq!(
            models::support(id, Feature::AdaptiveThinking),
            Support::Supported
        );
        assert_eq!(
            models::support(id, Feature::ForcedToolChoice),
            Support::Unsupported
        );
        assert!(MessageBuilder::new()
            .model(id)
            .user("hello")
            .build_validated()
            .is_ok());
        assert!(MessageBuilder::new()
            .model(id)
            .user("hello")
            .require_tool_use()
            .build_validated()
            .is_err());
        assert!(MessageBuilder::new()
            .model(id)
            .user("hello")
            .thinking(1024)
            .build_validated()
            .is_err());
        assert!(MessageBuilder::new()
            .model(id)
            .user("hello")
            .assistant("prefill")
            .build_validated()
            .is_err());
        assert!(MessageBuilder::new()
            .model(id)
            .user("hello")
            .temperature(0.5)
            .build_validated()
            .is_err());
    }
    let future = "claude-opus-99-custom";
    assert!(!models::is_valid_model(future));
    assert_eq!(
        models::support(future, Feature::AdaptiveThinking),
        Support::Unknown
    );
    assert!(MessageBuilder::new()
        .model(future)
        .user("hello")
        .require_tool_use()
        .thinking(1024)
        .build_validated()
        .is_ok());
    assert_eq!(
        models::support(models::OPUS_5, Feature::ForcedToolChoice),
        Support::Supported
    );
    assert!(MessageBuilder::new()
        .model(models::OPUS_5)
        .adaptive_thinking()
        .require_tool_use()
        .user("hello")
        .build_validated()
        .is_ok());
}

#[test]
fn sonnet_between_tools_checks_its_exact_constraints() {
    for effort in [OutputEffort::Low, OutputEffort::Medium, OutputEffort::High] {
        let request = MessageBuilder::new()
            .model(models::SONNET_5_5)
            .user("hello")
            .between_tools_thinking()
            .effort(effort)
            .build_validated()
            .unwrap();
        assert_eq!(
            serde_json::to_value(request.thinking.unwrap()).unwrap(),
            json!({"type":"between_tools"})
        );
    }
    for effort in [OutputEffort::XHigh, OutputEffort::Max] {
        assert!(MessageBuilder::new()
            .model(models::SONNET_5_5)
            .user("hello")
            .between_tools_thinking()
            .effort(effort)
            .build_validated()
            .is_err());
    }
    let mut thinking = ThinkingConfig::between_tools();
    thinking.display = Some("summarized".into());
    assert!(thinking.validate_between_tools().is_err());
    thinking.display = None;
    thinking.block_binding = Some(json!({"prefix_mismatch_behavior":"drop_block"}));
    assert!(thinking.validate_between_tools().is_err());
    thinking.block_binding = None;
    thinking.extra.insert("future_option".into(), json!(true));
    assert!(thinking.validate_between_tools().is_err());
    assert!(MessageBuilder::new()
        .model(models::OPUS_5_5)
        .user("hello")
        .between_tools_thinking()
        .build_validated()
        .is_err());
}

#[test]
fn model_capability_metadata_is_lossless_and_requires_explicit_support() {
    let raw = json!({"image_input":{"supported":true,"formats":["png"]},
        "tools":{"supported":false},"future":{"nested":{"limit":500}},
        "conditional":{"supported":"sometimes"},"thinking":{"supported":true,"types":["adaptive"]}});
    let model: Model =
        serde_json::from_value(json!({"id":"future-model","type":"model","capabilities":raw}))
            .unwrap();
    assert!(model.supports_vision());
    assert!(!model.supports_tools());
    assert!(!model.has_capability("future"));
    let caps = model.capabilities.as_ref().unwrap();
    assert_eq!(caps.support("future"), Support::Unknown);
    assert_eq!(caps.support("tools"), Support::Unsupported);
    assert_eq!(caps.support("conditional"), Support::Unknown);
    assert_eq!(caps.as_json(), &raw);
    assert_eq!(serde_json::to_value(&model).unwrap()["capabilities"], raw);
    let legacy: Model =
        serde_json::from_value(json!({"id":"legacy","capabilities":["vision","tool_use"]}))
            .unwrap();
    assert!(legacy.supports_vision());
    assert!(legacy.supports_tools());
    assert!(serde_json::from_value::<Model>(json!({"id":"bad","capabilities":42})).is_err());
}

#[test]
fn new_models_accept_none_choice_and_do_not_guess_sampling_defaults() {
    for model in [models::OPUS_5_5, models::SONNET_5_5] {
        assert!(MessageBuilder::new()
            .model(model)
            .user("hello")
            .tool_choice(ToolChoice::None)
            .build_validated()
            .is_ok());
        assert!(MessageBuilder::new()
            .model(model)
            .user("hello")
            .top_k(0)
            .build_validated()
            .is_err());
    }
}
