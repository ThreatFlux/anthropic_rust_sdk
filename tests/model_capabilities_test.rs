use serde_json::json;
use threatflux_anthropic_sdk::builders::{BatchBuilder, ParameterBuilder};
use threatflux_anthropic_sdk::config::models::{self, Feature, Support};
use threatflux_anthropic_sdk::{
    Config, MessageBuilder, MessageRequest, Model, OutputEffort, ThinkingConfig, ToolChoice,
    DEFAULT_MODEL,
};

#[test]
fn current_catalog_is_conservative() {
    for id in [
        models::OPUS_5_5,
        models::SONNET_5_5,
        models::FABLE_5_2,
        models::MYTHOS_5_2,
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
}

#[test]
fn future_ids_remain_usable_and_older_forced_choices_remain_supported() {
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

#[test]
fn default_model_is_haiku_5_5() {
    assert_eq!(DEFAULT_MODEL, "claude-haiku-5-5");
    assert_eq!(DEFAULT_MODEL, models::HAIKU_5_5);
    assert_eq!(MessageRequest::new().model, "claude-haiku-5-5");
    assert_eq!(MessageBuilder::new().build().model, "claude-haiku-5-5");
    assert_eq!(
        Config::new("sk-ant-test").unwrap().default_model,
        "claude-haiku-5-5"
    );
}

#[test]
fn current_model_ids_are_exact() {
    assert_eq!(models::HAIKU_5_5, "claude-haiku-5-5");
    assert_eq!(models::SONNET_5_5, "claude-sonnet-5-5");
    assert_eq!(models::OPUS_5_5, "claude-opus-5-5");
    assert_eq!(models::FABLE_5_1, "claude-fable-5-1");
    assert_eq!(models::FABLE_5_2, "claude-fable-5-2");
    assert_eq!(models::MYTHOS_5_2, "claude-mythos-5-2");
    for id in [
        models::HAIKU_5_5,
        models::SONNET_5_5,
        models::OPUS_5_5,
        models::FABLE_5_1,
        models::FABLE_5_2,
        models::MYTHOS_5_2,
        // Older IDs stay in the catalog for compatibility.
        models::SONNET_4_6,
        models::HAIKU_4_5,
    ] {
        assert!(models::is_valid_model(id), "{id} should be in the catalog");
    }
}

#[test]
fn current_models_document_limits_and_capabilities() {
    for id in [
        models::HAIKU_5_5,
        models::SONNET_5_5,
        models::OPUS_5_5,
        models::FABLE_5_1,
        models::FABLE_5_2,
    ] {
        assert_eq!(models::context_window(id), Some(1_000_000), "{id}");
        assert_eq!(models::max_output_tokens(id), Some(128_000), "{id}");
        assert_eq!(models::support(id, Feature::Context1M), Support::Supported);
        assert_eq!(
            models::support(id, Feature::AdaptiveThinking),
            Support::Supported
        );
        assert_eq!(models::support(id, Feature::Effort), Support::Supported);
        assert_eq!(
            models::support(id, Feature::SamplingParameters),
            Support::Unsupported
        );
        assert!(!models::supports_sampling_parameters(id));
        assert_eq!(models::max_thinking_tokens(id), None);
    }
    assert_eq!(models::context_window(models::HAIKU_4_5), Some(200_000));
    assert_eq!(models::max_output_tokens(models::HAIKU_4_5), Some(64_000));
    assert_eq!(models::context_window(models::SONNET_4_5), Some(200_000));
    assert_eq!(models::context_window("claude-future-9"), None);
    assert_eq!(models::max_output_tokens("claude-future-9"), None);
    assert!(models::supports_sampling_parameters(models::SONNET_4_6));
    assert!(models::supports_sampling_parameters("claude-future-9"));
    assert_eq!(
        models::support("claude-future-9", Feature::SamplingParameters),
        Support::Unknown
    );
    // Haiku 5.5 still accepts forced tool use; Fable 5.2 does not.
    assert_eq!(
        models::support(models::HAIKU_5_5, Feature::ForcedToolChoice),
        Support::Supported
    );
    assert_eq!(
        models::support(models::FABLE_5_2, Feature::ForcedToolChoice),
        Support::Unsupported
    );
}

#[test]
fn haiku_5_5_rejects_sampling_and_legacy_thinking_but_keeps_forced_tools() {
    let base = || MessageBuilder::new().model(models::HAIKU_5_5).user("hello");
    assert!(base().build_validated().is_ok());
    assert!(base().adaptive_thinking().build_validated().is_ok());
    assert!(base().require_tool_use().build_validated().is_ok());
    assert!(base().temperature(0.5).build_validated().is_err());
    assert!(base().top_p(0.9).build_validated().is_err());
    assert!(base().top_k(5).build_validated().is_err());
    assert!(base().thinking(1024).build_validated().is_err());
    assert!(base().assistant("prefill").build_validated().is_err());
    let disabled = |effort| {
        let mut request = base().effort(effort).build();
        request.thinking = Some(ThinkingConfig::disabled());
        threatflux_anthropic_sdk::builders::message_builder::validate_model_options(&request)
    };
    assert!(disabled(OutputEffort::High).is_ok());
    assert!(disabled(OutputEffort::XHigh).is_err());
    assert!(disabled(OutputEffort::Max).is_err());
}

#[test]
fn presets_omit_sampling_for_models_that_reject_it() {
    // The default model never receives preset sampling values.
    for builder in [
        MessageBuilder::new().creative(),
        MessageBuilder::new().analytical(),
        MessageBuilder::new().conversational(),
        MessageBuilder::new().code_generation(),
    ] {
        let request = builder.user("hello").build_validated().unwrap();
        assert_eq!(request.model, models::HAIKU_5_5);
        assert_eq!(request.temperature, None);
        assert_eq!(request.top_p, None);
        let body = serde_json::to_value(&request).unwrap();
        assert!(body.get("temperature").is_none());
        assert!(body.get("top_p").is_none());
    }
    // A preset keeps its non-sampling settings.
    let creative = MessageBuilder::new().creative().build();
    assert_eq!(creative.max_tokens, 2000);

    // Presets resolve against the final model, whatever the call order.
    let before = MessageBuilder::new()
        .creative()
        .model(models::SONNET_4_6)
        .build();
    let after = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .creative()
        .build();
    for request in [before, after] {
        assert_eq!(request.temperature, Some(0.9));
        assert_eq!(request.top_p, Some(0.95));
    }
    let raised = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .creative()
        .model(models::OPUS_5_5)
        .build();
    assert_eq!(raised.temperature, None);

    // Explicit values always win and are always sent.
    let explicit = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .creative()
        .temperature(0.2)
        .build();
    assert_eq!(explicit.temperature, Some(0.2));
    assert_eq!(explicit.top_p, Some(0.95));
    let overridden = MessageBuilder::new()
        .model(models::SONNET_4_6)
        .temperature(0.2)
        .creative()
        .build();
    assert_eq!(overridden.temperature, Some(0.9));

    // Batch presets go through the same resolution.
    let batch = BatchBuilder::new()
        .add_creative("haiku", models::HAIKU_5_5, "hi", 100)
        .add_creative("sonnet", models::SONNET_4_6, "hi", 100)
        .build();
    assert_eq!(batch.requests[0].params.temperature, None);
    assert_eq!(batch.requests[1].params.temperature, Some(0.9));
}
