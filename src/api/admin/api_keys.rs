//! API Keys Admin API implementation

use crate::{
    api::utils::{build_paginated_path, create_default_pagination},
    client::Client,
    error::Result,
    models::admin::{ApiKey, ApiKeyCreateRequest, ApiKeyListResponse, ApiKeyUpdateRequest},
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for API Keys admin endpoints
#[derive(Clone)]
pub struct ApiKeysApi {
    client: Client,
}

impl ApiKeysApi {
    /// Create a new API Keys API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// List API keys
    pub async fn list(
        &self,
        workspace_id: Option<&str>,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKeyListResponse> {
        let base_path = if let Some(workspace_id) = workspace_id {
            format!("/organization/workspaces/{}/api_keys", workspace_id)
        } else {
            "/organization/api_keys".to_string()
        };

        let path = build_paginated_path(&base_path, pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific API key
    pub async fn get(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let path = if let Some(workspace_id) = workspace_id {
            format!(
                "/organization/workspaces/{}/api_keys/{}",
                workspace_id, api_key_id
            )
        } else {
            format!("/organization/api_keys/{}", api_key_id)
        };

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Create a new API key
    pub async fn create(
        &self,
        request: ApiKeyCreateRequest,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let path = if let Some(workspace_id) = workspace_id {
            format!("/organization/workspaces/{}/api_keys", workspace_id)
        } else {
            "/organization/api_keys".to_string()
        };

        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Post, &path, Some(body), options)
            .await
    }

    /// Update an API key
    pub async fn update(
        &self,
        api_key_id: &str,
        request: ApiKeyUpdateRequest,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let path = if let Some(workspace_id) = workspace_id {
            format!(
                "/organization/workspaces/{}/api_keys/{}",
                workspace_id, api_key_id
            )
        } else {
            format!("/organization/api_keys/{}", api_key_id)
        };

        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Patch, &path, Some(body), options)
            .await
    }

    /// Rotate an API key
    pub async fn rotate(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let path = if let Some(workspace_id) = workspace_id {
            format!(
                "/organization/workspaces/{}/api_keys/{}/rotate",
                workspace_id, api_key_id
            )
        } else {
            format!("/organization/api_keys/{}/rotate", api_key_id)
        };

        self.client
            .request(HttpMethod::Post, &path, None, options)
            .await
    }

    /// Delete an API key
    pub async fn delete(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<()> {
        let path = if let Some(workspace_id) = workspace_id {
            format!(
                "/organization/workspaces/{}/api_keys/{}",
                workspace_id, api_key_id
            )
        } else {
            format!("/organization/api_keys/{}", api_key_id)
        };

        let _: serde_json::Value = self
            .client
            .request(HttpMethod::Delete, &path, None, options)
            .await?;
        Ok(())
    }

    /// List all API keys (convenience method)
    pub async fn list_all(
        &self,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<ApiKey>> {
        let mut all_keys = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self
                .list(workspace_id, Some(pagination), options.clone())
                .await?;

            all_keys.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_keys)
    }

    /// List API keys by status
    pub async fn list_by_status(
        &self,
        status: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<ApiKey>> {
        let all_keys = self.list_all(workspace_id, options).await?;

        Ok(all_keys
            .into_iter()
            .filter(|key| key.status.as_deref() == Some(status))
            .collect())
    }
}
