//! Message Batches API implementation

use crate::{
    api::utils::{build_paginated_path, id_cursor, paginate, TraversalPage},
    client::Client,
    error::Result,
    models::batch::{
        MessageBatch, MessageBatchCreateRequest, MessageBatchListResponse, MessageBatchResultEntry,
        MessageBatchStatus,
    },
    types::{HttpMethod, PageStream, Pagination, PaginationLimits, RequestOptions},
};
use futures::{Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

/// Memory ceiling for one JSONL result row, excluding its newline.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct BatchResultsStreamOptions {
    /// Maximum row size in bytes. Defaults to 8 MiB.
    pub max_row_bytes: usize,
}

impl BatchResultsStreamOptions {
    /// Construct a positive maximum row-byte ceiling.
    pub fn new(max_row_bytes: usize) -> Result<Self> {
        if max_row_bytes == 0 {
            return Err(crate::error::AnthropicError::invalid_input(
                "Batch result row limit must be positive",
            ));
        }
        Ok(Self { max_row_bytes })
    }
}

impl Default for BatchResultsStreamOptions {
    fn default() -> Self {
        Self {
            max_row_bytes: 8 * 1024 * 1024,
        }
    }
}

/// Incremental batch results. Dropping releases the response and cancels further reads.
pub struct BatchResultsStream {
    inner: Pin<Box<dyn Stream<Item = Result<MessageBatchResultEntry>> + Send>>,
}

impl Stream for BatchResultsStream {
    type Item = Result<MessageBatchResultEntry>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

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
    /// use threatflux_anthropic_sdk::{Client, Config, models::batch::MessageBatchCreateRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = MessageBatchCreateRequest::new()
    ///     .add_request("req_1", "claude-haiku-4-5", "Hello, Claude!", 1000);
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
        for item in &request.requests {
            crate::api::messages::validate_content(&item.params.messages)?;
        }
        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Post, "/messages/batches", Some(body), options)
            .await
    }

    /// Retrieve a message batch
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
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
    /// use threatflux_anthropic_sdk::{Client, Config, types::Pagination};
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
        if let Some(pagination) = &pagination {
            pagination.validate()?;
        }
        let path = build_paginated_path("/messages/batches", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Cancel a message batch
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
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
    /// use threatflux_anthropic_sdk::{Client, Config};
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

    /// Retrieve raw batch results (JSONL) for a completed batch.
    ///
    /// This hits `/messages/batches/{batch_id}/results` and returns the raw bytes.
    pub async fn results_raw(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<u8>> {
        let path = format!("/messages/batches/{}/results", batch_id);
        let response = self
            .client
            .request_stream(HttpMethod::Get, &path, None, options)
            .await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::error::AnthropicError::api_error(
                status.as_u16(),
                error_text,
                None,
            ));
        }

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Retrieve batch results as UTF-8 text (JSONL).
    pub async fn results_text(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<String> {
        let bytes = self.results_raw(batch_id, options).await?;
        String::from_utf8(bytes).map_err(|e| {
            crate::error::AnthropicError::invalid_input(format!(
                "Batch results are not valid UTF-8: {}",
                e
            ))
        })
    }

    /// Retrieve and parse batch results into structured entries.
    ///
    /// The endpoint returns one JSON object per line.
    pub async fn results(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<MessageBatchResultEntry>> {
        let mut stream = self.results_stream(batch_id, options).await?;
        let mut parsed = Vec::new();
        while let Some(entry) = stream.next().await {
            parsed.push(entry?);
        }
        Ok(parsed)
    }

    /// Stream parsed JSONL rows without buffering the complete HTTP response.
    pub async fn results_stream(
        &self,
        batch_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<BatchResultsStream> {
        self.results_stream_with_options(batch_id, BatchResultsStreamOptions::default(), options)
            .await
    }

    /// Stream rows with an explicit, positive memory ceiling.
    pub async fn results_stream_with_options(
        &self,
        batch_id: &str,
        limits: BatchResultsStreamOptions,
        options: Option<RequestOptions>,
    ) -> Result<BatchResultsStream> {
        if limits.max_row_bytes == 0 {
            return Err(crate::error::AnthropicError::invalid_input(
                "Batch result row limit must be positive",
            ));
        }
        let response = self
            .client
            .request_stream(
                HttpMethod::Get,
                &format!("/messages/batches/{batch_id}/results"),
                None,
                options,
            )
            .await?;
        if !response.status().is_success() {
            return Err(crate::error::AnthropicError::api_error(
                response.status().as_u16(),
                "Batch results request failed".to_string(),
                None,
            ));
        }
        // Retain one transport chunk and one row; parse UTF-8 only after a whole row.
        let source = Box::pin(response.bytes_stream());
        let state = (
            source,
            Vec::<u8>::new(),
            Vec::<u8>::new(),
            0_usize,
            1_usize,
            false,
        );
        let stream = futures::stream::try_unfold(state, move |state| async move {
            let (mut source, mut chunk, mut row, mut offset, mut line, mut eof) = state;
            loop {
                while offset < chunk.len() {
                    let remaining = &chunk[offset..];
                    let end = remaining.iter().position(|byte| *byte == b'\n');
                    let bytes = end.unwrap_or(remaining.len());
                    if row.len().saturating_add(bytes) > limits.max_row_bytes {
                        return Err(crate::error::AnthropicError::json(format!(
                            "Batch result row {line} exceeds the byte limit"
                        )));
                    }
                    row.extend_from_slice(&remaining[..bytes]);
                    offset += bytes;
                    if end.is_some() {
                        offset += 1;
                        let row_number = line;
                        line += 1;
                        if row.iter().all(u8::is_ascii_whitespace) {
                            row.clear();
                            continue;
                        }
                        let value = serde_json::from_slice(&row).map_err(|_| {
                            crate::error::AnthropicError::json(format!(
                                "Invalid JSON or UTF-8 in batch result row {row_number}"
                            ))
                        })?;
                        row.clear();
                        return Ok(Some((value, (source, chunk, row, offset, line, eof))));
                    }
                }
                if eof {
                    if row.is_empty() || row.iter().all(u8::is_ascii_whitespace) {
                        return Ok(None);
                    }
                    let value = serde_json::from_slice(&row).map_err(|_| {
                        crate::error::AnthropicError::json(format!(
                            "Invalid JSON or UTF-8 in batch result row {line}"
                        ))
                    })?;
                    row.clear();
                    return Ok(Some((value, (source, chunk, row, offset, line + 1, eof))));
                }
                match source.next().await {
                    Some(Ok(bytes)) => {
                        chunk = bytes.to_vec();
                        offset = 0;
                    }
                    Some(Err(_)) => {
                        return Err(crate::error::AnthropicError::stream(format!(
                            "Transport failure reading batch result row {line}"
                        )))
                    }
                    None => {
                        eof = true;
                        chunk.clear();
                        offset = 0;
                    }
                }
            }
        });
        Ok(BatchResultsStream {
            inner: Box::pin(stream),
        })
    }

    /// Lazily traverse batch pages with finite limits.
    pub fn pages(
        &self,
        mut pagination: Pagination,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<MessageBatch>> {
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
                let item_ids = response.data.iter().map(|batch| batch.id.clone()).collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor,
                    item_ids,
                })
            }
        })
    }

    /// Collect all batches with the finite default traversal ceilings.
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<MessageBatch>> {
        self.list_all_with_limits(
            Pagination::new().with_limit(100),
            PaginationLimits::default(),
            options,
        )
        .await
    }

    /// Collect all batches, failing rather than truncating at an explicit limit.
    pub async fn list_all_with_limits(
        &self,
        pagination: Pagination,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<MessageBatch>> {
        self.pages(pagination, limits, options)?
            .collect_items()
            .await
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
        let batches = self.list_all(options).await?;

        Ok(batches
            .into_iter()
            .filter(|batch| batch.processing_status == status)
            .collect())
    }
}
