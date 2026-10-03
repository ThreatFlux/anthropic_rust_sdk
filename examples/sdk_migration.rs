//! Compile-checked 0.4 migration: constructors, counting, and lossless replay.
use serde_json::json;
use threatflux_anthropic_sdk::{
    config::models, ContentBlock, MessageBuilder, MessageResponse, ReplayUnknownPolicy, StopReason,
    TokenCountRequest, Usage,
};

fn main() -> threatflux_anthropic_sdk::Result<()> {
    let request = MessageBuilder::new()
        .model(models::OPUS_5_5)
        .user("Explain ownership")
        .adaptive_thinking()
        .build_validated()?;
    let counting = TokenCountRequest::from_message(&request)?;
    assert!(counting.thinking.is_some());

    let opaque = ContentBlock::raw(json!({"type":"future_block","payload":{"id":"opaque"}}))?;
    let mut response = MessageResponse::new("msg_example", models::OPUS_5_5, Usage::new(10, 5));
    response.content = vec![ContentBlock::text("Ownership controls lifetimes."), opaque];
    response.stop_reason = Some(StopReason::EndTurn);
    let history = response.to_conversation_message(ReplayUnknownPolicy::Preserve)?;
    assert_eq!(
        serde_json::to_value(&history.content)?[1]["payload"]["id"],
        "opaque"
    );
    match response.stop_reason {
        Some(StopReason::EndTurn) => println!("Complete response retained in history"),
        Some(StopReason::Unknown(reason)) => println!("Future stop reason: {reason}"),
        _ => {}
    }
    Ok(())
}
