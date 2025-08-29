//! Files API implementation

use crate::{
    api::utils::{build_paginated_path, create_default_pagination},
    client::Client,
    error::Result,
    models::file::{File, FileListResponse, FileUploadRequest, FileUploadResponse},
    types::{HttpMethod, Pagination, ProgressCallback, RequestOptions},
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
    /// use threatflux::{Client, Config, models::file::FileUploadRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let file_content = std::fs::read("document.pdf")?;
    /// let request = FileUploadRequest::new(file_content, "document.pdf", "application/pdf")
    ///     .purpose("user_data");
    ///
    /// let file = client.files().upload(request, None).await?;
    /// println!("Uploaded file: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(
        &self,
        request: FileUploadRequest,
        options: Option<RequestOptions>,
    ) -> Result<FileUploadResponse> {
        let form = Form::new()
            .part(
                "file",
                Part::bytes(request.content)
                    .file_name(request.filename)
                    .mime_str(&request.mime_type)
                    .map_err(|e| {
                        crate::error::AnthropicError::file_error(format!(
                            "Invalid MIME type: {}",
                            e
                        ))
                    })?,
            )
            .text("purpose", request.purpose);

        // For file uploads, we need to use multipart form data instead of JSON
        let mut url = self.client.config().base_url.clone();
        url.set_path("/v1/files");
        let headers = self.client.build_admin_headers(&options)?;

        let mut request_builder = reqwest::Client::new()
            .post(url)
            .headers(headers)
            .multipart(form);

        if let Some(opts) = &options {
            if let Some(timeout) = opts.timeout {
                request_builder = request_builder.timeout(timeout);
            }
        }

        let response = request_builder.send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(crate::error::AnthropicError::api_error(
                status.as_u16(),
                error_text,
                None,
            ));
        }

        let file_response: FileUploadResponse = response.json().await?;
        Ok(file_response)
    }

    /// Upload a file from a path
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    ///
    /// let file = client.files().upload_from_path(
    ///     "document.pdf",
    ///     "user_data",
    ///     None,
    ///     None
    /// ).await?;
    /// println!("Uploaded file: {}", file.id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_from_path(
        &self,
        file_path: impl AsRef<Path>,
        purpose: &str,
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

        let request = FileUploadRequest::new(content, filename, &mime_type).purpose(purpose);

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
    /// use threatflux::{Client, Config, types::Pagination};
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
        let path = build_paginated_path("/files", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get file information
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
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
    /// use threatflux::{Client, Config};
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
        let path = format!("/files/{}/download", file_id);
        let response = self
            .client
            .request_stream(HttpMethod::Get, &path, None, options)
            .await?;

        let bytes = response.bytes().await?;
        Ok(bytes.to_vec())
    }

    /// Download file content to a path
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux::{Client, Config};
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
    /// use threatflux::{Client, Config};
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

    /// List files by purpose
    pub async fn list_by_purpose(
        &self,
        purpose: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<File>> {
        let pagination = create_default_pagination(None);
        let response = self.list(Some(pagination), options).await?;

        Ok(response
            .data
            .into_iter()
            .filter(|file| file.purpose == purpose)
            .collect())
    }
}
