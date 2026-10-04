use super::*;

struct TurnState {
    seen_ids: HashSet<String>,
    offered: HashSet<String>,
}
impl TurnState {
    fn new(request: &MessageRequest) -> Self {
        let seen_ids = request
            .messages
            .iter()
            .flat_map(|message| &message.content)
            .filter_map(|block| match block {
                ContentBlock::ToolUse { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect();
        let offered = request
            .tools
            .as_deref()
            .unwrap_or_default()
            .iter()
            .filter(|tool| tool.is_client())
            .map(|tool| tool.name.clone())
            .collect();
        Self { seen_ids, offered }
    }
}

impl ToolRunner {
    pub(super) async fn run_loop(
        &self,
        mut request: MessageRequest,
        options: Option<RequestOptions>,
        streaming: bool,
        result: &mut ToolRunnerResult,
        started: &StartedCalls,
    ) -> ToolRunnerTermination {
        let mut state = TurnState::new(&request);
        for _ in 0..self.options.max_turns {
            if self.cancellation.is_cancelled() {
                return ToolRunnerTermination::Cancelled;
            }
            request.messages = result.messages.clone();
            if let Err(failure) = self
                .run_turn(
                    &request,
                    options.clone(),
                    streaming,
                    result,
                    started,
                    &mut state,
                )
                .await
            {
                return result.stop(failure);
            }
        }
        ToolRunnerTermination::TurnLimit
    }

    async fn run_turn(
        &self,
        request: &MessageRequest,
        options: Option<RequestOptions>,
        streaming: bool,
        result: &mut ToolRunnerResult,
        started: &StartedCalls,
        state: &mut TurnState,
    ) -> RunResult<()> {
        let response = self
            .receive_turn(request.clone(), options, streaming)
            .await
            .map_err(|error| (ToolRunnerTermination::TransportFailed, Some(error)))?;
        result.record_response(&response);
        if let Some(termination) = response_termination(response.stop_reason.as_ref()) {
            return Err((termination, None));
        }
        let prepared = self.prepare_turn(request, &response, state, started)?;
        self.approve_calls(&prepared).await?;
        self.execute_calls(prepared, request.tool_choice.as_ref(), result, started)
            .await
    }

    async fn receive_turn(
        &self,
        request: MessageRequest,
        options: Option<RequestOptions>,
        streaming: bool,
    ) -> Result<MessageResponse> {
        if streaming {
            self.client
                .messages()
                .create_stream(request, options)
                .await?
                .collect_message()
                .await
        } else {
            self.client.messages().create(request, options).await
        }
    }

    fn prepare_turn(
        &self,
        request: &MessageRequest,
        response: &MessageResponse,
        state: &mut TurnState,
        started: &StartedCalls,
    ) -> RunResult<PreparedCalls> {
        if response
            .content
            .iter()
            .any(|block| matches!(block, ContentBlock::Compaction { .. }))
            && self.options.compaction == CompactionReplayPolicy::Reject
        {
            return Err((ToolRunnerTermination::UnsupportedReplay, None));
        }
        response
            .to_conversation_message(self.options.unknown_content)
            .map_err(|error| (ToolRunnerTermination::UnsupportedReplay, Some(error)))?;
        let calls = client_calls(response);
        if calls.is_empty() {
            return Err((ToolRunnerTermination::NoClientToolCalls, None));
        }
        let started_count = started.lock().expect("tool-start ledger poisoned").len();
        if calls.len() > self.options.max_tool_calls.saturating_sub(started_count) {
            return Err((ToolRunnerTermination::CallLimit, None));
        }
        self.prepare_calls(calls, request.tool_choice.as_ref(), state)
    }

    fn prepare_calls(
        &self,
        calls: Vec<ToolCall>,
        choice: Option<&ToolChoice>,
        state: &mut TurnState,
    ) -> RunResult<PreparedCalls> {
        let mut prepared = Vec::with_capacity(calls.len());
        for call in calls {
            validate_call_allowed(&call, choice, state)
                .map_err(|error| (ToolRunnerTermination::InvalidToolCall, Some(error)))?;
            let tool = self
                .registry
                .tools
                .get(&call.name)
                .expect("offered registered tool")
                .clone();
            (tool.validator)(&call.input)
                .map_err(|error| (ToolRunnerTermination::InvalidToolCall, Some(error)))?;
            prepared.push((call, tool));
        }
        Ok(prepared)
    }
}

fn client_calls(response: &MessageResponse) -> Vec<ToolCall> {
    response
        .content
        .iter()
        .filter_map(|block| match block {
            ContentBlock::ToolUse {
                id, name, input, ..
            } => Some(ToolCall {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            }),
            _ => None,
        })
        .collect()
}

fn validate_call_allowed(
    call: &ToolCall,
    choice: Option<&ToolChoice>,
    state: &mut TurnState,
) -> Result<()> {
    if call.id.is_empty()
        || !state.seen_ids.insert(call.id.clone())
        || !state.offered.contains(&call.name)
        || choice.is_some_and(|choice| choice.kind() == "none")
        || choice
            .and_then(ToolChoice::forced_tool_name)
            .is_some_and(|name| name != call.name)
    {
        return Err(AnthropicError::invalid_input(
            "unknown, disabled, duplicate, or mismatched tool invocation",
        ));
    }
    Ok(())
}

fn response_termination(reason: Option<&StopReason>) -> Option<ToolRunnerTermination> {
    Some(match reason {
        Some(StopReason::ToolUse) => return None,
        Some(StopReason::EndTurn) => ToolRunnerTermination::EndTurn,
        Some(StopReason::StopSequence) => ToolRunnerTermination::StopSequence,
        Some(StopReason::Refusal) => ToolRunnerTermination::Refusal,
        Some(StopReason::MaxTokens) => ToolRunnerTermination::MaxTokens,
        Some(StopReason::PauseTurn) => ToolRunnerTermination::PauseTurn,
        Some(StopReason::ModelContextWindowExceeded) => {
            ToolRunnerTermination::ContextWindowExceeded
        }
        Some(StopReason::Unknown(reason)) => {
            ToolRunnerTermination::UnknownStopReason(reason.clone())
        }
        None => ToolRunnerTermination::MissingStopReason,
    })
}
