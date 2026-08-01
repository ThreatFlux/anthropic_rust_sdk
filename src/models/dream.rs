//! Dreams API models (research preview: dreaming-2026-04-21).

use crate::types::{PaginatedResponse, Pagination};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An input source read by a Dream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DreamInput {
    /// Read an existing memory store without mutating it.
    MemoryStore { memory_store_id: String },
    /// Read session transcripts.
    Sessions { session_ids: Vec<String> },
}

/// A memory store written by a Dream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamOutput {
    pub memory_store_id: String,
    #[serde(rename = "type")]
    pub object_type: String,
}

/// Model configuration applied to every Dream pipeline stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamModelConfig {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
}

/// Model selector accepted by the create endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DreamModel {
    Id(String),
    Config(DreamModelConfig),
}

impl From<String> for DreamModel {
    fn from(value: String) -> Self {
        Self::Id(value)
    }
}

impl From<&str> for DreamModel {
    fn from(value: &str) -> Self {
        Self::Id(value.to_string())
    }
}

/// Dream lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DreamStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Canceled,
    #[serde(other)]
    Unknown,
}

/// Dream failure details.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DreamError {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

/// Aggregate token usage across all Dream pipeline stages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DreamUsage {
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    #[serde(default)]
    pub input_tokens: u32,
    #[serde(default)]
    pub output_tokens: u32,
}

/// An asynchronous memory-consolidation job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Dream {
    pub id: String,
    #[serde(default)]
    pub archived_at: Option<String>,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    #[serde(default)]
    pub error: Option<DreamError>,
    pub inputs: Vec<DreamInput>,
    #[serde(default)]
    pub instructions: Option<String>,
    pub model: DreamModelConfig,
    pub outputs: Vec<DreamOutput>,
    #[serde(default)]
    pub session_id: Option<String>,
    pub status: DreamStatus,
    #[serde(rename = "type")]
    pub object_type: String,
    pub usage: DreamUsage,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Request body for creating a Dream.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DreamCreateRequest {
    pub inputs: Vec<DreamInput>,
    pub model: DreamModel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
}

impl DreamCreateRequest {
    /// Create a Dream from input sources and a model id.
    pub fn new(inputs: Vec<DreamInput>, model: impl Into<DreamModel>) -> Self {
        Self {
            inputs,
            model: model.into(),
            instructions: None,
        }
    }

    /// Set optional consolidation instructions.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }
}

/// Filters supported by the Dreams list endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DreamListParams {
    pub pagination: Option<Pagination>,
    pub created_at_gt: Option<String>,
    pub created_at_lt: Option<String>,
    pub include_archived: Option<bool>,
    pub statuses: Vec<DreamStatus>,
}

impl DreamListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pagination(mut self, pagination: Pagination) -> Self {
        self.pagination = Some(pagination);
        self
    }

    pub fn include_archived(mut self, include_archived: bool) -> Self {
        self.include_archived = Some(include_archived);
        self
    }

    pub fn created_after(mut self, value: impl Into<String>) -> Self {
        self.created_at_gt = Some(value.into());
        self
    }

    pub fn created_before(mut self, value: impl Into<String>) -> Self {
        self.created_at_lt = Some(value.into());
        self
    }

    pub fn status(mut self, status: DreamStatus) -> Self {
        self.statuses.push(status);
        self
    }
}

/// Cursor response from the Dreams list endpoint.
pub type DreamListResponse = PaginatedResponse<Dream>;
