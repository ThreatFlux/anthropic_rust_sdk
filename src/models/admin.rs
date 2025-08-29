//! Admin API data models

use super::common::VecPush;
use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Organization information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    /// Organization ID
    pub id: String,
    /// Organization name
    pub name: String,
    /// Organization display name
    pub display_name: Option<String>,
    /// Organization description
    pub description: Option<String>,
    /// Organization settings
    pub settings: Option<OrganizationSettings>,
    /// When the organization was created
    pub created_at: DateTime<Utc>,
    /// When the organization was last updated
    pub updated_at: DateTime<Utc>,
}

/// Organization settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationSettings {
    /// Default model for the organization
    pub default_model: Option<String>,
    /// Rate limits
    pub rate_limits: Option<HashMap<String, u32>>,
    /// Feature flags
    pub features: Option<Vec<String>>,
}

/// Organization member
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// Member ID
    pub id: String,
    /// Member email
    pub email: String,
    /// Member name
    pub name: Option<String>,
    /// Member role
    pub role: MemberRole,
    /// Member status
    pub status: MemberStatus,
    /// When the member was invited
    pub invited_at: Option<DateTime<Utc>>,
    /// When the member joined
    pub joined_at: Option<DateTime<Utc>>,
    /// When the member was last active
    pub last_active_at: Option<DateTime<Utc>>,
}

/// Member role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    /// Organization owner
    Owner,
    /// Organization admin
    Admin,
    /// Organization member
    Member,
    /// Read-only access
    Viewer,
}

/// Member status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberStatus {
    /// Active member
    Active,
    /// Invited but not yet joined
    Invited,
    /// Suspended member
    Suspended,
}

/// Request to create a new member
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberCreateRequest {
    /// Member email
    pub email: String,
    /// Member role
    pub role: MemberRole,
    /// Member name (optional)
    pub name: Option<String>,
}

impl MemberCreateRequest {
    /// Create a new member creation request
    pub fn new(email: impl Into<String>, role: MemberRole) -> Self {
        Self {
            email: email.into(),
            role,
            name: None,
        }
    }

    /// Set the member name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// Request to update a member
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberUpdateRequest {
    /// New role (optional)
    pub role: Option<MemberRole>,
    /// New name (optional)
    pub name: Option<String>,
    /// New status (optional)
    pub status: Option<MemberStatus>,
}

impl MemberUpdateRequest {
    /// Create a new member update request
    pub fn new() -> Self {
        Self {
            role: None,
            name: None,
            status: None,
        }
    }

    /// Set the role
    pub fn role(mut self, role: MemberRole) -> Self {
        self.role = Some(role);
        self
    }

    /// Set the name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the status
    pub fn status(mut self, status: MemberStatus) -> Self {
        self.status = Some(status);
        self
    }
}

impl Default for MemberUpdateRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response when listing members
pub type MemberListResponse = PaginatedResponse<Member>;

/// Workspace information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Workspace ID
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Workspace display name
    pub display_name: Option<String>,
    /// Workspace description
    pub description: Option<String>,
    /// Workspace settings
    pub settings: Option<WorkspaceSettings>,
    /// Workspace status
    pub status: WorkspaceStatus,
    /// When the workspace was created
    pub created_at: DateTime<Utc>,
    /// When the workspace was last updated
    pub updated_at: DateTime<Utc>,
    /// When the workspace was archived (if applicable)
    pub archived_at: Option<DateTime<Utc>>,
}

/// Workspace status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    /// Active workspace
    Active,
    /// Archived workspace
    Archived,
    /// Suspended workspace
    Suspended,
}

/// Workspace settings
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSettings {
    /// Default model for the workspace
    pub default_model: Option<String>,
    /// Rate limits
    pub rate_limits: Option<HashMap<String, u32>>,
    /// Feature flags
    pub features: Option<Vec<String>>,
}

/// Request to create a new workspace
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceCreateRequest {
    /// Workspace name
    pub name: String,
    /// Workspace display name
    pub display_name: Option<String>,
    /// Workspace description
    pub description: Option<String>,
    /// Workspace settings
    pub settings: Option<WorkspaceSettings>,
}

impl WorkspaceCreateRequest {
    /// Create a new workspace creation request
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: None,
            description: None,
            settings: None,
        }
    }

    /// Set the display name
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the settings
    pub fn settings(mut self, settings: WorkspaceSettings) -> Self {
        self.settings = Some(settings);
        self
    }
}

/// Request to update a workspace
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceUpdateRequest {
    /// New name (optional)
    pub name: Option<String>,
    /// New display name (optional)
    pub display_name: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New settings (optional)
    pub settings: Option<WorkspaceSettings>,
}

impl WorkspaceUpdateRequest {
    /// Create a new workspace update request
    pub fn new() -> Self {
        Self {
            name: None,
            display_name: None,
            description: None,
            settings: None,
        }
    }

    /// Set the name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the display name
    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the settings
    pub fn settings(mut self, settings: WorkspaceSettings) -> Self {
        self.settings = Some(settings);
        self
    }
}

impl Default for WorkspaceUpdateRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response when listing workspaces
pub type WorkspaceListResponse = PaginatedResponse<Workspace>;

/// API key information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKey {
    /// API key ID
    pub id: String,
    /// API key name
    pub name: String,
    /// API key description
    pub description: Option<String>,
    /// Partial API key value (for display)
    pub partial_key: String,
    /// API key status
    pub status: Option<String>,
    /// API key permissions
    pub permissions: Option<Vec<String>>,
    /// Rate limits for this key
    pub rate_limits: Option<HashMap<String, u32>>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// When the key expires
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to create a new API key
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyCreateRequest {
    /// API key name
    pub name: String,
    /// API key description
    pub description: Option<String>,
    /// API key permissions
    pub permissions: Option<Vec<String>>,
    /// Rate limits
    pub rate_limits: Option<HashMap<String, u32>>,
    /// Expiration date
    pub expires_at: Option<DateTime<Utc>>,
}

impl ApiKeyCreateRequest {
    /// Create a new API key creation request
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            permissions: None,
            rate_limits: None,
            expires_at: None,
        }
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a permission
    pub fn add_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push_item(permission.into());
        self
    }

    /// Set rate limit
    pub fn rate_limit(mut self, operation: impl Into<String>, limit: u32) -> Self {
        self.rate_limits
            .get_or_insert_with(HashMap::new)
            .insert(operation.into(), limit);
        self
    }

    /// Set expiration date
    pub fn expires_at(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }
}

/// Request to update an API key
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyUpdateRequest {
    /// New name (optional)
    pub name: Option<String>,
    /// New description (optional)
    pub description: Option<String>,
    /// New status (optional)
    pub status: Option<String>,
    /// New permissions (optional)
    pub permissions: Option<Vec<String>>,
    /// New rate limits (optional)
    pub rate_limits: Option<HashMap<String, u32>>,
}

impl ApiKeyUpdateRequest {
    /// Create a new API key update request
    pub fn new() -> Self {
        Self {
            name: None,
            description: None,
            status: None,
            permissions: None,
            rate_limits: None,
        }
    }

    /// Set the name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the description
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the status
    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Add a permission
    pub fn add_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push_item(permission.into());
        self
    }
}

impl Default for ApiKeyUpdateRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response when listing API keys
pub type ApiKeyListResponse = PaginatedResponse<ApiKey>;

/// Usage report
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageReport {
    /// Total input tokens used
    pub input_tokens: u64,
    /// Total output tokens used
    pub output_tokens: u64,
    /// Total requests made
    pub request_count: u64,
    /// Usage breakdown by time period
    pub usage_by_period: Option<Vec<UsagePeriod>>,
    /// Usage breakdown by model
    pub usage_by_model: Option<HashMap<String, ModelUsage>>,
    /// Cost information
    pub cost: Option<CostInfo>,
}

/// Usage for a specific time period
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsagePeriod {
    /// Period start time
    pub period_start: DateTime<Utc>,
    /// Period end time
    pub period_end: DateTime<Utc>,
    /// Input tokens used in this period
    pub input_tokens: u64,
    /// Output tokens used in this period
    pub output_tokens: u64,
    /// Requests made in this period
    pub request_count: u64,
}

/// Usage statistics for a specific model
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Model name
    pub model: String,
    /// Input tokens used
    pub input_tokens: u64,
    /// Output tokens used
    pub output_tokens: u64,
    /// Number of requests
    pub request_count: u64,
}

/// Cost information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CostInfo {
    /// Total cost in cents
    pub total_cost_cents: u64,
    /// Input cost in cents
    pub input_cost_cents: u64,
    /// Output cost in cents
    pub output_cost_cents: u64,
    /// Currency code
    pub currency: String,
}

/// Usage query parameters
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageQuery {
    /// Start date for the query
    pub start_date: Option<DateTime<Utc>>,
    /// End date for the query
    pub end_date: Option<DateTime<Utc>>,
    /// Granularity for the report
    pub granularity: Option<String>,
    /// Specific workspace ID
    pub workspace_id: Option<String>,
    /// Specific API key ID
    pub api_key_id: Option<String>,
    /// Specific model
    pub model: Option<String>,
}

impl UsageQuery {
    /// Create a new usage query
    pub fn new() -> Self {
        Self {
            start_date: None,
            end_date: None,
            granularity: None,
            workspace_id: None,
            api_key_id: None,
            model: None,
        }
    }

    /// Set start date
    pub fn start_date(mut self, start_date: DateTime<Utc>) -> Self {
        self.start_date = Some(start_date);
        self
    }

    /// Set end date
    pub fn end_date(mut self, end_date: DateTime<Utc>) -> Self {
        self.end_date = Some(end_date);
        self
    }

    /// Set granularity
    pub fn granularity(mut self, granularity: impl Into<String>) -> Self {
        self.granularity = Some(granularity.into());
        self
    }

    /// Set workspace ID
    pub fn workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    /// Set API key ID
    pub fn api_key_id(mut self, api_key_id: impl Into<String>) -> Self {
        self.api_key_id = Some(api_key_id.into());
        self
    }

    /// Set model
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl Default for UsageQuery {
    fn default() -> Self {
        Self::new()
    }
}

/// Response when listing usage reports
pub type UsageReportListResponse = PaginatedResponse<UsageReport>;

/// API key usage information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyUsage {
    /// API key ID
    pub api_key_id: String,
    /// API key name
    pub api_key_name: String,
    /// Usage statistics
    pub usage: ModelUsage,
    /// Cost information
    pub cost: Option<CostInfo>,
}
