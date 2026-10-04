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
#[path = "tool_runner_test/budgets.rs"]
mod budgets;
#[path = "tool_runner_test/controlled_delivery.rs"]
mod controlled_delivery;
#[path = "tool_runner_test/execution.rs"]
mod execution;
#[path = "tool_runner_test/fixtures.rs"]
mod fixtures;
#[path = "tool_runner_test/history.rs"]
mod history;
#[path = "tool_runner_test/ready_cancellation.rs"]
mod ready_cancellation;
#[path = "tool_runner_test/streaming.rs"]
mod streaming;
#[path = "tool_runner_test/validation.rs"]
mod validation;

use fixtures::*;
