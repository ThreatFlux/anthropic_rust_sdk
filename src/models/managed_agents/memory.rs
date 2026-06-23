//! Managed Agents — Memory store data models (beta: managed-agents-2026-04-01)

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A persistent memory store that can be attached to sessions.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryStore {
    /// Object type (always `"memory_store"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique store identifier.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single memory entry within a store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Memory {
    /// Object type (always `"memory"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique memory identifier.
    pub id: String,
    /// Memory content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Current version identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// ISO 8601 update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A historical version of a memory entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryVersion {
    /// Object type (always `"memory_version"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique version identifier.
    pub id: String,
    /// Parent memory identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,
    /// Content captured at this version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Request body for creating a memory store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryStoreCreateRequest {
    /// Human-friendly name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl MemoryStoreCreateRequest {
    /// Create a new memory store create request.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Request body for updating a memory store.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemoryStoreUpdateRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl MemoryStoreUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Request body for creating a memory entry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoryCreateRequest {
    /// Memory content.
    pub content: String,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl MemoryCreateRequest {
    /// Create a new memory create request.
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            metadata: HashMap::new(),
        }
    }
}

/// Request body for updating a memory entry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemoryUpdateRequest {
    /// New content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl MemoryUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }
}

/// Request body for redacting a memory entry.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MemoryRedactRequest {
    /// Optional reason for the redaction.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl MemoryRedactRequest {
    /// Create an empty redact request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the reason.
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Response when listing memory stores (cursor-style pagination).
pub type MemoryStoreListResponse = PaginatedResponse<MemoryStore>;

/// Response when listing memory entries (cursor-style pagination).
pub type MemoryListResponse = PaginatedResponse<Memory>;

/// Response when listing memory versions (cursor-style pagination).
pub type MemoryVersionListResponse = PaginatedResponse<MemoryVersion>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_round_trip() {
        let json = r#"{"type":"memory_store","id":"mem_1","name":"notes"}"#;
        let store: MemoryStore = serde_json::from_str(json).unwrap();
        assert_eq!(store.id, "mem_1");
        assert_eq!(store.name, "notes");
        let value = serde_json::to_value(&store).unwrap();
        assert_eq!(value["type"], "memory_store");
    }
}
