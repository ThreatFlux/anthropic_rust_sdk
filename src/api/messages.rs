//! Messages API implementation

use crate::{
    client::Client,
    error::Result,
    models::message::{MessageRequest, MessageResponse, TokenCountRequest, TokenCountResponse},
    streaming::message_stream::MessageStream,
    types::{HttpMethod, RequestOptions},
};

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

    /// Create a message
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config, models::message::MessageRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = MessageRequest::new()
    ///     .model("claude-3-5-haiku-20241022")
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
        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Post, "/messages", Some(body), options)
            .await
    }

    /// Create a streaming message
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config, models::message::MessageRequest};
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = MessageRequest::new()
    ///     .model("claude-3-5-haiku-20241022")
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
        mut request: MessageRequest,
        options: Option<RequestOptions>,
    ) -> Result<MessageStream> {
        // Ensure streaming is enabled
        request.stream = Some(true);

        let body = serde_json::to_value(request)?;
        let response = self
            .client
            .request_stream(HttpMethod::Post, "/messages", Some(body), options)
            .await?;

        MessageStream::new(response).await
    }

    /// Count tokens in a message
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config, models::message::TokenCountRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = TokenCountRequest::new()
    ///     .model("claude-3-5-haiku-20241022")
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
        let body = serde_json::to_value(request)?;
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
