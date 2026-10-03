//! File-related data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A file uploaded to the Anthropic API
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct File {
    /// Unique identifier for the file
    pub id: String,
    /// Object type (always "file")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Original filename
    pub filename: String,
    /// MIME type of the file
    pub mime_type: String,
    /// Size of the file in bytes
    pub size_bytes: u64,
    /// Legacy purpose when supplied by a historical endpoint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    /// Whether the server allows downloading this file; omission is unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub downloadable: Option<bool>,
    /// Expiration timestamp, absent for a file that does not expire.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    /// When the file was uploaded
    pub created_at: DateTime<Utc>,
    /// When the file was last modified
    pub updated_at: Option<DateTime<Utc>>,
    /// File status
    pub status: Option<FileStatus>,
    /// Error information if file processing failed
    pub error: Option<FileError>,
    /// Future metadata retained without data loss.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// File processing status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    /// File is being processed
    Processing,
    /// File is ready for use
    Ready,
    /// File processing failed
    Error,
}

/// File error information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message
    pub message: String,
}

/// Request to upload a file
#[derive(Debug, Clone)]
pub struct FileUploadRequest {
    /// File content as bytes
    pub content: Vec<u8>,
    /// Original filename
    pub filename: String,
    /// MIME type
    pub mime_type: String,
    /// Expiration between one hour (3600) and ninety days (7,776,000).
    pub expires_in_seconds: Option<u32>,
}

impl FileUploadRequest {
    /// Create a new file upload request
    pub fn new(
        content: Vec<u8>,
        filename: impl Into<String>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            content,
            filename: filename.into(),
            mime_type: mime_type.into(),
            expires_in_seconds: None,
        }
    }

    /// Set expiration; [`validate`](Self::validate) enforces the server's range.
    pub fn expires_in_seconds(mut self, seconds: u32) -> Self {
        self.expires_in_seconds = Some(seconds);
        self
    }

    /// Validate the documented expiration range before opening a connection.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self
            .expires_in_seconds
            .is_some_and(|seconds| !(3600..=7_776_000).contains(&seconds))
        {
            return Err(crate::error::AnthropicError::invalid_input(
                "File expiration must be between 3600 and 7776000 seconds",
            ));
        }
        Ok(())
    }

    /// Get the file size
    pub fn size(&self) -> u64 {
        self.content.len() as u64
    }

    /// Check if the file is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

/// Response when uploading a file
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileUploadResponse {
    /// The uploaded file information
    #[serde(flatten)]
    pub file: File,
}

/// Current token-cursor response, retaining historical ID pagination metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileListResponse {
    /// Files on this page.
    pub data: Vec<File>,
    /// Current continuation token; determines continuation without `has_more`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
    /// Historical continuation marker.
    #[serde(default)]
    pub has_more: bool,
    /// Historical first ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub first_id: Option<String>,
    /// Historical last ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_id: Option<String>,
    /// Future response metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Optional filters for listing files.
///
/// Current token pagination and ID lookup. `scope_id` is available on the beta
/// Managed Agents Files path; callers explicitly select its beta header.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileListParams {
    /// Restrict results to files scoped to this id (e.g. a session id for
    /// Managed Agents session outputs).
    pub scope_id: Option<String>,
    /// Number of results (1–1000), mutually exclusive with `ids`.
    pub limit: Option<u32>,
    /// Opaque continuation token, mutually exclusive with `ids`.
    pub page: Option<String>,
    /// Lookup at most 100 unique IDs in one page, without limit/page.
    pub ids: Option<Vec<String>>,
}

impl FileListParams {
    /// Create an empty set of file-list filters.
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by scope id.
    pub fn scope_id(mut self, scope_id: impl Into<String>) -> Self {
        self.scope_id = Some(scope_id.into());
        self
    }

    /// Set page size.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set a continuation token.
    pub fn with_page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Select explicit file IDs.
    pub fn with_ids(mut self, ids: Vec<String>) -> Self {
        self.ids = Some(ids);
        self
    }

    /// Validate the current endpoint's pagination/lookup contract.
    pub fn validate(&self) -> crate::error::Result<()> {
        use crate::error::AnthropicError;
        if self.limit.is_some_and(|limit| !(1..=1000).contains(&limit)) {
            return Err(AnthropicError::invalid_input(
                "File list limit must be between 1 and 1000",
            ));
        }
        if self.page.as_deref() == Some("") {
            return Err(AnthropicError::invalid_input(
                "File page token must not be empty",
            ));
        }
        if let Some(ids) = &self.ids {
            if self.limit.is_some() || self.page.is_some() {
                return Err(AnthropicError::invalid_input(
                    "File IDs are mutually exclusive with limit/page",
                ));
            }
            if ids.iter().any(String::is_empty)
                || ids.iter().collect::<std::collections::HashSet<_>>().len() > 100
            {
                return Err(AnthropicError::invalid_input(
                    "File lookup requires nonempty IDs and at most 100 unique IDs",
                ));
            }
        }
        Ok(())
    }

    /// Build the query-parameter fragments for these filters.
    pub fn query_params(&self) -> Vec<String> {
        let mut params = Vec::new();
        if let Some(scope_id) = &self.scope_id {
            params.push(format!(
                "scope_id={}",
                crate::api::utils::encode_query_value(scope_id)
            ));
        }
        if let Some(limit) = self.limit {
            params.push(format!("limit={limit}"));
        }
        if let Some(page) = &self.page {
            params.push(format!(
                "page={}",
                crate::api::utils::encode_query_value(page)
            ));
        }
        if let Some(ids) = &self.ids {
            for id in ids {
                params.push(format!(
                    "ids%5B%5D={}",
                    crate::api::utils::encode_query_value(id)
                ));
            }
        }
        params
    }
}

/// File download information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileDownload {
    /// File content as bytes
    pub content: Vec<u8>,
    /// Content type
    pub content_type: String,
    /// Content length
    pub content_length: u64,
    /// Original filename
    pub filename: String,
}

impl FileDownload {
    /// Create a new file download
    pub fn new(content: Vec<u8>, content_type: String, filename: String) -> Self {
        let content_length = content.len() as u64;
        Self {
            content,
            content_type,
            content_length,
            filename,
        }
    }

    /// Get the file size
    pub fn size(&self) -> u64 {
        self.content_length
    }

    /// Check if the download is empty
    pub fn is_empty(&self) -> bool {
        self.content.is_empty()
    }

    /// Save to a file
    pub async fn save_to_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        tokio::fs::write(path, &self.content).await
    }
}

/// File purpose enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilePurpose {
    /// User-uploaded data
    UserData,
    /// Assistant-generated files
    AssistantData,
    /// Batch processing input
    BatchInput,
    /// Batch processing output
    BatchOutput,
    /// Training data
    Training,
    /// Fine-tuning data
    FineTuning,
}

impl std::fmt::Display for FilePurpose {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserData => write!(f, "user_data"),
            Self::AssistantData => write!(f, "assistant_data"),
            Self::BatchInput => write!(f, "batch_input"),
            Self::BatchOutput => write!(f, "batch_output"),
            Self::Training => write!(f, "training"),
            Self::FineTuning => write!(f, "fine_tuning"),
        }
    }
}

impl std::str::FromStr for FilePurpose {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "user_data" => Ok(Self::UserData),
            "assistant_data" => Ok(Self::AssistantData),
            "batch_input" => Ok(Self::BatchInput),
            "batch_output" => Ok(Self::BatchOutput),
            "training" => Ok(Self::Training),
            "fine_tuning" => Ok(Self::FineTuning),
            _ => Err(()),
        }
    }
}

impl File {
    /// Check if the file is ready for use
    pub fn is_ready(&self) -> bool {
        matches!(self.status, Some(FileStatus::Ready))
    }

    /// Check if the file is still processing
    pub fn is_processing(&self) -> bool {
        matches!(self.status, Some(FileStatus::Processing))
    }

    /// Check if the file has an error
    pub fn has_error(&self) -> bool {
        matches!(self.status, Some(FileStatus::Error))
    }

    /// Get the file extension
    pub fn extension(&self) -> Option<&str> {
        std::path::Path::new(&self.filename)
            .extension()
            .and_then(|ext| ext.to_str())
    }

    /// Check if the file is an image
    pub fn is_image(&self) -> bool {
        self.mime_type.starts_with("image/")
    }

    /// Check if the file is a document
    pub fn is_document(&self) -> bool {
        matches!(
            self.mime_type.as_str(),
            "application/pdf" | "text/plain" | "text/csv" | "application/json"
        ) || self.mime_type.starts_with("text/")
    }

    /// Get human-readable file size
    pub fn human_readable_size(&self) -> String {
        let size = self.size_bytes as f64;
        if size < 1024.0 {
            format!("{} B", size)
        } else if size < 1024.0 * 1024.0 {
            format!("{:.1} KB", size / 1024.0)
        } else if size < 1024.0 * 1024.0 * 1024.0 {
            format!("{:.1} MB", size / (1024.0 * 1024.0))
        } else {
            format!("{:.1} GB", size / (1024.0 * 1024.0 * 1024.0))
        }
    }
}
