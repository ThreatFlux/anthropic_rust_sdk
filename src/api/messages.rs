//! Messages API implementation

use crate::{
    client::Client,
    error::Result,
    models::message::{MessageRequest, MessageResponse, TokenCountRequest, TokenCountResponse},
    streaming::message_stream::MessageStream,
    types::{HttpMethod, RequestOptions},
};
use serde_json::Value;

/// Validate known request content while preserving explicitly supplied future blocks.
pub(crate) fn validate_content(messages: &[crate::models::Message]) -> Result<()> {
    for message in messages {
        for block in &message.content {
            block.checked_replay(
                &message.role,
                crate::models::common::ReplayUnknownPolicy::Preserve,
            )?;
        }
    }
    Ok(())
}

/// API client for Messages endpoints
#[derive(Clone)]
pub struct MessagesApi {
    client: Client,
}

impl MessagesApi {
    /// Create a new Messages API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Serialize a message request and move the profile attribution into the
    /// header required by Anthropic's Messages API.
    fn prepare_request(
        mut request: MessageRequest,
        options: Option<RequestOptions>,
    ) -> Result<(Value, Option<RequestOptions>)> {
        validate_content(&request.messages)?;
        let user_profile_id = request.user_profile_id.take();
        let mut options = options.unwrap_or_default();
        if let Some(user_profile_id) = user_profile_id {
            options
                .headers
                .insert("anthropic-user-profile-id".to_string(), user_profile_id);
        }
        Ok((serde_json::to_value(request)?, Some(options)))
    }

    fn prepare_token_count_request(
        mut request: TokenCountRequest,
        options: Option<RequestOptions>,
    ) -> Result<(Value, Option<RequestOptions>)> {
        validate_content(&request.messages)?;
        let user_profile_id = request.user_profile_id.take();
        let mut options = options.unwrap_or_default();
        if let Some(user_profile_id) = user_profile_id {
            options
                .headers
                .insert("anthropic-user-profile-id".to_string(), user_profile_id);
        }
        Ok((serde_json::to_value(request)?, Some(options)))
    }

    /// Create a message
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config, models::message::MessageRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = MessageRequest::new()
    ///     .model("claude-haiku-4-5")
    ///     .max_tokens(1000)
    ///     .add_user_message("Hello, Claude!");
    ///
    /// let response = client.messages().create(request, None).await?;
    /// println!("Response: {:?}", response);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        request: MessageRequest,
        options: Option<RequestOptions>,
    ) -> Result<MessageResponse> {
        let (body, options) = Self::prepare_request(request, options)?;
        self.client
            .request(HttpMethod::Post, "/messages", Some(body), options)
            .await
    }

    /// Create a streaming message
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config, models::message::MessageRequest};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = MessageRequest::new()
    ///     .model("claude-haiku-4-5")
    ///     .max_tokens(1000)
    ///     .add_user_message("Hello, Claude!")
    ///     .stream(true);
    ///
    /// let mut stream = client.messages().create_stream(request, None).await?;
    /// while let Some(event) = stream.next().await {
    ///     match event {
    ///         Ok(event) => println!("Event: {:?}", event),
    ///         Err(e) => eprintln!("Error: {}", e),
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create_stream(
        &self,
        request: MessageRequest,
        options: Option<RequestOptions>,
    ) -> Result<MessageStream> {
        self.create_stream_with_limits(request, crate::streaming::StreamLimits::default(), options)
            .await
    }

    /// Create a message stream with explicit frame, tool-input, block and queue limits.
    /// Dropping the returned stream cancels its HTTP producer.
    pub async fn create_stream_with_limits(
        &self,
        mut request: MessageRequest,
        limits: crate::streaming::StreamLimits,
        options: Option<RequestOptions>,
    ) -> Result<MessageStream> {
        limits.validate()?;
        // Ensure streaming is enabled
        request.stream = Some(true);

        let (body, options) = Self::prepare_request(request, options)?;
        let response = self
            .client
            .request_stream(HttpMethod::Post, "/messages", Some(body), options)
            .await?;

        MessageStream::new_with_limits(response, limits).await
    }

    /// Count tokens in a message
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config, models::message::TokenCountRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = TokenCountRequest::new()
    ///     .model("claude-haiku-4-5")
    ///     .add_user_message("Hello, Claude!");
    ///
    /// let response = client.messages().count_tokens(request, None).await?;
    /// println!("Token count: {}", response.input_tokens);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count_tokens(
        &self,
        request: TokenCountRequest,
        options: Option<RequestOptions>,
    ) -> Result<TokenCountResponse> {
        let (body, options) = Self::prepare_token_count_request(request, options)?;
        self.client
            .request(
                HttpMethod::Post,
                "/messages/count_tokens",
                Some(body),
                options,
            )
            .await
    }

    /// Count tokens for a simple text message (convenience method)
    pub async fn count_tokens_simple(
        &self,
        model: &str,
        text: &str,
        options: Option<RequestOptions>,
    ) -> Result<TokenCountResponse> {
        let request = TokenCountRequest::new().model(model).add_user_message(text);

        self.count_tokens(request, options).await
    }
}
