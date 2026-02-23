//! Message batch-related data models

use super::message::{MessageRequest, MessageResponse};
use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a message batch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageBatchStatus {
    /// Batch is being processed
    InProgress,
    /// Batch completed successfully
    Completed,
    /// Batch failed
    Failed,
    /// Batch was cancelled
    Cancelled,
    /// Batch is waiting to be processed
    Pending,
}

/// A batch of message requests
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageBatch {
    /// Unique identifier for the batch
    pub id: String,
    /// Object type (always "message_batch")
    #[serde(rename = "type")]
    pub object_type: String,
    /// Current processing status
    pub processing_status: MessageBatchStatus,
    /// Number of requests in the batch
    pub request_counts: RequestCounts,
    /// When the batch was created
    pub created_at: DateTime<Utc>,
    /// When the batch processing started
    pub in_progress_at: Option<DateTime<Utc>>,
    /// When the batch processing completed
    pub completed_at: Option<DateTime<Utc>>,
    /// When the batch was cancelled
    pub cancelled_at: Option<DateTime<Utc>>,
    /// When the batch failed
    pub failed_at: Option<DateTime<Utc>>,
    /// When the batch expires (if not processed)
    pub expires_at: DateTime<Utc>,
    /// Error information if the batch failed
    pub error: Option<BatchError>,
    /// Results file ID (available after completion)
    pub results_file_id: Option<String>,
}

/// Request counts for a batch
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RequestCounts {
    /// Total number of requests
    pub total: u32,
    /// Number of completed requests
    pub completed: u32,
    /// Number of failed requests
    pub failed: u32,
    /// Number of cancelled requests
    pub cancelled: u32,
}

/// Batch error information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message
    pub message: String,
    /// Additional error details
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Request to create a message batch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageBatchCreateRequest {
    /// List of batch requests
    pub requests: Vec<BatchRequestItem>,
}

impl MessageBatchCreateRequest {
    /// Create a new batch request
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// Add a request to the batch
    pub fn add_request_item(mut self, item: BatchRequestItem) -> Self {
        self.requests.push(item);
        self
    }

    /// Add a simple request to the batch (convenience method)
    pub fn add_request(
        mut self,
        custom_id: impl Into<String>,
        model: impl Into<String>,
        message: impl Into<String>,
        max_tokens: u32,
    ) -> Self {
        let request = MessageRequest::new()
            .model(model)
            .max_tokens(max_tokens)
            .add_user_message(message);

        let item = BatchRequestItem::new(custom_id, request);
        self.requests.push(item);
        self
    }
}

impl Default for MessageBatchCreateRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Individual request item in a batch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchRequestItem {
    /// Custom ID for tracking this request
    pub custom_id: String,
    /// The message request
    pub params: MessageRequest,
}

impl BatchRequestItem {
    /// Create a new batch request item
    pub fn new(custom_id: impl Into<String>, params: MessageRequest) -> Self {
        Self {
            custom_id: custom_id.into(),
            params,
        }
    }
}

/// Generic message batch request (for backward compatibility)
pub type MessageBatchRequest = MessageBatchCreateRequest;

/// Response when listing message batches
pub type MessageBatchListResponse = PaginatedResponse<MessageBatch>;

/// Result of a batch request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchResult {
    /// Custom ID from the request
    pub custom_id: String,
    /// Result type
    #[serde(rename = "type")]
    pub result_type: String,
    /// The message response (if successful)
    pub message: Option<MessageResponse>,
    /// Error information (if failed)
    pub error: Option<BatchResultError>,
}

/// A single line in `/messages/batches/{id}/results` output
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageBatchResultEntry {
    /// Custom ID from the original batch request
    pub custom_id: String,
    /// Result payload
    pub result: MessageBatchResult,
}

/// Result payload for a single batch entry
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageBatchResult {
    /// The request completed successfully
    Succeeded {
        /// Message response generated by Claude
        message: MessageResponse,
    },
    /// The request failed with an error
    Errored {
        /// Error details
        error: BatchResultError,
    },
    /// The request was canceled before completion
    #[serde(rename = "canceled", alias = "cancelled")]
    Canceled {},
    /// The request expired before completion
    Expired {},
}

impl MessageBatchResult {
    /// Whether this entry is successful
    pub fn is_success(&self) -> bool {
        matches!(self, Self::Succeeded { .. })
    }

    /// Get the message response when successful
    pub fn message(&self) -> Option<&MessageResponse> {
        match self {
            Self::Succeeded { message } => Some(message),
            _ => None,
        }
    }

    /// Get error details when failed
    pub fn error(&self) -> Option<&BatchResultError> {
        match self {
            Self::Errored { error } => Some(error),
            _ => None,
        }
    }
}

/// Error information for a failed batch request
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BatchResultError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error message
    pub message: String,
}

impl MessageBatch {
    /// Check if the batch is complete
    pub fn is_complete(&self) -> bool {
        matches!(
            self.processing_status,
            MessageBatchStatus::Completed
                | MessageBatchStatus::Failed
                | MessageBatchStatus::Cancelled
        )
    }

    /// Check if the batch was successful
    pub fn is_successful(&self) -> bool {
        matches!(self.processing_status, MessageBatchStatus::Completed)
    }

    /// Check if the batch failed
    pub fn is_failed(&self) -> bool {
        matches!(self.processing_status, MessageBatchStatus::Failed)
    }

    /// Check if the batch was cancelled
    pub fn is_cancelled(&self) -> bool {
        matches!(self.processing_status, MessageBatchStatus::Cancelled)
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.request_counts.total == 0 {
            return 0.0;
        }

        let completed = self.request_counts.completed
            + self.request_counts.failed
            + self.request_counts.cancelled;
        (completed as f64 / self.request_counts.total as f64) * 100.0
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total_processed = self.request_counts.completed + self.request_counts.failed;
        if total_processed == 0 {
            return 0.0;
        }

        (self.request_counts.completed as f64 / total_processed as f64) * 100.0
    }

    /// Check if the batch has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Get processing duration
    pub fn processing_duration(&self) -> Option<chrono::Duration> {
        match (
            self.in_progress_at,
            self.completed_at.or(self.failed_at).or(self.cancelled_at),
        ) {
            (Some(start), Some(end)) => Some(end - start),
            _ => None,
        }
    }
}
