use super::*;

/// A requested client invocation supplied to an optional execution hook.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolCall {
    /// Original identifier used in the corresponding tool result.
    pub id: String,
    /// Explicitly registered name.
    pub name: String,
    /// Complete, already decoded object input.
    pub input: Value,
}

/// Callback error handling. Remote error content never includes callback details.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CallbackErrorPolicy {
    /// Stop and return the partial transcript plus the local callback error.
    Stop,
    /// Return fixed sanitized `is_error` content to the model, then continue.
    ReturnSanitizedError,
}
/// How a complete compaction block is replayed on a subsequent turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CompactionReplayPolicy {
    /// Stop before callbacks; the caller decides how to continue compacted history.
    Reject,
    /// Keep the full append-only history and compaction block verbatim. Never
    /// manufacture a summary or silently discard messages before the boundary.
    PreserveHistory,
}

/// Finite execution limits. Start from `Default` and change individual fields.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ToolRunnerOptions {
    /// Maximum Messages API turns.
    pub max_turns: usize,
    /// Maximum callbacks started across the whole run.
    pub max_tool_calls: usize,
    /// Deadline for an individual execution hook or callback.
    pub call_timeout: Duration,
    /// Deadline for all HTTP, hook, and callback work.
    pub overall_timeout: Duration,
    /// Maximum serialized tool-result content bytes per call.
    pub max_result_bytes: usize,
    /// Maximum concurrently executing callbacks; defaults to one.
    pub max_parallel_calls: usize,
    /// Callback failure handling; defaults to returning a partial result.
    pub callback_errors: CallbackErrorPolicy,
    /// Unknown content replay requires an explicit preservation choice.
    pub unknown_content: ReplayUnknownPolicy,
    /// Explicit handling for server-produced compaction snapshots.
    pub compaction: CompactionReplayPolicy,
}
impl Default for ToolRunnerOptions {
    fn default() -> Self {
        Self {
            max_turns: 20,
            max_tool_calls: 100,
            call_timeout: Duration::from_secs(30),
            overall_timeout: Duration::from_secs(300),
            max_result_bytes: 1024 * 1024,
            max_parallel_calls: 1,
            callback_errors: CallbackErrorPolicy::Stop,
            unknown_content: ReplayUnknownPolicy::Reject,
            compaction: CompactionReplayPolicy::Reject,
        }
    }
}
impl ToolRunnerOptions {
    /// Reject zero or overflowing budgets before constructing a runner.
    pub fn validate(&self) -> Result<()> {
        if self.max_turns == 0
            || self.max_tool_calls == 0
            || self.call_timeout.is_zero()
            || self.overall_timeout.is_zero()
            || self.max_result_bytes == 0
            || self.max_parallel_calls == 0
        {
            return Err(AnthropicError::invalid_input(
                "tool-runner budgets must be nonzero",
            ));
        }
        if tokio::time::Instant::now()
            .checked_add(self.overall_timeout)
            .is_none()
            || tokio::time::Instant::now()
                .checked_add(self.call_timeout)
                .is_none()
        {
            return Err(AnthropicError::invalid_input(
                "tool-runner deadline exceeds supported duration",
            ));
        }
        Ok(())
    }
}
