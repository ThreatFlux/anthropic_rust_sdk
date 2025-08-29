//! Models API implementation

use crate::{
    api::utils::{build_paginated_path, create_default_pagination},
    client::Client,
    error::Result,
    models::model::{Model, ModelListResponse},
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for Models endpoints
#[derive(Clone)]
pub struct ModelsApi {
    client: Client,
}

impl ModelsApi {
    /// Create a new Models API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// List available models
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config, types::Pagination};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let pagination = Pagination::new().with_limit(10);
    ///
    /// let response = client.models().list(Some(pagination), None).await?;
    /// for model in response.data {
    ///     println!("Model: {} - {}", model.id, model.display_name);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<ModelListResponse> {
        let path = build_paginated_path("/models", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific model by ID
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let model = client.models().get("claude-3-5-haiku-20241022", None).await?;
    /// println!("Model: {} - {}", model.id, model.display_name);
    /// println!("Max tokens: {:?}", model.max_tokens);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, model_id: &str, options: Option<RequestOptions>) -> Result<Model> {
        let path = format!("/models/{}", model_id);
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// List all models (convenience method that handles pagination)
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<Model>> {
        let mut all_models = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self.list(Some(pagination), options.clone()).await?;

            all_models.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_models)
    }

    /// Get models by capability (e.g., vision, tool use)
    pub async fn list_by_capability(
        &self,
        capability: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<Model>> {
        let all_models = self.list_all(options).await?;

        Ok(all_models
            .into_iter()
            .filter(|model| {
                model
                    .capabilities
                    .as_ref()
                    .map(|caps| caps.contains(&capability.to_string()))
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Check if a model exists
    pub async fn exists(&self, model_id: &str, options: Option<RequestOptions>) -> bool {
        self.get(model_id, options).await.is_ok()
    }
}
