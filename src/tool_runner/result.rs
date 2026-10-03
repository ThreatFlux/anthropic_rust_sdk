use super::*;

/// Why a bounded tool run stopped. Details and usage remain in its result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ToolRunnerTermination {
    /// Natural completion.
    EndTurn,
    /// A configured stop sequence ended generation.
    StopSequence,
    /// The model refused.
    Refusal,
    /// The response hit its generation limit.
    MaxTokens,
    /// Server tools paused; automatic continuation is not attempted.
    PauseTurn,
    /// The context window was exhausted.
    ContextWindowExceeded,
    /// An unfamiliar reason is returned verbatim.
    UnknownStopReason(String),
    /// A complete response had no stop reason.
    MissingStopReason,
    /// Maximum model turns reached.
    TurnLimit,
    /// The next complete batch of calls would exceed the call limit.
    CallLimit,
    /// Caller cancellation.
    Cancelled,
    /// The whole-run deadline expired.
    OverallTimeout,
    /// Registered callback or execution-hook failure.
    CallbackFailed,
    /// Invalid or incompatible request configuration.
    InvalidRequest,
    /// A tool was unknown, duplicated, disabled, or failed input validation.
    InvalidToolCall,
    /// A response cannot be safely replayed under the selected policy.
    UnsupportedReplay,
    /// `tool_use` was returned without a client tool invocation.
    NoClientToolCalls,
    /// HTTP, response parsing, or incomplete SSE failed.
    TransportFailed,
    /// A callback produced content above the configured byte limit.
    ResultLimit,
}

/// A complete or partial run. Callback/transport failures do not discard state.
#[derive(Debug)]
#[non_exhaustive]
pub struct ToolRunnerResult {
    /// Recorded conversation, including the original request, complete assistant
    /// blocks, and every completed result. Interrupted runs may omit results for
    /// started calls; reconcile those IDs before resuming the conversation.
    pub messages: Vec<Message>,
    /// Complete received response snapshots, with original metadata and usage.
    pub responses: Vec<MessageResponse>,
    /// Per-turn usage, kept separate rather than merging incompatible metadata.
    pub usage: Vec<Usage>,
    /// IDs whose callbacks started. Interrupted callbacks may already have had
    /// external side effects, so callers must not blindly retry these IDs.
    pub started_calls: Vec<String>,
    /// IDs whose callbacks completed successfully or produced sanitized errors.
    pub completed_calls: Vec<String>,
    /// Stop cause.
    pub termination: ToolRunnerTermination,
    /// A local failure, never automatically copied into prompts or logs.
    pub error: Option<AnthropicError>,
}
impl ToolRunnerResult {
    pub(super) fn record_response(&mut self, response: &MessageResponse) {
        self.usage.push(response.usage.clone());
        self.messages
            .push(Message::new(Role::Assistant, response.content.clone()));
        self.responses.push(response.clone());
    }

    pub(super) fn stop(&mut self, failure: RunFailure) -> ToolRunnerTermination {
        self.error = failure.1;
        failure.0
    }

    /// Sum the authoritative per-turn token totals using wide counters. Thinking
    /// details and compaction iteration usage are not double-counted.
    pub fn total_tokens(&self) -> u64 {
        self.usage
            .iter()
            .map(|usage| {
                u64::from(usage.input_tokens)
                    + u64::from(usage.cache_creation_input_tokens)
                    + u64::from(usage.cache_read_input_tokens)
                    + u64::from(usage.output_tokens)
            })
            .sum()
    }
    /// The final received response, including a partial-run response if present.
    pub fn last_response(&self) -> Option<&MessageResponse> {
        self.responses.last()
    }
}
