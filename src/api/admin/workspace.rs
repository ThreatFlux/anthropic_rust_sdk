//! Workspace Admin API implementation

use crate::{
    api::utils::{build_paginated_path, create_default_pagination},
    client::Client,
    error::Result,
    models::admin::{
        Workspace, WorkspaceCreateRequest, WorkspaceListResponse, WorkspaceUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for Workspace admin endpoints
#[derive(Clone)]
pub struct WorkspaceApi {
    client: Client,
}

impl WorkspaceApi {
    /// Create a new Workspace API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// List workspaces
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceListResponse> {
        let path = build_paginated_path("/organization/workspaces", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific workspace
    pub async fn get(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organization/workspaces/{}", workspace_id);
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Create a new workspace
    pub async fn create(
        &self,
        request: WorkspaceCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/organization/workspaces",
                Some(body),
                options,
            )
            .await
    }

    /// Update a workspace
    pub async fn update(
        &self,
        workspace_id: &str,
        request: WorkspaceUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organization/workspaces/{}", workspace_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Patch, &path, Some(body), options)
            .await
    }

    /// Delete a workspace
    pub async fn delete(&self, workspace_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/organization/workspaces/{}", workspace_id);
        let _: serde_json::Value = self
            .client
            .request(HttpMethod::Delete, &path, None, options)
            .await?;
        Ok(())
    }

    /// Archive a workspace
    pub async fn archive(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organization/workspaces/{}/archive", workspace_id);
        self.client
            .request(HttpMethod::Post, &path, None, options)
            .await
    }

    /// Restore an archived workspace
    pub async fn restore(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organization/workspaces/{}/restore", workspace_id);
        self.client
            .request(HttpMethod::Post, &path, None, options)
            .await
    }

    /// List all workspaces (convenience method)
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<Workspace>> {
        let mut all_workspaces = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self.list(Some(pagination), options.clone()).await?;

            all_workspaces.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_workspaces)
    }
}
