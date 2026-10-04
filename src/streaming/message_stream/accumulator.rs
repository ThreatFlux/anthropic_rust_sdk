//! Strict lifecycle and response snapshot accumulation.

use super::{content_delta::apply_content_delta, StreamLimits};
use crate::{
    error::{AnthropicError, Result},
    models::{
        ContentBlock, ContentBlockDelta, FieldUpdate, MessageDelta, MessageResponse, StreamEvent,
        UsageDelta,
    },
};
use serde_json::Value;

pub(super) struct MessageAccumulator {
    message: Option<MessageResponse>,
    blocks: Vec<Value>,
    open: Vec<bool>,
    inputs: Vec<Option<String>>,
    stopped: bool,
    limits: StreamLimits,
}

impl MessageAccumulator {
    pub(super) fn new(limits: StreamLimits) -> Self {
        Self {
            message: None,
            blocks: Vec::new(),
            open: Vec::new(),
            inputs: Vec::new(),
            stopped: false,
            limits,
        }
    }
    pub(super) fn apply(&mut self, event: StreamEvent) -> Result<()> {
        if matches!(event, StreamEvent::Ping | StreamEvent::Unknown { .. }) {
            return Ok(());
        }
        if self.stopped {
            return Err(AnthropicError::stream("Event received after message_stop"));
        }
        match event {
            StreamEvent::Ping | StreamEvent::Unknown { .. } => Ok(()),
            StreamEvent::Error { .. } => Err(AnthropicError::stream(
                "API returned an error event during message streaming",
            )),
            StreamEvent::MessageStart { message } => self.start(message),
            StreamEvent::ContentBlockStart {
                index,
                content_block,
            } => self.start_block(index, content_block),
            StreamEvent::ContentBlockDelta { index, delta } => self.update_block(index, delta),
            StreamEvent::ContentBlockStop { index } => self.stop_block(index),
            StreamEvent::MessageDelta {
                delta,
                usage,
                context_management,
                input_transformations,
                ..
            } => self.update_message(delta, usage, context_management, input_transformations),
            StreamEvent::MessageStop => self.stop(),
        }
    }

    fn start(&mut self, message: MessageResponse) -> Result<()> {
        if self.message.is_some() {
            return Err(AnthropicError::stream("Duplicate message_start"));
        }
        if message.content.len() > self.limits.max_content_blocks {
            return Err(AnthropicError::stream("Content block limit exceeded"));
        }
        self.blocks = message
            .content
            .iter()
            .map(serde_json::to_value)
            .collect::<std::result::Result<_, _>>()?;
        self.open = vec![false; self.blocks.len()];
        self.inputs = vec![None; self.blocks.len()];
        self.message = Some(message);
        Ok(())
    }

    fn start_block(&mut self, index: usize, content_block: ContentBlock) -> Result<()> {
        self.require_start()?;
        if index != self.blocks.len() {
            return Err(AnthropicError::stream(
                "Content block start index must be the next contiguous index",
            ));
        }
        if index >= self.limits.max_content_blocks {
            return Err(AnthropicError::stream("Content block limit exceeded"));
        }
        let block = serde_json::to_value(content_block)?;
        if block["type"] == "fallback" {
            let model = block
                .get("to")
                .and_then(|to| to.get("model"))
                .and_then(Value::as_str)
                .ok_or_else(|| AnthropicError::stream("Fallback block is missing to.model"))?;
            self.message.as_mut().expect("start checked").model = model.to_owned();
        }
        self.blocks.push(block);
        self.open.push(true);
        self.inputs.push(None);
        Ok(())
    }

    fn update_block(&mut self, index: usize, delta: ContentBlockDelta) -> Result<()> {
        self.require_open(index)?;
        delta.validate()?;
        apply_content_delta(
            &mut self.blocks[index],
            &mut self.inputs[index],
            delta,
            self.limits.max_tool_input_bytes,
        )
    }

    fn stop_block(&mut self, index: usize) -> Result<()> {
        self.require_open(index)?;
        if let Some(input) = self.inputs[index].take() {
            let value: Value = serde_json::from_str(&input)
                .map_err(|_| AnthropicError::stream("Invalid completed tool input JSON"))?;
            if !value.is_object() {
                return Err(AnthropicError::stream(
                    "Completed tool input JSON must be an object",
                ));
            }
            self.blocks[index]["input"] = value;
        }
        // Validate without coercing malformed known payloads.
        serde_json::from_value::<ContentBlock>(self.blocks[index].clone())
            .map_err(|_| AnthropicError::stream("Invalid completed content block"))?;
        self.open[index] = false;
        Ok(())
    }

    fn update_message(
        &mut self,
        delta: MessageDelta,
        usage: UsageDelta,
        context_management: FieldUpdate<Value>,
        input_transformations: FieldUpdate<Value>,
    ) -> Result<()> {
        self.require_start()?;
        let message = self.message.as_mut().expect("start checked");
        delta.stop_reason.apply(&mut message.stop_reason);
        delta.stop_sequence.apply(&mut message.stop_sequence);
        delta.stop_details.apply(&mut message.stop_details);
        delta.container.apply_non_null(&mut message.container);
        context_management.apply_non_null(&mut message.context_management);
        input_transformations.apply_non_null(&mut message.input_transformations);
        usage.apply(&mut message.usage);
        Ok(())
    }

    fn stop(&mut self) -> Result<()> {
        self.require_start()?;
        if self.open.iter().any(|open| *open) || self.inputs.iter().any(Option::is_some) {
            return Err(AnthropicError::stream(
                "message_stop received with unfinished content blocks",
            ));
        }
        self.stopped = true;
        Ok(())
    }
    fn require_start(&self) -> Result<()> {
        if self.message.is_none() {
            return Err(AnthropicError::stream(
                "Event received before message_start",
            ));
        }
        Ok(())
    }
    fn require_open(&self, index: usize) -> Result<()> {
        self.require_start()?;
        if !self.open.get(index).copied().unwrap_or(false) {
            return Err(AnthropicError::stream(
                "Content delta/stop index does not refer to an open block",
            ));
        }
        Ok(())
    }
    pub(super) fn finish(self) -> Result<MessageResponse> {
        if !self.stopped {
            return Err(AnthropicError::stream("Stream ended before message_stop"));
        }
        let mut message = self
            .message
            .ok_or_else(|| AnthropicError::stream("No message_start received"))?;
        message.content = self
            .blocks
            .into_iter()
            .map(serde_json::from_value)
            .collect::<std::result::Result<_, _>>()?;
        Ok(message)
    }
}
