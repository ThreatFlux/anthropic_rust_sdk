//! User Profiles API models (beta: user-profiles-2026-03-24).

use crate::types::{PaginatedResponse, Pagination};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A user profile used to attribute API requests to an end user or reseller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: String,
    pub created_at: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    pub relationship: UserProfileRelationship,
    #[serde(default)]
    pub trust_grants: HashMap<String, TrustGrant>,
    #[serde(rename = "type")]
    pub object_type: String,
    pub updated_at: String,
    #[serde(default)]
    pub external_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

/// Relationship between a profile and the platform owning the API key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserProfileRelationship {
    External,
    Resold,
    Internal,
    #[serde(other)]
    Unknown,
}

/// State of a profile trust grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrustGrantStatus {
    Active,
    Pending,
    Rejected,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustGrant {
    pub status: TrustGrantStatus,
}

/// Request to create a user profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserProfileCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<UserProfileRelationship>,
}

impl UserProfileCreateRequest {
    pub fn new(relationship: UserProfileRelationship) -> Self {
        Self {
            relationship: Some(relationship),
            ..Self::default()
        }
    }

    pub fn external_id(mut self, external_id: impl Into<String>) -> Self {
        self.external_id = Some(external_id.into());
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Request to update a user profile. Metadata values set to an empty string are removed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserProfileUpdateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relationship: Option<UserProfileRelationship>,
}

/// Enrollment URL returned for a profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnrollmentUrl {
    pub expires_at: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub url: String,
}

/// Parameters for listing profiles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserProfileListParams {
    pub pagination: Option<Pagination>,
    pub order: Option<String>,
}

impl UserProfileListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pagination(mut self, pagination: Pagination) -> Self {
        self.pagination = Some(pagination);
        self
    }

    pub fn order(mut self, order: impl Into<String>) -> Self {
        self.order = Some(order.into());
        self
    }
}

pub type UserProfileListResponse = PaginatedResponse<UserProfile>;
