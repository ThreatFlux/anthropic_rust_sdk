use super::*;

type CallOutcome = RunResult<ContentBlock>;

#[derive(Default)]
struct CompletedTurn {
    blocks: Vec<(usize, ContentBlock)>,
    message_index: Option<usize>,
}
impl CompletedTurn {
    fn record(
        &mut self,
        result: &mut ToolRunnerResult,
        index: usize,
        id: String,
        block: ContentBlock,
    ) {
        result.completed_calls.push(id.clone());
        self.blocks.push((index, attach_tool_id(block, id)));
        self.blocks.sort_by_key(|(index, _)| *index);
        let message_index = *self.message_index.get_or_insert_with(|| {
            let index = result.messages.len();
            result.messages.push(Message::new(Role::User, Vec::new()));
            index
        });
        // Persist before the next await so interruption retains completed external work.
        result.messages[message_index].content =
            self.blocks.iter().map(|(_, block)| block.clone()).collect();
    }
}

impl ToolRunner {
    pub(super) async fn approve_calls(&self, prepared: &PreparedCalls) -> RunResult<()> {
        self.check_cancelled()?;
        if let Some(hook) = &self.hook {
            for (call, _) in prepared {
                self.check_cancelled()?;
                let approval = tokio::time::timeout(self.options.call_timeout, hook(call.clone()))
                    .await
                    .unwrap_or_else(|_| Err(AnthropicError::timeout(self.options.call_timeout)));
                self.check_cancelled()?;
                approval.map_err(|error| (ToolRunnerTermination::CallbackFailed, Some(error)))?;
            }
        }
        Ok(())
    }

    fn check_cancelled(&self) -> RunResult<()> {
        if self.cancellation.is_cancelled() {
            return Err((ToolRunnerTermination::Cancelled, None));
        }
        Ok(())
    }

    pub(super) async fn execute_calls(
        &self,
        prepared: PreparedCalls,
        choice: Option<&ToolChoice>,
        result: &mut ToolRunnerResult,
        started: &StartedCalls,
    ) -> RunResult<()> {
        self.check_cancelled()?;
        let parallel = if choice.and_then(ToolChoice::disable_parallel_tool_use) == Some(true) {
            1
        } else {
            self.options.max_parallel_calls
        };
        let execution = futures::stream::iter(prepared.into_iter().enumerate())
            .map(|(index, (call, tool))| self.execute_one(index, call, tool, started.clone()))
            .buffer_unordered(parallel);
        tokio::pin!(execution);
        let mut completed = CompletedTurn::default();
        while let Some((index, id, outcome)) = execution.next().await {
            let block = outcome?;
            completed.record(result, index, id, block);
            self.check_cancelled()?;
        }
        Ok(())
    }

    async fn execute_one(
        &self,
        index: usize,
        call: ToolCall,
        tool: RegisteredTool,
        started: StartedCalls,
    ) -> (usize, String, CallOutcome) {
        if let Err(cancellation) = self.check_cancelled() {
            return (index, call.id, Err(cancellation));
        }
        started
            .lock()
            .expect("tool-start ledger poisoned")
            .push(call.id.clone());
        let outcome = tokio::time::timeout(self.options.call_timeout, (tool.callback)(call.input))
            .await
            .unwrap_or_else(|_| Err(AnthropicError::timeout(self.options.call_timeout)));
        let block = self
            .make_result_block(outcome)
            .map_err(|(termination, error)| (termination, Some(error)));
        (index, call.id, block)
    }

    fn make_result_block(
        &self,
        outcome: Result<ToolResultContent>,
    ) -> std::result::Result<ContentBlock, (ToolRunnerTermination, AnthropicError)> {
        let (content, is_error) = match outcome {
            Ok(ToolResultContent::Json(value)) => {
                (ToolResultContent::Text(value.to_string()), false)
            }
            Ok(content) => (content, false),
            Err(_) if self.options.callback_errors == CallbackErrorPolicy::ReturnSanitizedError => {
                (
                    ToolResultContent::Text("Tool execution failed".into()),
                    true,
                )
            }
            Err(error) => return Err((ToolRunnerTermination::CallbackFailed, error)),
        };
        let bytes = serde_json::to_vec(&content).map_err(|error| {
            (
                ToolRunnerTermination::UnsupportedReplay,
                AnthropicError::from(error),
            )
        })?;
        if bytes.len() > self.options.max_result_bytes {
            return Err((
                ToolRunnerTermination::ResultLimit,
                AnthropicError::invalid_input("tool result exceeds configured byte limit"),
            ));
        }
        let block = ContentBlock::ToolResult {
            tool_use_id: "pending".into(),
            content: Some(content),
            is_error: Some(is_error),
            extra: ExtraFields::new(),
        };
        block
            .checked_replay(&Role::User, self.options.unknown_content)
            .map_err(|error| (ToolRunnerTermination::UnsupportedReplay, error))
    }
}

fn attach_tool_id(mut block: ContentBlock, id: String) -> ContentBlock {
    if let ContentBlock::ToolResult { tool_use_id, .. } = &mut block {
        *tool_use_id = id;
    }
    block
}
