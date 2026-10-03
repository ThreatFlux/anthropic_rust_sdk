//! Service-account lifecycle and account-addressed workspace memberships.

use super::{models::*, validation::*};
use crate::{
    error::Result,
    types::{HttpMethod, PageStream, PaginationLimits, RequestOptions},
};

use super::collection::OAuthResources;

impl OAuthResources<ServiceAccount> {
    /// Create an account. Admin-role creation requires an interactive OAuth user.
    pub async fn create(
        &self,
        request: ServiceAccountCreate,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccount> {
        validate_name(&request.name)?;
        if matches!(request.organization_role, OrganizationRole::Admin) {
            self.client.require_interactive_if_known().await?;
        }
        self.client
            .request(
                HttpMethod::Post,
                &Self::path(None)?,
                Some(serde_json::to_value(request)?),
                options,
            )
            .await
    }
    /// Update description/role; the server checks interactive-only admin promotion.
    pub async fn update(
        &self,
        id: &str,
        request: ServiceAccountUpdate,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccount> {
        if matches!(request.organization_role, Some(OrganizationRole::Admin)) {
            self.client.require_interactive_if_known().await?;
        }
        self.client
            .request(
                HttpMethod::Post,
                &Self::path(Some(id))?,
                Some(serde_json::to_value(request)?),
                options,
            )
            .await
    }
    /// Retrieve workspace memberships, including the implicit default membership.
    pub async fn workspaces(
        &self,
        id: &str,
        params: OAuthListParams,
        options: Option<RequestOptions>,
    ) -> Result<OAuthPage<ServiceAccountWorkspace>> {
        let query = workspace_query(params)?;
        self.client
            .request(
                HttpMethod::Get,
                &format!("{}/workspaces{query}", Self::path(Some(id))?),
                None,
                options,
            )
            .await
    }

    /// Traverse account-addressed memberships with finite ceilings.
    pub fn workspace_pages(
        &self,
        id: &str,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<ServiceAccountWorkspace>> {
        validate_id(id)?;
        workspace_query(params.clone())?;
        let api = self.clone();
        let id = id.to_string();
        let initial = params.page.clone();
        crate::api::utils::paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let id = id.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.workspaces(&id, params, options).await?;
                let item_ids = response
                    .data
                    .iter()
                    .map(|member| member.workspace_id.clone())
                    .collect();
                Ok(crate::api::utils::TraversalPage {
                    data: response.data,
                    next_cursor: response.next_page,
                    item_ids,
                })
            }
        })
    }

    /// Collect account memberships, failing at an explicit traversal ceiling.
    pub async fn list_all_workspaces_with_limits(
        &self,
        id: &str,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<ServiceAccountWorkspace>> {
        self.workspace_pages(id, params, limits, options)?
            .collect_items()
            .await
    }
    /// Add a workspace membership with an explicit non-billing role.
    pub async fn add_workspace(
        &self,
        id: &str,
        workspace: &str,
        role: &str,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccountWorkspace> {
        validate_id(workspace)?;
        validate_workspace_role(role)?;
        self.client
            .request(
                HttpMethod::Post,
                &format!("{}/workspaces", Self::path(Some(id))?),
                Some(serde_json::json!({"workspace_id":workspace,"workspace_role":role})),
                options,
            )
            .await
    }
    /// Remove an explicit workspace membership.
    pub async fn remove_workspace(
        &self,
        id: &str,
        workspace: &str,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccountWorkspaceDeleted> {
        validate_id(workspace)?;
        self.client
            .request(
                HttpMethod::Delete,
                &format!("{}/workspaces/{workspace}", Self::path(Some(id))?),
                None,
                options,
            )
            .await
    }
}
