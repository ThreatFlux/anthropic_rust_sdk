//! Models API implementation

use crate::{
    api::utils::{build_paginated_path, id_cursor, paginate, TraversalPage},
    client::Client,
    error::Result,
    models::model::{Model, ModelListResponse},
    types::{HttpMethod, PageStream, Pagination, PaginationLimits, RequestOptions},
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
    /// use threatflux_anthropic_sdk::{Client, Config, types::Pagination};
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
        if let Some(pagination) = &pagination {
            pagination.validate()?;
        }
        let path = build_paginated_path("/models", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific model by ID
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let model = client.models().get("claude-haiku-4-5", None).await?;
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
        self.list_all_with_limits(
            Pagination::new().with_limit(100),
            PaginationLimits::default(),
            options,
        )
        .await
    }

    /// Lazily traverse ID-cursor pages; reverse traversal uses `first_id`/`before`.
    pub fn pages(
        &self,
        mut pagination: Pagination,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<Model>> {
        pagination.validate()?;
        let reverse = pagination.before.is_some();
        let initial = if reverse {
            pagination.before.take()
        } else {
            pagination.after.take()
        };
        let api = self.clone();
        paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let mut pagination = pagination.clone();
            let options = options.clone();
            if reverse {
                pagination.before = cursor;
            } else {
                pagination.after = cursor;
            }
            async move {
                let response = api.list(Some(pagination), options).await?;
                let cursor = if reverse {
                    response.first_id
                } else {
                    response.last_id
                };
                let next_cursor = id_cursor(response.has_more, cursor)?;
                let item_ids = response.data.iter().map(|model| model.id.clone()).collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor,
                    item_ids,
                })
            }
        })
    }

    /// Collect all models subject to explicit finite traversal ceilings.
    pub async fn list_all_with_limits(
        &self,
        pagination: Pagination,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<Model>> {
        self.pages(pagination, limits, options)?
            .collect_items()
            .await
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
                    .map(|caps| caps.contains(capability))
                    .unwrap_or(false)
            })
            .collect())
    }

    /// Check if a model exists
    pub async fn exists(&self, model_id: &str, options: Option<RequestOptions>) -> bool {
        self.get(model_id, options).await.is_ok()
    }
}
