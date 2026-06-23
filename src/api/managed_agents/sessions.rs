//! Managed Agents — Sessions API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::{
        session_events::SessionEventsApi, session_resources::SessionResourcesApi,
        session_threads::SessionThreadsApi, with_managed_agents_beta,
    },
    client::Client,
    error::Result,
    models::managed_agents::session::{Session, SessionCreateRequest, SessionUpdateRequest},
    types::{HttpMethod, RequestOptions},
};

/// API client for the Managed Agents — Sessions endpoints (`/v1/sessions`).
#[derive(Clone)]
pub struct SessionsApi {
    client: Client,
}

impl SessionsApi {
    /// Create a new Sessions API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a session.
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, models::managed_agents::SessionCreateRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = SessionCreateRequest::new("agent_123").title("triage run");
    ///
    /// let session = client.sessions().create(request, None).await?;
    /// println!("Created session: {} ({:?})", session.id, session.status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        request: SessionCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Session> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/sessions",
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve a session by id.
    pub async fn get(&self, session_id: &str, options: Option<RequestOptions>) -> Result<Session> {
        let path = format!("/sessions/{}", session_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a session.
    pub async fn update(
        &self,
        session_id: &str,
        request: SessionUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Session> {
        let path = format!("/sessions/{}", session_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Delete a session.
    pub async fn delete(&self, session_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/sessions/{}", session_id);
        let _: serde_json::Value = self
            .client
            .request(
                HttpMethod::Delete,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await?;
        Ok(())
    }

    /// Archive a session.
    pub async fn archive(
        &self,
        session_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Session> {
        let path = format!("/sessions/{}/archive", session_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Access the events sub-resource for a session.
    pub fn events(&self, session_id: &str) -> SessionEventsApi {
        SessionEventsApi::new(self.client.clone(), session_id.to_string())
    }

    /// Access the resources sub-resource for a session.
    pub fn resources(&self, session_id: &str) -> SessionResourcesApi {
        SessionResourcesApi::new(self.client.clone(), session_id.to_string())
    }

    /// Access the threads sub-resource for a session (multiagent).
    pub fn threads(&self, session_id: &str) -> SessionThreadsApi {
        SessionThreadsApi::new(self.client.clone(), session_id.to_string())
    }

    /// Poll [`get`](Self::get) until the session reaches a resting state
    /// (idle or terminated), mirroring `MessageBatchesApi::wait_for_completion`.
    pub async fn wait_until_idle(
        &self,
        session_id: &str,
        poll_interval: std::time::Duration,
        max_wait: std::time::Duration,
    ) -> Result<Session> {
        let start_time = std::time::Instant::now();

        loop {
            let session = self.get(session_id, None).await?;

            if session.is_resting() {
                return Ok(session);
            }

            if start_time.elapsed() >= max_wait {
                return Err(crate::error::AnthropicError::invalid_input(format!(
                    "Session {} did not reach a resting state within timeout",
                    session_id
                )));
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}
