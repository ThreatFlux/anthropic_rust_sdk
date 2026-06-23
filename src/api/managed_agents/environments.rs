//! Managed Agents — Environments API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::environment::{
        Environment, EnvironmentCreateRequest, EnvironmentListResponse, EnvironmentUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Environments endpoints (`/v1/environments`).
#[derive(Clone)]
pub struct EnvironmentsApi {
    client: Client,
}

impl EnvironmentsApi {
    /// Create a new Environments API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create an environment.
    pub async fn create(
        &self,
        request: EnvironmentCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Environment> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/environments",
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List environments (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<EnvironmentListResponse> {
        let path = build_paginated_path("/environments", pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve an environment by id.
    pub async fn get(
        &self,
        environment_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Environment> {
        let path = format!("/environments/{}", environment_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update an environment.
    pub async fn update(
        &self,
        environment_id: &str,
        request: EnvironmentUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Environment> {
        let path = format!("/environments/{}", environment_id);
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

    /// Delete an environment.
    pub async fn delete(
        &self,
        environment_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<()> {
        let path = format!("/environments/{}", environment_id);
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

    /// Archive an environment.
    pub async fn archive(
        &self,
        environment_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Environment> {
        let path = format!("/environments/{}/archive", environment_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }
}
