//! API Keys Admin API implementation

use crate::{
    api::utils::build_path_with_query,
    client::Client,
    error::{AnthropicError, Result},
    models::admin::{
        ApiKey, ApiKeyCreateRequest, ApiKeyListParams, ApiKeyListResponse, ApiKeyUpdateRequest,
    },
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
        let mut params = ApiKeyListParams::new();
        if let Some(workspace_id) = workspace_id {
            params = params.with_workspace_id(workspace_id);
        }
        if let Some(pagination) = pagination {
            if let Some(limit) = pagination.limit {
                params = params.with_limit(limit);
            }
            if let Some(after) = pagination.after {
                params = params.with_after_id(after);
            }
            if let Some(before) = pagination.before {
                params = params.with_before_id(before);
            }
        }

        self.list_with_params(params, options).await
    }

    /// List API keys with full Admin API filters.
    pub async fn list_with_params(
        &self,
        params: ApiKeyListParams,
        options: Option<RequestOptions>,
    ) -> Result<ApiKeyListResponse> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }
        if let Some(after_id) = params.after_id {
            query.push(format!("after_id={}", after_id));
        }
        if let Some(before_id) = params.before_id {
            query.push(format!("before_id={}", before_id));
        }
        if let Some(workspace_id) = params.workspace_id {
            query.push(format!("workspace_id={}", workspace_id));
        }
        if let Some(status) = params.status {
            query.push(format!("status={}", status));
        }
        if let Some(created_by_user_id) = params.created_by_user_id {
            query.push(format!("created_by_user_id={}", created_by_user_id));
        }

        let path = build_path_with_query("/organizations/api_keys", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific API key
    pub async fn get(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let _ = workspace_id;
        let path = format!("/organizations/api_keys/{}", api_key_id);

        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Create a new API key.
    ///
    /// The current Admin API does not support API-key creation programmatically.
    pub async fn create(
        &self,
        _request: ApiKeyCreateRequest,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let _ = workspace_id;
        let _ = options;
        Err(AnthropicError::invalid_input(
            "Creating Admin API keys via API is not supported by the current Anthropic Admin API",
        ))
    }

    /// Update an API key
    pub async fn update(
        &self,
        api_key_id: &str,
        request: ApiKeyUpdateRequest,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let _ = workspace_id;
        let path = format!("/organizations/api_keys/{}", api_key_id);

        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(HttpMethod::Post, &path, Some(body), options)
            .await
    }

    /// Rotate an API key.
    ///
    /// Not currently supported by the public Admin API.
    pub async fn rotate(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<ApiKey> {
        let _ = api_key_id;
        let _ = workspace_id;
        let _ = options;
        Err(AnthropicError::invalid_input(
            "Rotating Admin API keys via API is not supported by the current Anthropic Admin API",
        ))
    }

    /// Delete an API key.
    ///
    /// Not currently supported by the public Admin API.
    pub async fn delete(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<()> {
        let _ = api_key_id;
        let _ = workspace_id;
        let _ = options;
        Err(AnthropicError::invalid_input(
            "Deleting Admin API keys via API is not supported by the current Anthropic Admin API",
        ))
    }

    /// List all API keys (convenience method)
    pub async fn list_all(
        &self,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<ApiKey>> {
        let mut all_keys = Vec::new();
        let mut after_id: Option<String> = None;

        loop {
            let mut params = ApiKeyListParams::new().with_limit(100);
            if let Some(workspace_id) = workspace_id {
                params = params.with_workspace_id(workspace_id);
            }
            if let Some(after_id_value) = &after_id {
                params = params.with_after_id(after_id_value.clone());
            }

            let response = self.list_with_params(params, options.clone()).await?;

            all_keys.extend(response.data);

            if !response.has_more {
                break;
            }

            after_id = response.last_id;
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
