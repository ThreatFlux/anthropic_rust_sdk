//! Workspace Admin API implementation

use crate::{
    api::utils::{build_path_with_query, create_default_pagination},
    client::Client,
    error::Result,
    models::admin::{
        Workspace, WorkspaceCreateRequest, WorkspaceListParams, WorkspaceListResponse,
        WorkspaceMember, WorkspaceMemberCreateRequest, WorkspaceMemberDeleteResponse,
        WorkspaceMemberListParams, WorkspaceMemberListResponse, WorkspaceMemberUpdateRequest,
        WorkspaceUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for Workspace admin endpoints
#[derive(Clone)]
pub struct WorkspaceApi {
    client: Client,
}

impl WorkspaceApi {
    /// Create a new Workspace API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// List workspaces
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceListResponse> {
        let mut params = WorkspaceListParams::new();
        if let Some(pagination) = pagination {
            if let Some(limit) = pagination.limit {
                params = params.with_limit(limit);
            }
            if let Some(after) = pagination.after {
                params = params.with_after_id(after);
            }
            if let Some(before) = pagination.before {
                params = params.with_before_id(before);
            }
        }
        self.list_with_params(params, options).await
    }

    /// List workspaces with full query controls.
    pub async fn list_with_params(
        &self,
        params: WorkspaceListParams,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceListResponse> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }
        if let Some(after_id) = params.after_id {
            query.push(format!("after_id={}", after_id));
        }
        if let Some(before_id) = params.before_id {
            query.push(format!("before_id={}", before_id));
        }
        if let Some(include_archived) = params.include_archived {
            query.push(format!("include_archived={}", include_archived));
        }

        let path = build_path_with_query("/organizations/workspaces", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific workspace
    pub async fn get(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organizations/workspaces/{}", workspace_id);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Create a new workspace
    pub async fn create(
        &self,
        request: WorkspaceCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(
                HttpMethod::Post,
                "/organizations/workspaces",
                Some(body),
                options,
            )
            .await
    }

    /// Update a workspace
    pub async fn update(
        &self,
        workspace_id: &str,
        request: WorkspaceUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organizations/workspaces/{}", workspace_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(HttpMethod::Post, &path, Some(body), options)
            .await
    }

    /// Delete a workspace
    pub async fn delete(&self, workspace_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/organizations/workspaces/{}", workspace_id);
        let _: serde_json::Value = self
            .client
            .request_admin(HttpMethod::Delete, &path, None, options)
            .await?;
        Ok(())
    }

    /// Archive a workspace
    pub async fn archive(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organizations/workspaces/{}/archive", workspace_id);
        self.client
            .request_admin(HttpMethod::Post, &path, None, options)
            .await
    }

    /// Restore an archived workspace
    pub async fn restore(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Workspace> {
        let path = format!("/organizations/workspaces/{}/restore", workspace_id);
        self.client
            .request_admin(HttpMethod::Post, &path, None, options)
            .await
    }

    /// List members in a workspace.
    pub async fn list_members(
        &self,
        workspace_id: &str,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceMemberListResponse> {
        let mut params = WorkspaceMemberListParams::new();
        if let Some(pagination) = pagination {
            if let Some(limit) = pagination.limit {
                params = params.with_limit(limit);
            }
            if let Some(after) = pagination.after {
                params = params.with_after_id(after);
            }
            if let Some(before) = pagination.before {
                params = params.with_before_id(before);
            }
        }

        self.list_members_with_params(workspace_id, params, options)
            .await
    }

    /// List members in a workspace with full query controls.
    pub async fn list_members_with_params(
        &self,
        workspace_id: &str,
        params: WorkspaceMemberListParams,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceMemberListResponse> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }
        if let Some(after_id) = params.after_id {
            query.push(format!("after_id={}", after_id));
        }
        if let Some(before_id) = params.before_id {
            query.push(format!("before_id={}", before_id));
        }

        let base_path = format!("/organizations/workspaces/{}/members", workspace_id);
        let path = build_path_with_query(&base_path, query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific workspace member.
    pub async fn get_member(
        &self,
        workspace_id: &str,
        user_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceMember> {
        let path = format!(
            "/organizations/workspaces/{}/members/{}",
            workspace_id, user_id
        );
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Add a user to a workspace.
    pub async fn add_member(
        &self,
        workspace_id: &str,
        request: WorkspaceMemberCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceMember> {
        let path = format!("/organizations/workspaces/{}/members", workspace_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(HttpMethod::Post, &path, Some(body), options)
            .await
    }

    /// Update a workspace member role.
    pub async fn update_member(
        &self,
        workspace_id: &str,
        user_id: &str,
        request: WorkspaceMemberUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceMember> {
        let path = format!(
            "/organizations/workspaces/{}/members/{}",
            workspace_id, user_id
        );
        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(HttpMethod::Post, &path, Some(body), options)
            .await
    }

    /// Remove a user from a workspace.
    pub async fn remove_member(
        &self,
        workspace_id: &str,
        user_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<WorkspaceMemberDeleteResponse> {
        let path = format!(
            "/organizations/workspaces/{}/members/{}",
            workspace_id, user_id
        );
        self.client
            .request_admin(HttpMethod::Delete, &path, None, options)
            .await
    }

    /// List all workspaces (convenience method)
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<Workspace>> {
        let mut all_workspaces = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self.list(Some(pagination), options.clone()).await?;

            all_workspaces.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_workspaces)
    }

    /// List all members in a workspace (convenience method).
    pub async fn list_all_members(
        &self,
        workspace_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Vec<WorkspaceMember>> {
        let mut all_members = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self
                .list_members(workspace_id, Some(pagination), options.clone())
                .await?;

            all_members.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_members)
    }
}
