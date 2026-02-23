//! Admin API data models

use super::common::VecPush;
use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Organization information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Organization {
    /// Object type, typically `organization`.
    #[serde(rename = "type", default)]
    pub object_type: Option<String>,
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
    pub created_at: Option<DateTime<Utc>>,
    /// When the organization was last updated
    pub updated_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
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

/// Organization user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct User {
    /// Object type.
    #[serde(rename = "type")]
    pub object_type: String,
    /// User ID.
    pub id: String,
    /// User email.
    pub email: String,
    /// Optional user display name.
    #[serde(default)]
    pub name: Option<String>,
    /// Organization role.
    pub role: UserRole,
    /// Time this user was added to the organization.
    pub added_at: DateTime<Utc>,
}

/// Role values for organization users.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    User,
    Developer,
    Billing,
    Admin,
    ClaudeCodeUser,
    Managed,
}

/// Role values accepted by user update endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserUpdateRole {
    User,
    Developer,
    Billing,
    ClaudeCodeUser,
    Managed,
}

/// Request body for updating an organization user.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserUpdateRequest {
    /// New organization role.
    pub role: UserUpdateRole,
}

impl UserUpdateRequest {
    /// Create a new user update request.
    pub fn new(role: UserUpdateRole) -> Self {
        Self { role }
    }
}

/// User deletion response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserDeleteResponse {
    /// Deleted user ID.
    pub id: String,
    /// Object type.
    #[serde(rename = "type")]
    pub object_type: String,
}

/// Query parameters for listing users.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct UserListParams {
    /// Number of items to return.
    pub limit: Option<u32>,
    /// Cursor for forward pagination.
    pub after_id: Option<String>,
    /// Cursor for reverse pagination.
    pub before_id: Option<String>,
    /// Filter by exact user email.
    pub email: Option<String>,
}

impl UserListParams {
    /// Create empty list params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page size limit.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set forward pagination cursor.
    pub fn with_after_id(mut self, after_id: impl Into<String>) -> Self {
        self.after_id = Some(after_id.into());
        self
    }

    /// Set reverse pagination cursor.
    pub fn with_before_id(mut self, before_id: impl Into<String>) -> Self {
        self.before_id = Some(before_id.into());
        self
    }

    /// Filter by email.
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }
}

/// Response when listing users.
pub type UserListResponse = PaginatedResponse<User>;

/// Organization member
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    /// Member ID
    pub id: String,
    /// Member email
    pub email: String,
    /// Member name
    #[serde(default)]
    pub name: Option<String>,
    /// Member role
    pub role: MemberRole,
    /// Member status
    pub status: MemberStatus,
    /// When the member was invited
    #[serde(default)]
    pub invited_at: Option<DateTime<Utc>>,
    /// When the member joined
    #[serde(default)]
    pub joined_at: Option<DateTime<Utc>>,
    /// When the member was last active
    #[serde(default)]
    pub last_active_at: Option<DateTime<Utc>>,
}

/// Member role
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemberRole {
    /// Organization owner
    Owner,
    /// Organization billing admin
    Billing,
    /// Organization developer
    Developer,
    /// Organization admin
    Admin,
    /// Organization member
    Member,
    /// Claude Code seat
    ClaudeCodeUser,
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
    /// Pending invitation
    Pending,
    /// Suspended member
    Suspended,
    /// Inactive user
    Inactive,
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

/// Organization invite information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Invite {
    /// Object type.
    #[serde(rename = "type")]
    pub object_type: String,
    /// Invite ID.
    pub id: String,
    /// Invitee email.
    pub email: String,
    /// Invite expiration timestamp.
    pub expires_at: DateTime<Utc>,
    /// Invite creation timestamp.
    pub invited_at: DateTime<Utc>,
    /// Role granted when accepted.
    pub role: UserRole,
    /// Invite status.
    pub status: InviteStatus,
}

/// Invite status values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteStatus {
    Accepted,
    Expired,
    Deleted,
    Pending,
}

/// Role values accepted by invite creation endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteCreateRole {
    User,
    Developer,
    Billing,
    ClaudeCodeUser,
    Managed,
}

/// Request to create an organization invite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InviteCreateRequest {
    /// Invitee email.
    pub email: String,
    /// Role to grant.
    pub role: InviteCreateRole,
}

impl InviteCreateRequest {
    /// Create a new invite request.
    pub fn new(email: impl Into<String>, role: InviteCreateRole) -> Self {
        Self {
            email: email.into(),
            role,
        }
    }
}

/// Invite deletion response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InviteDeleteResponse {
    /// Deleted invite ID.
    pub id: String,
    /// Object type.
    #[serde(rename = "type")]
    pub object_type: String,
}

/// Query parameters for listing invites.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct InviteListParams {
    /// Number of items to return.
    pub limit: Option<u32>,
    /// Cursor for forward pagination.
    pub after_id: Option<String>,
    /// Cursor for reverse pagination.
    pub before_id: Option<String>,
}

impl InviteListParams {
    /// Create empty list params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page size.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set forward cursor.
    pub fn with_after_id(mut self, after_id: impl Into<String>) -> Self {
        self.after_id = Some(after_id.into());
        self
    }

    /// Set reverse cursor.
    pub fn with_before_id(mut self, before_id: impl Into<String>) -> Self {
        self.before_id = Some(before_id.into());
        self
    }
}

/// Response when listing invites.
pub type InviteListResponse = PaginatedResponse<Invite>;

/// Workspace data residency settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkspaceDataResidency {
    /// Allowed inference geographies (ISO-3166-1 alpha-2 country codes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geographies: Option<Vec<String>>,
    /// Additional data residency fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl WorkspaceDataResidency {
    /// Create a new empty data residency configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set allowed inference geographies.
    pub fn inference_geographies(
        mut self,
        inference_geographies: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.inference_geographies = Some(
            inference_geographies
                .into_iter()
                .map(|v| v.into())
                .collect(),
        );
        self
    }
}

/// Workspace information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Object type, typically `workspace`.
    #[serde(rename = "type", default)]
    pub object_type: Option<String>,
    /// Workspace ID
    pub id: String,
    /// Workspace name
    pub name: String,
    /// Workspace display name
    pub display_name: Option<String>,
    /// Workspace display color.
    #[serde(default)]
    pub display_color: Option<String>,
    /// Workspace description
    pub description: Option<String>,
    /// Workspace settings
    pub settings: Option<WorkspaceSettings>,
    /// Workspace status
    #[serde(default)]
    pub status: Option<WorkspaceStatus>,
    /// Region/residency setting for workspace data.
    #[serde(default)]
    pub data_residency: Option<WorkspaceDataResidency>,
    /// When the workspace was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the workspace was last updated
    pub updated_at: Option<DateTime<Utc>>,
    /// When the workspace was archived (if applicable)
    pub archived_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
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
    /// Region/residency setting for this workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_residency: Option<WorkspaceDataResidency>,
}

impl WorkspaceCreateRequest {
    /// Create a new workspace creation request
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            display_name: None,
            description: None,
            settings: None,
            data_residency: None,
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

    /// Set data residency value (for example `us` or `eu`).
    pub fn data_residency(mut self, data_residency: WorkspaceDataResidency) -> Self {
        self.data_residency = Some(data_residency);
        self
    }

    /// Set data residency inference geographies convenience helper.
    pub fn data_residency_inference_geographies(
        self,
        inference_geographies: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.data_residency(
            WorkspaceDataResidency::new().inference_geographies(inference_geographies),
        )
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
    /// New data residency value (optional).
    pub data_residency: Option<WorkspaceDataResidency>,
}

impl WorkspaceUpdateRequest {
    /// Create a new workspace update request
    pub fn new() -> Self {
        Self {
            name: None,
            display_name: None,
            description: None,
            settings: None,
            data_residency: None,
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

    /// Set data residency.
    pub fn data_residency(mut self, data_residency: WorkspaceDataResidency) -> Self {
        self.data_residency = Some(data_residency);
        self
    }
    /// Set data residency inference geographies convenience helper.
    pub fn data_residency_inference_geographies(
        self,
        inference_geographies: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.data_residency(
            WorkspaceDataResidency::new().inference_geographies(inference_geographies),
        )
    }
}

impl Default for WorkspaceUpdateRequest {
    fn default() -> Self {
        Self::new()
    }
}

/// Response when listing workspaces
pub type WorkspaceListResponse = PaginatedResponse<Workspace>;

/// Workspace list query parameters.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorkspaceListParams {
    /// Number of items to return.
    pub limit: Option<u32>,
    /// Pagination cursor for forward traversal.
    pub after_id: Option<String>,
    /// Pagination cursor for backward traversal.
    pub before_id: Option<String>,
    /// Include archived workspaces.
    pub include_archived: Option<bool>,
}

impl WorkspaceListParams {
    /// Create empty list params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set limit.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set after cursor.
    pub fn with_after_id(mut self, after_id: impl Into<String>) -> Self {
        self.after_id = Some(after_id.into());
        self
    }

    /// Set before cursor.
    pub fn with_before_id(mut self, before_id: impl Into<String>) -> Self {
        self.before_id = Some(before_id.into());
        self
    }

    /// Include archived workspaces.
    pub fn include_archived(mut self, include_archived: bool) -> Self {
        self.include_archived = Some(include_archived);
        self
    }
}

/// Workspace member object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceMember {
    /// Object type.
    #[serde(rename = "type")]
    pub object_type: String,
    /// User ID.
    pub user_id: String,
    /// Workspace ID.
    pub workspace_id: String,
    /// Workspace-scoped role.
    pub workspace_role: WorkspaceMemberRole,
}

/// Role values for workspace members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMemberRole {
    WorkspaceUser,
    WorkspaceDeveloper,
    WorkspaceAdmin,
    WorkspaceBilling,
}

/// Role values accepted by workspace-member creation endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceMemberCreateRole {
    WorkspaceUser,
    WorkspaceDeveloper,
    WorkspaceAdmin,
}

/// Request body for adding a workspace member.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceMemberCreateRequest {
    /// User ID to add.
    pub user_id: String,
    /// Role to assign.
    pub workspace_role: WorkspaceMemberCreateRole,
}

impl WorkspaceMemberCreateRequest {
    /// Create a new workspace-member create request.
    pub fn new(user_id: impl Into<String>, workspace_role: WorkspaceMemberCreateRole) -> Self {
        Self {
            user_id: user_id.into(),
            workspace_role,
        }
    }
}

/// Request body for updating a workspace member role.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceMemberUpdateRequest {
    /// New role to assign.
    pub workspace_role: WorkspaceMemberRole,
}

impl WorkspaceMemberUpdateRequest {
    /// Create a new workspace-member update request.
    pub fn new(workspace_role: WorkspaceMemberRole) -> Self {
        Self { workspace_role }
    }
}

/// Workspace-member deletion response payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceMemberDeleteResponse {
    /// Object type.
    #[serde(rename = "type")]
    pub object_type: String,
    /// Deleted user ID.
    pub user_id: String,
    /// Workspace ID.
    pub workspace_id: String,
}

/// Query parameters for listing workspace members.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WorkspaceMemberListParams {
    /// Number of items to return.
    pub limit: Option<u32>,
    /// Cursor for forward pagination.
    pub after_id: Option<String>,
    /// Cursor for reverse pagination.
    pub before_id: Option<String>,
}

impl WorkspaceMemberListParams {
    /// Create empty list params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page size.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set forward cursor.
    pub fn with_after_id(mut self, after_id: impl Into<String>) -> Self {
        self.after_id = Some(after_id.into());
        self
    }

    /// Set reverse cursor.
    pub fn with_before_id(mut self, before_id: impl Into<String>) -> Self {
        self.before_id = Some(before_id.into());
        self
    }
}

/// Response when listing workspace members.
pub type WorkspaceMemberListResponse = PaginatedResponse<WorkspaceMember>;

/// API key creator actor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyActor {
    /// Actor ID.
    pub id: String,
    /// Actor type.
    #[serde(rename = "type")]
    pub object_type: String,
}

/// API key information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKey {
    /// Object type, typically `api_key`.
    #[serde(rename = "type", default)]
    pub object_type: Option<String>,
    /// API key ID
    pub id: String,
    /// API key name
    pub name: String,
    /// Actor that created this API key.
    #[serde(default)]
    pub created_by: Option<ApiKeyActor>,
    /// Workspace that owns this key.
    #[serde(default)]
    pub workspace_id: Option<String>,
    /// API key description
    pub description: Option<String>,
    /// Partial API key value (for display).
    ///
    /// The current Admin API field is `partial_key_hint`; `partial_key` is accepted
    /// for backwards compatibility.
    #[serde(rename = "partial_key_hint", alias = "partial_key", default)]
    pub partial_key_hint: Option<String>,
    /// API key status
    pub status: Option<String>,
    /// API key permissions
    pub permissions: Option<Vec<String>>,
    /// Rate limits for this key
    pub rate_limits: Option<HashMap<String, u32>>,
    /// When the key was created
    pub created_at: Option<DateTime<Utc>>,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// When the key expires
    pub expires_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
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

/// API key list query parameters for Admin API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ApiKeyListParams {
    /// Number of items to return.
    pub limit: Option<u32>,
    /// Pagination cursor for forward traversal.
    pub after_id: Option<String>,
    /// Pagination cursor for backward traversal.
    pub before_id: Option<String>,
    /// Filter by workspace.
    pub workspace_id: Option<String>,
    /// Filter by status.
    pub status: Option<String>,
    /// Filter by creator user.
    pub created_by_user_id: Option<String>,
}

impl ApiKeyListParams {
    /// Create a new empty list params object.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set limit.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set after cursor.
    pub fn with_after_id(mut self, after_id: impl Into<String>) -> Self {
        self.after_id = Some(after_id.into());
        self
    }

    /// Set before cursor.
    pub fn with_before_id(mut self, before_id: impl Into<String>) -> Self {
        self.before_id = Some(before_id.into());
        self
    }

    /// Filter by workspace ID.
    pub fn with_workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    /// Filter by status.
    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    /// Filter by creator user ID.
    pub fn with_created_by_user_id(mut self, created_by_user_id: impl Into<String>) -> Self {
        self.created_by_user_id = Some(created_by_user_id.into());
        self
    }
}

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

/// Query parameters for `/organizations/usage_report/messages`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageUsageReportParams {
    /// Inclusive start timestamp for the report window.
    pub starting_at: DateTime<Utc>,
    /// Exclusive end timestamp for the report window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_at: Option<DateTime<Utc>>,
    /// Filter by API key IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_ids: Option<Vec<String>>,
    /// Optional report granularity / bucket width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_width: Option<String>,
    /// Filter by context window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<Vec<String>>,
    /// Optional fields to group by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<String>>,
    /// Filter by inference geography.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geos: Option<Vec<String>>,
    /// Filter by model IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<Vec<String>>,
    /// Optional pagination token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// Filter by service tiers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tiers: Option<Vec<String>>,
    /// Filter by speed category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speeds: Option<Vec<String>>,
    /// Filter by workspace IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_ids: Option<Vec<String>>,
    /// Optional item count limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl MessageUsageReportParams {
    /// Create usage-report parameters.
    pub fn new(starting_at: DateTime<Utc>) -> Self {
        Self {
            starting_at,
            ending_at: None,
            api_key_ids: None,
            bucket_width: None,
            context_window: None,
            group_by: None,
            inference_geos: None,
            models: None,
            page: None,
            service_tiers: None,
            speeds: None,
            workspace_ids: None,
            limit: None,
        }
    }

    /// Set report ending timestamp.
    pub fn ending_at(mut self, ending_at: DateTime<Utc>) -> Self {
        self.ending_at = Some(ending_at);
        self
    }

    /// Filter by API key IDs.
    pub fn api_key_ids(mut self, api_key_ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.api_key_ids = Some(api_key_ids.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Set bucket width (for example `1h` or `1d`).
    pub fn bucket_width(mut self, bucket_width: impl Into<String>) -> Self {
        self.bucket_width = Some(bucket_width.into());
        self
    }

    /// Filter by context window identifiers.
    pub fn context_window(
        mut self,
        context_window: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.context_window = Some(context_window.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Set grouping dimensions.
    pub fn group_by(mut self, group_by: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.group_by = Some(group_by.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Filter by inference geographies.
    pub fn inference_geos(
        mut self,
        inference_geos: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.inference_geos = Some(inference_geos.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Filter by model IDs.
    pub fn models(mut self, models: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.models = Some(models.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Set pagination token.
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Filter by service tiers.
    pub fn service_tiers(
        mut self,
        service_tiers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.service_tiers = Some(service_tiers.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Filter by speeds.
    pub fn speeds(mut self, speeds: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.speeds = Some(speeds.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Filter by workspace IDs.
    pub fn workspace_ids(
        mut self,
        workspace_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.workspace_ids = Some(workspace_ids.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Set result limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query parameters for `/organizations/cost_report`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageCostReportParams {
    /// Inclusive start timestamp for the report window.
    pub starting_at: DateTime<Utc>,
    /// Exclusive end timestamp for the report window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_at: Option<DateTime<Utc>>,
    /// Optional report granularity / bucket width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket_width: Option<String>,
    /// Optional fields to group by.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_by: Option<Vec<String>>,
    /// Optional invoice ID filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_id: Option<String>,
    /// Optional pagination token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// Filter by service tiers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tiers: Option<Vec<String>>,
    /// Filter by workspace IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_ids: Option<Vec<String>>,
    /// Optional item count limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

impl MessageCostReportParams {
    /// Create cost-report parameters.
    pub fn new(starting_at: DateTime<Utc>) -> Self {
        Self {
            starting_at,
            ending_at: None,
            bucket_width: None,
            group_by: None,
            invoice_id: None,
            page: None,
            service_tiers: None,
            workspace_ids: None,
            limit: None,
        }
    }

    /// Set report ending timestamp.
    pub fn ending_at(mut self, ending_at: DateTime<Utc>) -> Self {
        self.ending_at = Some(ending_at);
        self
    }

    /// Set bucket width (for example `1h` or `1d`).
    pub fn bucket_width(mut self, bucket_width: impl Into<String>) -> Self {
        self.bucket_width = Some(bucket_width.into());
        self
    }

    /// Set grouping dimensions.
    pub fn group_by(mut self, group_by: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.group_by = Some(group_by.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Filter by invoice ID.
    pub fn invoice_id(mut self, invoice_id: impl Into<String>) -> Self {
        self.invoice_id = Some(invoice_id.into());
        self
    }

    /// Set pagination token.
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Filter by service tiers.
    pub fn service_tiers(
        mut self,
        service_tiers: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.service_tiers = Some(service_tiers.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Filter by workspace IDs.
    pub fn workspace_ids(
        mut self,
        workspace_ids: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.workspace_ids = Some(workspace_ids.into_iter().map(|v| v.into()).collect());
        self
    }

    /// Set result limit.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Usage-report bucket for messages usage endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageUsageReportBucket {
    /// Bucket start timestamp.
    #[serde(default)]
    pub starting_at: Option<DateTime<Utc>>,
    /// Bucket end timestamp.
    #[serde(default)]
    pub ending_at: Option<DateTime<Utc>>,
    /// Message request count.
    #[serde(default)]
    pub request_count: Option<u64>,
    /// Input tokens.
    #[serde(default)]
    pub input_tokens: Option<u64>,
    /// Output tokens.
    #[serde(default)]
    pub output_tokens: Option<u64>,
    /// Cache writes.
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
    /// Cache reads.
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
    /// Additional fields returned by grouped reports.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Cost-report bucket for messages cost endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageCostReportBucket {
    /// Bucket start timestamp.
    #[serde(default)]
    pub starting_at: Option<DateTime<Utc>>,
    /// Bucket end timestamp.
    #[serde(default)]
    pub ending_at: Option<DateTime<Utc>>,
    /// Additional dynamic cost breakdown fields.
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Query parameters for `/organizations/usage_report/claude_code`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaudeCodeUsageReportParams {
    /// Inclusive report start date.
    pub starting_at: chrono::NaiveDate,
    /// Exclusive report end date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ending_at: Option<chrono::NaiveDate>,
    /// Optional page size.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// Optional pagination token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<String>,
    /// Optional customer organization filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    /// Optional customer type filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_type: Option<String>,
    /// Optional terminal type filter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal_type: Option<String>,
}

impl ClaudeCodeUsageReportParams {
    /// Create params with a required start date.
    pub fn new(starting_at: chrono::NaiveDate) -> Self {
        Self {
            starting_at,
            ending_at: None,
            limit: None,
            page: None,
            organization_id: None,
            customer_type: None,
            terminal_type: None,
        }
    }

    /// Set report end date.
    pub fn ending_at(mut self, ending_at: chrono::NaiveDate) -> Self {
        self.ending_at = Some(ending_at);
        self
    }

    /// Set pagination size.
    pub fn limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set pagination token.
    pub fn page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Filter by organization.
    pub fn organization_id(mut self, organization_id: impl Into<String>) -> Self {
        self.organization_id = Some(organization_id.into());
        self
    }

    /// Filter by customer type.
    pub fn customer_type(mut self, customer_type: impl Into<String>) -> Self {
        self.customer_type = Some(customer_type.into());
        self
    }

    /// Filter by terminal type.
    pub fn terminal_type(mut self, terminal_type: impl Into<String>) -> Self {
        self.terminal_type = Some(terminal_type.into());
        self
    }
}

/// Actor info for Claude Code usage reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClaudeCodeUsageActor {
    /// Actor type.
    #[serde(rename = "type", default)]
    pub actor_type: Option<String>,
    /// User email address, when available.
    #[serde(default)]
    pub email_address: Option<String>,
    /// API key name, when available.
    #[serde(default)]
    pub api_key_name: Option<String>,
    /// Additional actor fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Core Claude Code metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClaudeCodeCoreMetrics {
    /// Number of sessions.
    #[serde(default)]
    pub num_sessions: Option<u64>,
    /// Number of lines of code added.
    #[serde(default)]
    pub num_lines_of_code_added: Option<u64>,
    /// Number of lines of code removed.
    #[serde(default)]
    pub num_lines_of_code_removed: Option<u64>,
    /// Number of commits made by Claude Code.
    #[serde(default)]
    pub num_commits_by_claude_code: Option<u64>,
    /// Number of pull requests created by Claude Code.
    #[serde(default)]
    pub num_pull_requests_created_by_claude_code: Option<u64>,
    /// Additional metrics not yet explicitly modeled.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Per-tool accept/reject metrics in Claude Code reporting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClaudeCodeToolMetric {
    /// Accepted suggestion count.
    #[serde(default, alias = "accepted")]
    pub accepted_count: Option<u64>,
    /// Rejected suggestion count.
    #[serde(default, alias = "rejected")]
    pub rejected_count: Option<u64>,
    /// Additional tool metric fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Claude Code usage report row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClaudeCodeUsageReportRow {
    /// Report date.
    #[serde(default)]
    pub date: Option<chrono::NaiveDate>,
    /// Actor metadata.
    #[serde(default)]
    pub actor: Option<ClaudeCodeUsageActor>,
    /// Core usage metrics.
    #[serde(default)]
    pub core_metrics: Option<ClaudeCodeCoreMetrics>,
    /// Tool-level metrics.
    #[serde(default)]
    pub tool_metrics: Option<HashMap<String, ClaudeCodeToolMetric>>,
    /// Additional row fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Messages usage report response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageUsageReportResponse {
    /// Report data buckets.
    #[serde(default)]
    pub data: Vec<MessageUsageReportBucket>,
    /// Indicates whether another page is available.
    #[serde(default)]
    pub has_more: bool,
    /// Pagination token for the next page.
    #[serde(default)]
    pub next_page: Option<String>,
}

/// Messages cost report response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct MessageCostReportResponse {
    /// Report data buckets.
    #[serde(default)]
    pub data: Vec<MessageCostReportBucket>,
    /// Indicates whether another page is available.
    #[serde(default)]
    pub has_more: bool,
    /// Pagination token for the next page.
    #[serde(default)]
    pub next_page: Option<String>,
}

/// Claude Code usage report response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ClaudeCodeUsageReportResponse {
    /// Report data rows.
    #[serde(default)]
    pub data: Vec<ClaudeCodeUsageReportRow>,
    /// Indicates whether another page is available.
    #[serde(default)]
    pub has_more: bool,
    /// Pagination token for the next page.
    #[serde(default)]
    pub next_page: Option<String>,
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use serde_json::json;

    #[test]
    fn test_message_usage_report_params_builder() {
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap();

        let params = MessageUsageReportParams::new(start)
            .ending_at(end)
            .bucket_width("1h")
            .group_by(["model", "workspace_id"])
            .models(["claude-sonnet-4-5"])
            .page("page_1")
            .limit(100);

        assert_eq!(params.bucket_width.as_deref(), Some("1h"));
        assert_eq!(params.group_by.unwrap().len(), 2);
        assert_eq!(params.page.as_deref(), Some("page_1"));
        assert_eq!(
            params
                .models
                .as_ref()
                .and_then(|m| m.first())
                .map(|m| m.as_str()),
            Some("claude-sonnet-4-5")
        );
        assert_eq!(params.limit, Some(100));
    }

    #[test]
    fn test_message_usage_report_response_deserialization() {
        let response: MessageUsageReportResponse = serde_json::from_value(json!({
            "data": [{
                "starting_at": "2026-01-01T00:00:00Z",
                "ending_at": "2026-01-01T01:00:00Z",
                "request_count": 42,
                "input_tokens": 1200,
                "output_tokens": 500,
                "workspace_id": "ws_123"
            }],
            "has_more": false,
            "next_page": "page_2"
        }))
        .unwrap();

        assert_eq!(response.data.len(), 1);
        assert_eq!(response.next_page.as_deref(), Some("page_2"));
        assert_eq!(response.data[0].request_count, Some(42));
        assert_eq!(
            response.data[0].extra.get("workspace_id"),
            Some(&json!("ws_123"))
        );
    }

    #[test]
    fn test_message_cost_report_response_deserialization() {
        let response: MessageCostReportResponse = serde_json::from_value(json!({
            "data": [{
                "starting_at": "2026-01-01T00:00:00Z",
                "ending_at": "2026-01-01T01:00:00Z",
                "amount": "1.23",
                "currency": "USD"
            }],
            "has_more": false,
            "next_page": null
        }))
        .unwrap();

        assert_eq!(response.data.len(), 1);
        assert!(response.next_page.is_none());
        assert_eq!(response.data[0].extra.get("currency"), Some(&json!("USD")));
    }

    #[test]
    fn test_claude_code_usage_report_response_deserialization() {
        let response: ClaudeCodeUsageReportResponse = serde_json::from_value(json!({
            "data": [{
                "date": "2026-01-01",
                "actor": {
                    "type": "user",
                    "email_address": "dev@example.com"
                },
                "core_metrics": {
                    "num_sessions": 4,
                    "num_lines_of_code_added": 120
                },
                "tool_metrics": {
                    "edit_file": {
                        "accepted_count": 8,
                        "rejected_count": 1
                    }
                }
            }],
            "has_more": false
        }))
        .unwrap();

        assert_eq!(response.data.len(), 1);
        assert_eq!(
            response.data[0]
                .actor
                .as_ref()
                .and_then(|a| a.email_address.as_deref()),
            Some("dev@example.com")
        );
    }

    #[test]
    fn test_invite_create_request_serialization() {
        let req = InviteCreateRequest::new("user@example.com", InviteCreateRole::Developer);
        let json = serde_json::to_value(req).unwrap();

        assert_eq!(json["email"], "user@example.com");
        assert_eq!(json["role"], "developer");
    }

    #[test]
    fn test_user_list_params_builder() {
        let params = UserListParams::new()
            .with_limit(10)
            .with_after_id("after_1")
            .with_before_id("before_1")
            .with_email("user@example.com");

        assert_eq!(params.limit, Some(10));
        assert_eq!(params.after_id.as_deref(), Some("after_1"));
        assert_eq!(params.before_id.as_deref(), Some("before_1"));
        assert_eq!(params.email.as_deref(), Some("user@example.com"));
    }

    #[test]
    fn test_workspace_member_create_request_serialization() {
        let req = WorkspaceMemberCreateRequest::new(
            "usr_123",
            WorkspaceMemberCreateRole::WorkspaceDeveloper,
        );
        let json = serde_json::to_value(req).unwrap();

        assert_eq!(json["user_id"], "usr_123");
        assert_eq!(json["workspace_role"], "workspace_developer");
    }
}
