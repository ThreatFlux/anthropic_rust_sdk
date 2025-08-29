//! Builder for constructing message requests

use crate::builders::common::{FluentBuilder, ParameterBuilder, ValidatedBuilder, ValidationUtils};
use crate::models::{
    common::{ContentBlock, ImageSource, Metadata, Role, Tool, ToolChoice},
    message::{Message, MessageRequest, ThinkingConfig},
};
use std::path::Path;

/// Builder for constructing message requests with a fluent API
#[derive(Debug, Clone)]
pub struct MessageBuilder {
    request: MessageRequest,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new() -> Self {
        Self {
            request: MessageRequest::new(),
        }
    }

    /// Create a message builder with a specific model
    pub fn with_model(model: impl Into<String>) -> Self {
        Self::new().model(model)
    }

    /// Set the model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.request.model = model.into();
        self
    }

    /// Set max tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.request.max_tokens = max_tokens;
        self
    }

    /// Set system prompt
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.request.system = Some(system.into());
        self
    }

    /// Set temperature (0.0 to 1.0)
    pub fn temperature(mut self, temperature: f32) -> Self {
        // Note: We clamp here for backwards compatibility, but validation will catch invalid values
        self.request.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Set top-p (0.0 to 1.0)
    pub fn top_p(mut self, top_p: f32) -> Self {
        // Note: We clamp here for backwards compatibility, but validation will catch invalid values
        self.request.top_p = Some(top_p.clamp(0.0, 1.0));
        self
    }

    /// Set top-k
    pub fn top_k(mut self, top_k: u32) -> Self {
        self.request.top_k = Some(top_k);
        self
    }

    /// Enable streaming
    pub fn stream(mut self) -> Self {
        self.request.stream = Some(true);
        self
    }

    /// Disable streaming (default)
    pub fn no_stream(mut self) -> Self {
        self.request.stream = Some(false);
        self
    }

    /// Add a stop sequence
    pub fn stop_sequence(mut self, stop: impl Into<String>) -> Self {
        self.request
            .stop_sequences
            .get_or_insert_with(Vec::new)
            .push(stop.into());
        self
    }

    /// Add multiple stop sequences
    pub fn stop_sequences(mut self, stops: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let sequences = self.request.stop_sequences.get_or_insert_with(Vec::new);
        sequences.extend(stops.into_iter().map(|s| s.into()));
        self
    }

    /// Add a tool
    pub fn tool(mut self, tool: Tool) -> Self {
        self.request.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Add multiple tools
    pub fn tools(mut self, tools: impl IntoIterator<Item = Tool>) -> Self {
        let tool_list = self.request.tools.get_or_insert_with(Vec::new);
        tool_list.extend(tools);
        self
    }

    /// Add a simple function tool
    pub fn function_tool(
        mut self,
        name: impl Into<String>,
        description: impl Into<String>,
        parameters: serde_json::Value,
    ) -> Self {
        let tool = Tool::new(name, description, parameters);
        self.request.tools.get_or_insert_with(Vec::new).push(tool);
        self
    }

    /// Set tool choice
    pub fn tool_choice(mut self, tool_choice: ToolChoice) -> Self {
        self.request.tool_choice = Some(tool_choice);
        self
    }

    /// Require tool use (any tool)
    pub fn require_tool_use(mut self) -> Self {
        self.request.tool_choice = Some(ToolChoice::Any);
        self
    }

    /// Require specific tool
    pub fn require_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.request.tool_choice = Some(ToolChoice::Tool {
            name: tool_name.into(),
        });
        self
    }

    /// Set metadata
    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.request.metadata = Some(metadata);
        self
    }

    /// Set user ID in metadata
    pub fn user_id(mut self, user_id: impl Into<String>) -> Self {
        let metadata = self.request.metadata.unwrap_or_default();
        self.request.metadata = Some(metadata.with_user_id(user_id));
        self
    }

    /// Add custom metadata field
    pub fn custom_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let metadata = self.request.metadata.unwrap_or_default();
        self.request.metadata = Some(metadata.with_custom(key, value));
        self
    }

    /// Add a message
    pub fn message(mut self, message: Message) -> Self {
        self.request.messages.push(message);
        self
    }

    /// Add multiple messages
    pub fn messages(mut self, messages: impl IntoIterator<Item = Message>) -> Self {
        self.request.messages.extend(messages);
        self
    }

    /// Add a user message with text
    pub fn user(mut self, text: impl Into<String>) -> Self {
        self.request.messages.push(Message::user(text));
        self
    }

    /// Add an assistant message with text
    pub fn assistant(mut self, text: impl Into<String>) -> Self {
        self.request.messages.push(Message::assistant(text));
        self
    }

    /// Add a user message with image
    pub fn user_with_image(
        mut self,
        text: impl Into<String>,
        image_data: Vec<u8>,
        media_type: impl Into<String>,
    ) -> Self {
        let image_source = ImageSource::from_bytes(media_type, &image_data);
        let mut message = Message::user(text);
        message.content.push(ContentBlock::image(image_source));
        self.request.messages.push(message);
        self
    }

    /// Add a user message with base64 image
    pub fn user_with_base64_image(
        mut self,
        text: impl Into<String>,
        base64_data: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        let image_source = ImageSource::base64(media_type, base64_data);
        let mut message = Message::user(text);
        message.content.push(ContentBlock::image(image_source));
        self.request.messages.push(message);
        self
    }

    /// Add a user message with image from file path
    pub async fn user_with_image_file(
        self,
        text: impl Into<String>,
        image_path: impl AsRef<Path>,
    ) -> Result<Self, crate::error::AnthropicError> {
        let path = image_path.as_ref();
        let image_data = tokio::fs::read(path).await.map_err(|e| {
            crate::error::AnthropicError::file_error(format!("Failed to read image: {}", e))
        })?;

        let media_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        Ok(self.user_with_image(text, image_data, media_type))
    }

    /// Add conversation history
    pub fn conversation(mut self, messages: &[(Role, &str)]) -> Self {
        for (role, content) in messages {
            match role {
                Role::User => self = self.user(*content),
                Role::Assistant => self = self.assistant(*content),
                Role::System => {
                    // System messages are handled as system prompt, not in conversation
                    self = self.system(*content);
                }
            }
        }
        self
    }

    /// Create a simple question-answer conversation
    pub fn qa(mut self, question: impl Into<String>, previous_context: Option<&str>) -> Self {
        if let Some(context) = previous_context {
            self = self.system(context);
        }
        self.user(question)
    }

    /// Create a chat completion with conversation history
    pub fn chat(self, messages: &[(Role, &str)]) -> Self {
        self.conversation(messages)
    }

    /// Add a tool use result
    pub fn tool_result(
        mut self,
        tool_use_id: impl Into<String>,
        result: impl Into<String>,
    ) -> Self {
        let content_block = ContentBlock::tool_result(tool_use_id, Some(result.into()));
        let message = Message::new(Role::User, vec![content_block]);
        self.request.messages.push(message);
        self
    }

    /// Add a tool use error result
    pub fn tool_error(mut self, tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        let content_block = ContentBlock::tool_error(tool_use_id, error);
        let message = Message::new(Role::User, vec![content_block]);
        self.request.messages.push(message);
        self
    }

    /// Preset for code generation (includes stop sequences specific to code)
    pub fn code_generation(self) -> Self {
        use crate::builders::common::PresetConfig;
        PresetConfig::CODE_GENERATION
            .apply_to_builder(self)
            .stop_sequences(vec!["```".to_string()])
    }

    /// Enable extended thinking mode (Claude 4 models)
    pub fn thinking(mut self, budget_tokens: u32) -> Self {
        self.request.thinking = Some(ThinkingConfig::enabled(budget_tokens));
        self
    }

    /// Enable extended thinking mode with tool use (Claude 4 models)
    pub fn thinking_with_tools(mut self, budget_tokens: u32) -> Self {
        self.request.thinking = Some(ThinkingConfig::enabled_with_tools(budget_tokens));
        self
    }

    /// Preset for Claude 4 Opus with maximum thinking
    pub fn opus_4_deep_thinking(self) -> Self {
        self.model(crate::config::models::OPUS_4_1)
            .thinking(64000)
            .max_tokens(8192)
    }

    /// Preset for Claude 4 Sonnet with balanced thinking
    pub fn sonnet_4_balanced(self) -> Self {
        self.model(crate::config::models::SONNET_4)
            .thinking(32000)
            .max_tokens(4096)
    }

    /// Preset for Claude 4 with tool use during thinking
    pub fn claude_4_agentic(self) -> Self {
        self.model(crate::config::models::OPUS_4_1)
            .thinking_with_tools(50000)
            .max_tokens(8192)
    }

    /// Build the message request
    pub fn build(self) -> MessageRequest {
        self.request
    }

    /// Build and validate the message request
    pub fn build_validated(self) -> Result<MessageRequest, crate::error::AnthropicError> {
        let request = self.request;

        // Use common validation utilities
        ValidationUtils::validate_messages_not_empty(request.messages.len(), "MessageRequest")?;
        ValidationUtils::validate_max_tokens(request.max_tokens, "MessageRequest")?;

        if let Some(temp) = request.temperature {
            ValidationUtils::validate_temperature(temp)?;
        }

        if let Some(top_p) = request.top_p {
            ValidationUtils::validate_top_p(top_p)?;
        }

        // Validate Claude 4 specific constraints
        ValidationUtils::validate_claude_4_constraints(
            &request.model,
            request.temperature,
            request.top_p,
        )?;

        // Validate thinking configuration
        if let Some(thinking) = &request.thinking {
            ValidationUtils::validate_thinking_config(&request.model, thinking.budget_tokens)?;
        }

        Ok(request)
    }

    /// Get a reference to the current request (for inspection)
    pub fn as_request(&self) -> &MessageRequest {
        &self.request
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<MessageBuilder> for MessageRequest {
    fn from(builder: MessageBuilder) -> Self {
        builder.build()
    }
}

impl ValidatedBuilder<MessageRequest> for MessageBuilder {
    fn build_validated(self) -> Result<MessageRequest, crate::error::AnthropicError> {
        self.build_validated()
    }
}

impl FluentBuilder for MessageBuilder {
    fn inspect(&self) -> Option<&dyn std::fmt::Debug> {
        Some(self)
    }
}

impl ParameterBuilder for MessageBuilder {
    fn temperature(self, temperature: f32) -> Self {
        self.temperature(temperature)
    }

    fn top_p(self, top_p: f32) -> Self {
        self.top_p(top_p)
    }

    fn max_tokens(self, max_tokens: u32) -> Self {
        self.max_tokens(max_tokens)
    }
}
