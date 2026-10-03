//! Offline wire fixtures verified against Anthropic's Python SDK revision
//! 18f25547f20cf5f01da69ac611e700e3bc9ebf21 on 2026-10-03.

use serde_json::{json, Value};
use threatflux_anthropic_sdk::models::{
    batch::BatchRequestItem, ContentBlock, DocumentSource, ImageSource, Message,
    MessageBatchCreateRequest, MessageBatchResult, MessageBatchResultEntry, MessageRequest,
    MessageResponse, RawContentBlock, ReplayUnknownPolicy, Role, SendEvent, SessionCreateRequest,
    SessionEvent, StopReason, TokenCountRequest, ToolChoice, Usage,
};

use threatflux_anthropic_sdk::{Client, Config};
use wiremock::{
    matchers::{body_partial_json, header, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[path = "protocol_content_test/choices.rs"]
mod choices;
#[path = "protocol_content_test/content.rs"]
mod content;
#[path = "protocol_content_test/fixtures.rs"]
mod fixtures;
#[path = "protocol_content_test/request_boundaries.rs"]
mod request_boundaries;
#[path = "protocol_content_test/sessions.rs"]
mod sessions;

use fixtures::*;
