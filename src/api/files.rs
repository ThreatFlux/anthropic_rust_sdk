//! Files API implementation

use crate::{
    api::utils::{build_path_with_query, paginate, TraversalPage},
    client::Client,
    error::Result,
    models::file::{File, FileListParams, FileListResponse, FileUploadRequest, FileUploadResponse},
    types::{
        HttpMethod, PageStream, Pagination, PaginationLimits, ProgressCallback, RequestOptions,
    },
};
use reqwest::multipart::{Form, Part};
use std::path::Path;
use tokio::fs;

/// API client for Files endpoints
#[derive(Clone)]
pub struct FilesApi {
    client: Client,
}

impl FilesApi {
    /// Create a new Files API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Upload a file
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config, models::file::FileUploadRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let file_content = std::fs::read("document.pdf")?;
    /// let request = FileUploadRequest::new(file_content, "document.pdf", "application/pdf")
    ///     .expires_in_seconds(3600);
    ///
    /// let file = client.files().upload(request, None).await?;
    /// println!("Uploaded file: {}", file.file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(
        &self,
        request: FileUploadRequest,
        options: Option<RequestOptions>,
    ) -> Result<FileUploadResponse> {
        request.validate()?;
        let mut form = Form::new().part(
            "file",
            Part::bytes(request.content)
                .file_name(request.filename)
                .mime_str(&request.mime_type)
                .map_err(|e| {
                    crate::error::AnthropicError::file_error(format!("Invalid MIME type: {e}"))
                })?,
        );
        if let Some(seconds) = request.expires_in_seconds {
            form = form.text("expires_in_seconds", seconds.to_string());
        }
        self.client
            .request_multipart(HttpMethod::Post, "/files", form, options)
            .await
    }

    /// Upload a file from a path
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let file = client.files().upload_from_path(
    ///     "document.pdf",
    ///     None,
    ///     None
    /// ).await?;
    /// println!("Uploaded file: {}", file.file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_from_path(
        &self,
        file_path: impl AsRef<Path>,
        progress_callback: Option<ProgressCallback>,
        options: Option<RequestOptions>,
    ) -> Result<FileUploadResponse> {
        let path = file_path.as_ref();
        let content = fs::read(path).await.map_err(|e| {
            crate::error::AnthropicError::file_error(format!("Failed to read file: {}", e))
        })?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");

        let mime_type = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();

        let content_len = content.len() as u64;

        if let Some(ref callback) = progress_callback {
            callback(0, content_len);
        }

        let request = FileUploadRequest::new(content, filename, &mime_type);

        let result = self.upload(request, options).await;

        if let Some(ref callback) = progress_callback {
            let progress = if result.is_ok() { content_len } else { 0 };
            callback(progress, content_len);
        }

        result
    }

    /// List files
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config, types::Pagination};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let pagination = Pagination::new().with_limit(20);
    ///
    /// let response = client.files().list(Some(pagination), None).await?;
    /// for file in response.data {
    ///     println!("File: {} - Size: {} bytes", file.filename, file.size_bytes);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<FileListResponse> {
        self.list_with_params(pagination, FileListParams::new(), options)
            .await
    }

    /// List files with token pagination, explicit IDs, or a beta scope filter.
    ///
    /// This is a backward-compatible companion to [`list`](Self::list): pass a
    /// [`FileListParams`] (e.g. a session `scope_id` for Managed Agents session
    /// outputs) alongside the usual pagination.
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, models::file::FileListParams};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let params = FileListParams::new().scope_id("session_123");
    ///
    /// let response = client.files().list_with_params(None, params, None).await?;
    /// for file in response.data {
    ///     println!("File: {}", file.filename);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_with_params(
        &self,
        pagination: Option<Pagination>,
        params: FileListParams,
        options: Option<RequestOptions>,
    ) -> Result<FileListResponse> {
        let mut params = params;
        if let Some(pagination) = pagination {
            pagination.validate()?;
            if pagination.after.is_some() || pagination.before.is_some() {
                return Err(crate::error::AnthropicError::invalid_input(
                    "Current Files pagination uses page tokens, not after/before IDs",
                ));
            }
            if params.limit.is_some() && pagination.limit.is_some() {
                return Err(crate::error::AnthropicError::invalid_input(
                    "File page size specified twice",
                ));
            }
            params.limit = params.limit.or(pagination.limit);
        }
        params.validate()?;
        let path = build_path_with_query("/files", params.query_params());
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get file information
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let file = client.files().get("file_123", None).await?;
    /// println!("File: {} - Size: {} bytes", file.filename, file.size_bytes);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self, file_id: &str, options: Option<RequestOptions>) -> Result<File> {
        let path = format!("/files/{}", file_id);
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Download file content
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let content = client.files().download("file_123", None).await?;
    /// std::fs::write("downloaded_file.pdf", content)?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download(
        &self,
        file_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<u8>> {
        let path = format!("/files/{}/content", file_id);
        let response = self
            .client
            .request_stream(HttpMethod::Get, &path, None, options)
            .await?;

        if !response.status().is_success() {
            return Err(crate::error::AnthropicError::api_error(
                response.status().as_u16(),
                "File download failed".to_string(),
                None,
            ));
        }
        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Download file content to a path
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// client.files().download_to_path("file_123", "downloaded_file.pdf", None, None).await?;
    /// println!("File downloaded successfully");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_to_path(
        &self,
        file_id: &str,
        output_path: impl AsRef<Path>,
        progress_callback: Option<ProgressCallback>,
        options: Option<RequestOptions>,
    ) -> Result<()> {
        let content = self.download(file_id, options).await?;

        if let Some(callback) = &progress_callback {
            callback(0, content.len() as u64);
        }

        fs::write(output_path, &content).await.map_err(|e| {
            crate::error::AnthropicError::file_error(format!("Failed to write file: {}", e))
        })?;

        if let Some(callback) = progress_callback {
            callback(content.len() as u64, content.len() as u64);
        }

        Ok(())
    }

    /// Delete a file
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// client.files().delete("file_123", None).await?;
    /// println!("File deleted");
    /// # Ok(())
    /// # }
    /// ```
    pub async fn delete(&self, file_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/files/{}", file_id);
        let _: serde_json::Value = self
            .client
            .request(HttpMethod::Delete, &path, None, options)
            .await?;
        Ok(())
    }

    /// Lazily traverse current Files pages with finite limits and preserved filters/options.
    pub fn pages(
        &self,
        mut params: FileListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<File>> {
        params.validate()?;
        if params.ids.is_none() {
            params.limit = params.limit.or(Some(100));
        }
        let initial = params.page.take();
        let api = self.clone();
        paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.list_with_params(None, params, options).await?;
                if response.has_more && response.next_page.is_none() {
                    return Err(crate::error::AnthropicError::invalid_input(
                        "Files response advertises more pages without next_page",
                    ));
                }
                let item_ids = response.data.iter().map(|file| file.id.clone()).collect();
                Ok(TraversalPage {
                    data: response.data,
                    next_cursor: response.next_page,
                    item_ids,
                })
            }
        })
    }

    /// Collect current Files pages using the documented finite defaults.
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<File>> {
        self.list_all_with_limits(FileListParams::new(), PaginationLimits::default(), options)
            .await
    }

    /// Collect current Files pages, returning an error on a traversal limit.
    pub async fn list_all_with_limits(
        &self,
        params: FileListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<File>> {
        self.pages(params, limits, options)?.collect_items().await
    }
}
