//! File-related data models

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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
    /// Purpose of the file
    pub purpose: String,
    /// When the file was uploaded
    pub created_at: DateTime<Utc>,
    /// When the file was last modified
    pub updated_at: Option<DateTime<Utc>>,
    /// File status
    pub status: Option<FileStatus>,
    /// Error information if file processing failed
    pub error: Option<FileError>,
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
    /// Purpose of the file
    pub purpose: String,
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
            purpose: "user_data".to_string(), // default purpose
        }
    }

    /// Set the purpose of the file
    pub fn purpose(mut self, purpose: impl Into<String>) -> Self {
        self.purpose = purpose.into();
        self
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

/// Response when listing files
pub type FileListResponse = PaginatedResponse<File>;

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
