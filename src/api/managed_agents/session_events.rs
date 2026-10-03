//! Managed Agents — Session Events API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::{build_paginated_path, build_query_path},
    client::Client,
    error::Result,
    models::managed_agents::session_event::{SendEvent, SessionEvent, SessionEventListResponse},
    streaming::session_event_stream::SessionEventStream,
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Session Events endpoints
/// (`/v1/sessions/{session_id}/events`).
#[derive(Clone)]
pub struct SessionEventsApi {
    client: Client,
    session_id: String,
}

impl SessionEventsApi {
    /// Create a new Session Events API client scoped to a session.
    pub fn new(client: Client, session_id: String) -> Self {
        Self { client, session_id }
    }

    /// List events for the session (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<SessionEventListResponse> {
        let base = format!("/sessions/{}/events", self.session_id);
        let path = build_paginated_path(&base, pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Send a client-originated event to the session.
    ///
    /// Known user-message blocks are validated against the Managed Agents
    /// surface, which accepts text, images, documents, and redacted placeholders.
    /// Explicit raw future types remain unchanged, with service support unknown.
    pub async fn send(
        &self,
        event: SendEvent,
        options: Option<RequestOptions>,
    ) -> Result<SessionEvent> {
        let path = format!("/sessions/{}/events", self.session_id);
        validate_send_event(&event)?;
        let body = serde_json::to_value(event)?;
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Stream session events as Server-Sent Events.
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::Client;
    /// use futures::StreamExt;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let mut stream = client.sessions().events("session_123").stream(None).await?;
    /// while let Some(event) = stream.next().await {
    ///     let event = event?;
    ///     println!("event: {:?}", event);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn stream(&self, options: Option<RequestOptions>) -> Result<SessionEventStream> {
        self.stream_with_event_deltas(&[], options).await
    }

    /// Stream session events with optional incremental event-delta payloads.
    pub async fn stream_with_event_deltas(
        &self,
        event_deltas: &[String],
        options: Option<RequestOptions>,
    ) -> Result<SessionEventStream> {
        let query = event_deltas
            .iter()
            .map(|delta| ("event_deltas[]".to_string(), delta.clone()))
            .collect();
        let path = build_query_path(
            &format!("/sessions/{}/events/stream", self.session_id),
            query,
        );
        let response = self
            .client
            .request_stream(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await?;
        SessionEventStream::new(response).await
    }

    /// Send a user text message.
    pub async fn send_user_message(
        &self,
        text: impl Into<String>,
        options: Option<RequestOptions>,
    ) -> Result<SessionEvent> {
        self.send(SendEvent::user_text(text), options).await
    }

    /// Interrupt the running agent.
    pub async fn interrupt(&self, options: Option<RequestOptions>) -> Result<SessionEvent> {
        self.send(SendEvent::interrupt(), options).await
    }

    /// Confirm or reject a pending tool use.
    pub async fn confirm_tool(
        &self,
        tool_use_id: &str,
        approve: bool,
        options: Option<RequestOptions>,
    ) -> Result<SessionEvent> {
        self.send(SendEvent::confirm_tool(tool_use_id, approve), options)
            .await
    }

    /// Provide a custom tool result for a pending tool use.
    pub async fn custom_tool_result(
        &self,
        tool_use_id: &str,
        content: serde_json::Value,
        options: Option<RequestOptions>,
    ) -> Result<SessionEvent> {
        let event = SendEvent::UserCustomToolResult {
            tool_use_id: tool_use_id.to_string(),
            content,
        };
        self.send(event, options).await
    }

    /// Define a session outcome.
    pub async fn define_outcome(
        &self,
        outcome: serde_json::Value,
        options: Option<RequestOptions>,
    ) -> Result<SessionEvent> {
        let event = SendEvent::UserDefineOutcome { outcome };
        self.send(event, options).await
    }

    /// Inject a system message mid-session.
    pub async fn system_message(
        &self,
        text: impl Into<String>,
        options: Option<RequestOptions>,
    ) -> Result<SessionEvent> {
        self.send(SendEvent::system(text), options).await
    }
}

/// Schema verified 2026-10-03 against Anthropic SDK revision
/// 18f25547f20cf5f01da69ac611e700e3bc9ebf21, sessions/user-message params.
fn validate_send_event(event: &SendEvent) -> Result<()> {
    use crate::models::common::{ContentBlock, DocumentSource};
    if let SendEvent::UserMessage { content } = event {
        for block in content {
            // Check typed field collisions and malformed known constructors
            // without applying the broader Messages role union to sessions.
            let wire = serde_json::to_value(block)?;
            let checked = ContentBlock::raw(wire)?;
            match checked {
                ContentBlock::Text { .. }
                | ContentBlock::Image { .. }
                | ContentBlock::Unknown(_) => {}
                ContentBlock::Document {
                    source: DocumentSource::Content { .. },
                    ..
                } => {
                    return Err(crate::AnthropicError::invalid_input("Managed Agents user messages do not accept inline document content sources"));
                }
                ContentBlock::Document { .. } => {}
                _ => {
                    return Err(crate::AnthropicError::invalid_input(
                        "unsupported known block in Managed Agents user message",
                    ))
                }
            }
        }
    }
    Ok(())
}

/// Validate a raw initial event without rewriting its optional/unknown fields.
pub(super) fn validate_initial_event(event: &serde_json::Value) -> Result<()> {
    crate::models::common::RawContentBlock::new(event.clone())?;
    if event.get("type").and_then(serde_json::Value::as_str) == Some("user.message") {
        let typed: SendEvent = serde_json::from_value(event.clone()).map_err(|_| {
            crate::AnthropicError::invalid_input("malformed Managed Agents initial user message")
        })?;
        validate_send_event(&typed)?;
    }
    Ok(())
}
