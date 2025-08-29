//! Builder for constructing batch requests

use crate::builders::common::{FluentBuilder, ParameterBuilder, ValidatedBuilder, ValidationUtils};
use crate::builders::MessageBuilder;
use crate::models::{
    batch::{BatchRequestItem, MessageBatchCreateRequest},
    message::MessageRequest,
};

/// Builder for constructing batch requests with a fluent API
#[derive(Debug, Clone)]
pub struct BatchBuilder {
    requests: Vec<BatchRequestItem>,
}

impl BatchBuilder {
    /// Create a new batch builder
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Add a request item to the batch
    pub fn add_item(mut self, item: BatchRequestItem) -> Self {
        self.requests.push(item);
        self
    }

    /// Add multiple request items to the batch
    pub fn add_items(mut self, items: impl IntoIterator<Item = BatchRequestItem>) -> Self {
        self.requests.extend(items);
        self
    }

    /// Add a request with custom ID and message request
    pub fn add_request(mut self, custom_id: impl Into<String>, request: MessageRequest) -> Self {
        let item = BatchRequestItem::new(custom_id, request);
        self.requests.push(item);
        self
    }

    /// Add a simple text request
    pub fn add_simple_request(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        message: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        let request = MessageRequest::new()
            .model(model)
            .max_tokens(max_tokens)
            .add_user_message(message);

        self.add_request(custom_id, request)
    }

    /// Add a request using a message builder
    pub fn add_with_builder(self, custom_id: impl Into<String>, builder: MessageBuilder) -> Self {
        let request = builder.build();
        self.add_request(custom_id, request)
    }

    /// Add a conversation request
    pub fn add_conversation(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        messages: &[(crate::models::common::Role, &str)],
        max_tokens: u32,
    ) -> Self {
        let mut builder = MessageBuilder::new().model(model).max_tokens(max_tokens);

        for (role, content) in messages {
            match role {
                crate::models::common::Role::User => builder = builder.user(*content),
                crate::models::common::Role::Assistant => builder = builder.assistant(*content),
                crate::models::common::Role::System => builder = builder.system(*content),
            }
        }

        self.add_with_builder(custom_id, builder)
    }

    /// Add a Q&A request
    pub fn add_qa(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        question: impl Into<String>,
        context: Option<impl Into<String>>,
        max_tokens: u32,
    ) -> Self {
        let mut builder = MessageBuilder::new()
            .model(model)
            .max_tokens(max_tokens)
            .user(question);

        if let Some(ctx) = context {
            builder = builder.system(ctx);
        }

        self.add_with_builder(custom_id, builder)
    }

    /// Add a creative writing request
    pub fn add_creative(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        prompt: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        let builder = MessageBuilder::new()
            .model(model)
            .creative()
            .max_tokens(max_tokens)
            .user(prompt);

        self.add_with_builder(custom_id, builder)
    }

    /// Add an analytical request
    pub fn add_analytical(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        prompt: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        let builder = MessageBuilder::new()
            .model(model)
            .analytical()
            .max_tokens(max_tokens)
            .user(prompt);

        self.add_with_builder(custom_id, builder)
    }

    /// Add a code generation request
    pub fn add_code_generation(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        prompt: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        let builder = MessageBuilder::new()
            .model(model)
            .code_generation()
            .max_tokens(max_tokens)
            .user(prompt);

        self.add_with_builder(custom_id, builder)
    }

    /// Add a conversational request
    pub fn add_conversational(
        self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        prompt: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        let builder = MessageBuilder::new()
            .model(model)
            .conversational()
            .max_tokens(max_tokens)
            .user(prompt);

        self.add_with_builder(custom_id, builder)
    }

    /// Add multiple similar requests with different inputs
    pub fn add_batch_variations(
        mut self,
        base_custom_id: impl Into<String>,
        model: impl Into<String>,
        base_prompt: impl Into<String>,
        variations: impl IntoIterator<Item = impl Into<String>>,
        max_tokens: u32,
    ) -> Self {
        let base_id = base_custom_id.into();
        let model = model.into();
        let base_prompt = base_prompt.into();

        for (i, variation) in variations.into_iter().enumerate() {
            let custom_id = format!("{}_{}", base_id, i);
            let full_prompt = format!("{} {}", base_prompt, variation.into());

            self = self.add_simple_request(custom_id, &model, full_prompt, max_tokens);
        }

        self
    }

    /// Add requests from a template
    pub fn add_from_template(
        mut self,
        template_custom_id: impl Into<String>,
        model: impl Into<String>,
        template: impl Into<String>,
        substitutions: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        max_tokens: u32,
    ) -> Self {
        let base_id = template_custom_id.into();
        let model = model.into();
        let template = template.into();

        for (i, (key, value)) in substitutions.into_iter().enumerate() {
            let custom_id = format!("{}_{}", base_id, i);
            let prompt = template.replace(&format!("{{{}}}", key.into()), &value.into());

            self = self.add_simple_request(custom_id, &model, prompt, max_tokens);
        }

        self
    }

    /// Set default parameters for subsequent requests
    pub fn with_defaults(
        self,
        model: impl Into<String>,
        max_tokens: u32,
    ) -> BatchBuilderWithDefaults {
        BatchBuilderWithDefaults {
            builder: self,
            default_model: model.into(),
            default_max_tokens: max_tokens,
        }
    }

    /// Get the number of requests in the batch
    pub fn len(&self) -> usize {
        self.requests.len()
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Get a reference to the requests (for inspection)
    pub fn requests(&self) -> &[BatchRequestItem] {
        &self.requests
    }

    /// Build the batch request
    pub fn build(self) -> MessageBatchCreateRequest {
        MessageBatchCreateRequest {
            requests: self.requests,
        }
    }

    /// Build and validate the batch request
    pub fn build_validated(
        self,
    ) -> Result<MessageBatchCreateRequest, crate::error::AnthropicError> {
        // Use common validation for empty batch
        ValidationUtils::validate_messages_not_empty(self.requests.len(), "Batch")?;

        // Check for duplicate custom IDs
        let mut custom_ids = std::collections::HashSet::new();
        for request in &self.requests {
            if !custom_ids.insert(&request.custom_id) {
                return Err(crate::error::AnthropicError::invalid_input(format!(
                    "Duplicate custom_id found: {}",
                    request.custom_id
                )));
            }
        }

        // Validate individual requests using common utilities
        for request in &self.requests {
            ValidationUtils::validate_messages_not_empty(
                request.params.messages.len(),
                &format!("Request {}", request.custom_id),
            )?;

            ValidationUtils::validate_max_tokens(
                request.params.max_tokens,
                &format!("Request {}", request.custom_id),
            )?;

            // Validate temperature and top_p if present
            if let Some(temp) = request.params.temperature {
                ValidationUtils::validate_temperature(temp).map_err(|e| {
                    crate::error::AnthropicError::invalid_input(format!(
                        "Request {}: {}",
                        request.custom_id, e
                    ))
                })?;
            }

            if let Some(top_p) = request.params.top_p {
                ValidationUtils::validate_top_p(top_p).map_err(|e| {
                    crate::error::AnthropicError::invalid_input(format!(
                        "Request {}: {}",
                        request.custom_id, e
                    ))
                })?;
            }

            // Validate Claude 4 constraints
            ValidationUtils::validate_claude_4_constraints(
                &request.params.model,
                request.params.temperature,
                request.params.top_p,
            )
            .map_err(|e| {
                crate::error::AnthropicError::invalid_input(format!(
                    "Request {}: {}",
                    request.custom_id, e
                ))
            })?;

            // Validate thinking configuration
            if let Some(thinking) = &request.params.thinking {
                ValidationUtils::validate_thinking_config(
                    &request.params.model,
                    thinking.budget_tokens,
                )
                .map_err(|e| {
                    crate::error::AnthropicError::invalid_input(format!(
                        "Request {}: {}",
                        request.custom_id, e
                    ))
                })?;
            }
        }

        Ok(MessageBatchCreateRequest {
            requests: self.requests,
        })
    }
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl From<BatchBuilder> for MessageBatchCreateRequest {
    fn from(builder: BatchBuilder) -> Self {
        builder.build()
    }
}

impl ValidatedBuilder<MessageBatchCreateRequest> for BatchBuilder {
    fn build_validated(self) -> Result<MessageBatchCreateRequest, crate::error::AnthropicError> {
        self.build_validated()
    }
}

impl FluentBuilder for BatchBuilder {
    fn inspect(&self) -> Option<&dyn std::fmt::Debug> {
        Some(self)
    }
}

/// Batch builder with default parameters
#[derive(Debug, Clone)]
pub struct BatchBuilderWithDefaults {
    builder: BatchBuilder,
    default_model: String,
    default_max_tokens: u32,
}

impl BatchBuilderWithDefaults {
    /// Add a simple request using defaults
    pub fn add(mut self, custom_id: impl Into<String>, message: impl Into<String>) -> Self {
        self.builder = self.builder.add_simple_request(
            custom_id,
            &self.default_model,
            message,
            self.default_max_tokens,
        );
        self
    }

    /// Add multiple requests using defaults
    pub fn add_many(
        mut self,
        requests: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        for (custom_id, message) in requests {
            self = self.add(custom_id, message);
        }
        self
    }

    /// Add a Q&A request using defaults
    pub fn add_qa_with_defaults(
        mut self,
        custom_id: impl Into<String>,
        question: impl Into<String>,
        context: Option<impl Into<String>>,
    ) -> Self {
        self.builder = self.builder.add_qa(
            custom_id,
            &self.default_model,
            question,
            context,
            self.default_max_tokens,
        );
        self
    }

    /// Add a creative request using defaults
    pub fn add_creative_with_defaults(
        mut self,
        custom_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        self.builder = self.builder.add_creative(
            custom_id,
            &self.default_model,
            prompt,
            self.default_max_tokens,
        );
        self
    }

    /// Add an analytical request using defaults
    pub fn add_analytical_with_defaults(
        mut self,
        custom_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        self.builder = self.builder.add_analytical(
            custom_id,
            &self.default_model,
            prompt,
            self.default_max_tokens,
        );
        self
    }

    /// Add a code generation request using defaults
    pub fn add_code_generation_with_defaults(
        mut self,
        custom_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        self.builder = self.builder.add_code_generation(
            custom_id,
            &self.default_model,
            prompt,
            self.default_max_tokens,
        );
        self
    }

    /// Add a conversational request using defaults
    pub fn add_conversational_with_defaults(
        mut self,
        custom_id: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        self.builder = self.builder.add_conversational(
            custom_id,
            &self.default_model,
            prompt,
            self.default_max_tokens,
        );
        self
    }

    /// Get the underlying builder
    pub fn into_builder(self) -> BatchBuilder {
        self.builder
    }

    /// Build the batch request
    pub fn build(self) -> MessageBatchCreateRequest {
        self.builder.build()
    }

    /// Build and validate the batch request
    pub fn build_validated(
        self,
    ) -> Result<MessageBatchCreateRequest, crate::error::AnthropicError> {
        self.builder.build_validated()
    }
}

impl ValidatedBuilder<MessageBatchCreateRequest> for BatchBuilderWithDefaults {
    fn build_validated(self) -> Result<MessageBatchCreateRequest, crate::error::AnthropicError> {
        self.build_validated()
    }
}

impl FluentBuilder for BatchBuilderWithDefaults {
    fn inspect(&self) -> Option<&dyn std::fmt::Debug> {
        Some(self)
    }
}
