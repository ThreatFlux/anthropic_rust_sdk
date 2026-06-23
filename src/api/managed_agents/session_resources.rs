//! Managed Agents — Session Resources API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::session::{
        SessionResource, SessionResourceListResponse, SessionResourceSpec,
        SessionResourceUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Session Resources endpoints
/// (`/v1/sessions/{session_id}/resources`).
#[derive(Clone)]
pub struct SessionResourcesApi {
    client: Client,
    session_id: String,
}

impl SessionResourcesApi {
    /// Create a new Session Resources API client scoped to a session.
    pub fn new(client: Client, session_id: String) -> Self {
        Self { client, session_id }
    }

    /// Add a resource to the session.
    pub async fn add(
        &self,
        resource: SessionResourceSpec,
        options: Option<RequestOptions>,
    ) -> Result<SessionResource> {
        let path = format!("/sessions/{}/resources", self.session_id);
        let body = serde_json::to_value(resource)?;
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List resources attached to the session (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<SessionResourceListResponse> {
        let base = format!("/sessions/{}/resources", self.session_id);
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

    /// Retrieve a resource by id.
    pub async fn get(
        &self,
        resource_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<SessionResource> {
        let path = format!("/sessions/{}/resources/{}", self.session_id, resource_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a resource.
    pub async fn update(
        &self,
        resource_id: &str,
        request: SessionResourceUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<SessionResource> {
        let path = format!("/sessions/{}/resources/{}", self.session_id, resource_id);
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

    /// Delete a resource.
    pub async fn delete(&self, resource_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/sessions/{}/resources/{}", self.session_id, resource_id);
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
}
