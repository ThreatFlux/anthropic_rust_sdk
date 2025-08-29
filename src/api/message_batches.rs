//! Message Batches API implementation

use crate::{
    api::utils::{build_paginated_path, create_default_pagination},
    client::Client,
    error::Result,
    models::batch::{
        MessageBatch, MessageBatchCreateRequest, MessageBatchListResponse, MessageBatchStatus,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for Message Batches endpoints
#[derive(Clone)]
pub struct MessageBatchesApi {
    client: Client,
}

impl MessageBatchesApi {
    /// Create a new Message Batches API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a message batch
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config, models::batch::MessageBatchCreateRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = MessageBatchCreateRequest::new()
    ///     .add_request("req_1", "claude-3-5-haiku-20241022", "Hello, Claude!", 1000);
    ///
    /// let batch = client.message_batches().create(request, None).await?;
    /// println!("Created batch: {}", batch.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        request: MessageBatchCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatch> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Post, "/messages/batches", Some(body), options)
            .await
    }

    /// Retrieve a message batch
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let batch = client.message_batches().retrieve("batch_123", None).await?;
    /// println!("Batch status: {:?}", batch.processing_status);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn retrieve(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatch> {
        let path = format!("/messages/batches/{}", batch_id);
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// List message batches
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config, types::Pagination};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let pagination = Pagination::new().with_limit(20);
    ///
    /// let response = client.message_batches().list(Some(pagination), None).await?;
    /// for batch in response.data {
    ///     println!("Batch: {} - Status: {:?}", batch.id, batch.processing_status);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatchListResponse> {
        let path = build_paginated_path("/messages/batches", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Cancel a message batch
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let batch = client.message_batches().cancel("batch_123", None).await?;
    /// println!("Cancelled batch: {}", batch.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cancel(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<MessageBatch> {
        let path = format!("/messages/batches/{}/cancel", batch_id);
        self.client
            .request(HttpMethod::Post, &path, None, options)
            .await
    }

    /// Delete a message batch
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// client.message_batches().delete("batch_123", None).await?;
    /// println!("Deleted batch");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, batch_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/messages/batches/{}", batch_id);
        let _: serde_json::Value = self
            .client
            .request(HttpMethod::Delete, &path, None, options)
            .await?;
        Ok(())
    }

    /// Wait for a batch to complete processing
    pub async fn wait_for_completion(
        &self,
        batch_id: &str,
        poll_interval: std::time::Duration,
        max_wait: std::time::Duration,
    ) -> Result<MessageBatch> {
        let start_time = std::time::Instant::now();

        loop {
            let batch = self.retrieve(batch_id, None).await?;

            match batch.processing_status {
                MessageBatchStatus::Completed
                | MessageBatchStatus::Failed
                | MessageBatchStatus::Cancelled => {
                    return Ok(batch);
                }
                _ => {
                    if start_time.elapsed() >= max_wait {
                        return Err(crate::error::AnthropicError::invalid_input(format!(
                            "Batch {} did not complete within timeout",
                            batch_id
                        )));
                    }

                    tokio::time::sleep(poll_interval).await;
                }
            }
        }
    }

    /// List batches by status
    pub async fn list_by_status(
        &self,
        status: MessageBatchStatus,
        options: Option<RequestOptions>,
    ) -> Result<Vec<MessageBatch>> {
        // This would typically involve API filtering, but for now we'll filter client-side
        let pagination = create_default_pagination(None);
        let response = self.list(Some(pagination), options).await?;

        Ok(response
            .data
            .into_iter()
            .filter(|batch| batch.processing_status == status)
            .collect())
    }
}
