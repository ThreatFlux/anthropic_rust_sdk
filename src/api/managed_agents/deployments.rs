//! Managed Agents — Deployments & Deployment Runs API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::deployment::{
        Deployment, DeploymentCreateRequest, DeploymentListResponse, DeploymentRun,
        DeploymentRunListResponse, DeploymentUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Deployments endpoints
/// (`/v1/deployments`).
#[derive(Clone)]
pub struct DeploymentsApi {
    client: Client,
}

impl DeploymentsApi {
    /// Create a new Deployments API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a deployment.
    pub async fn create(
        &self,
        request: DeploymentCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Deployment> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/deployments",
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List deployments (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<DeploymentListResponse> {
        let path = build_paginated_path("/deployments", pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve a deployment by id.
    pub async fn get(
        &self,
        deployment_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Deployment> {
        let path = format!("/deployments/{}", deployment_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a deployment.
    pub async fn update(
        &self,
        deployment_id: &str,
        request: DeploymentUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Deployment> {
        let path = format!("/deployments/{}", deployment_id);
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

    /// Delete a deployment.
    pub async fn delete(&self, deployment_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/deployments/{}", deployment_id);
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

    /// Access the runs sub-resource for a deployment.
    pub fn runs(&self, deployment_id: &str) -> DeploymentRunsApi {
        DeploymentRunsApi::new(self.client.clone(), deployment_id.to_string())
    }
}

/// API client for the Managed Agents — Deployment Runs endpoints
/// (`/v1/deployments/{deployment_id}/runs`).
#[derive(Clone)]
pub struct DeploymentRunsApi {
    client: Client,
    deployment_id: String,
}

impl DeploymentRunsApi {
    /// Create a new Deployment Runs API client scoped to a deployment.
    pub fn new(client: Client, deployment_id: String) -> Self {
        Self {
            client,
            deployment_id,
        }
    }

    /// List runs for the deployment (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<DeploymentRunListResponse> {
        let base = format!("/deployments/{}/runs", self.deployment_id);
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

    /// Retrieve a run by id.
    pub async fn get(
        &self,
        run_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<DeploymentRun> {
        let path = format!("/deployments/{}/runs/{}", self.deployment_id, run_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Trigger a manual run of the deployment.
    pub async fn trigger(&self, options: Option<RequestOptions>) -> Result<DeploymentRun> {
        let path = format!("/deployments/{}/runs", self.deployment_id);
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
