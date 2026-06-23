//! Managed Agents — Session Threads API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::session::{SessionThread, SessionThreadListResponse},
    models::managed_agents::session_event::SessionEventListResponse,
    streaming::session_event_stream::SessionEventStream,
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Session Threads endpoints
/// (`/v1/sessions/{session_id}/threads`).
#[derive(Clone)]
pub struct SessionThreadsApi {
    client: Client,
    session_id: String,
}

impl SessionThreadsApi {
    /// Create a new Session Threads API client scoped to a session.
    pub fn new(client: Client, session_id: String) -> Self {
        Self { client, session_id }
    }

    /// List threads for the session (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<SessionThreadListResponse> {
        let base = format!("/sessions/{}/threads", self.session_id);
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

    /// Retrieve a thread by id.
    pub async fn get(
        &self,
        thread_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SessionThread> {
        let path = format!("/sessions/{}/threads/{}", self.session_id, thread_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Archive a thread.
    pub async fn archive(
        &self,
        thread_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SessionThread> {
        let path = format!(
            "/sessions/{}/threads/{}/archive",
            self.session_id, thread_id
        );
        self.client
            .request(
                HttpMethod::Post,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List events for a thread (cursor-style pagination).
    pub async fn list_events(
        &self,
        thread_id: &str,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<SessionEventListResponse> {
        let base = format!("/sessions/{}/threads/{}/events", self.session_id, thread_id);
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

    /// Stream events for a thread as Server-Sent Events.
    pub async fn stream_events(
        &self,
        thread_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SessionEventStream> {
        let path = format!(
            "/sessions/{}/threads/{}/events/stream",
            self.session_id, thread_id
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
}
