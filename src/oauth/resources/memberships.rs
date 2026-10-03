//! Workspace-addressed service-account memberships.

use super::{models::*, validation::*};
use crate::{
    error::Result,
    types::{HttpMethod, PageStream, PaginationLimits, RequestOptions},
};

use crate::oauth::OAuthAdminClient;

/// Factories for workspace-side OAuth administration.
#[derive(Clone)]
pub struct OAuthWorkspacesApi {
    pub(super) client: OAuthAdminClient,
}

impl OAuthWorkspacesApi {
    /// Select a workspace's service-account memberships.
    pub fn service_accounts(&self, workspace_id: &str) -> Result<OAuthWorkspaceServiceAccountsApi> {
        validate_id(workspace_id)?;
        Ok(OAuthWorkspaceServiceAccountsApi {
            client: self.client.clone(),
            workspace_id: workspace_id.to_string(),
        })
    }
}

/// Workspace-addressed service-account membership lifecycle.
#[derive(Clone)]
pub struct OAuthWorkspaceServiceAccountsApi {
    pub(super) client: OAuthAdminClient,
    workspace_id: String,
}

impl OAuthWorkspaceServiceAccountsApi {
    fn path(&self, account_id: Option<&str>) -> Result<String> {
        let mut path = format!(
            "/organizations/workspaces/{}/service_accounts",
            self.workspace_id
        );
        if let Some(id) = account_id {
            validate_id(id)?;
            path.push('/');
            path.push_str(id);
        }
        Ok(path)
    }

    /// Retrieve membership, including the implicit default membership if applicable.
    pub async fn get(
        &self,
        account_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccountWorkspace> {
        self.client
            .request(
                HttpMethod::Get,
                &self.path(Some(account_id))?,
                None,
                options,
            )
            .await
    }

    /// List one page of explicit memberships; implicit default membership is omitted.
    pub async fn list(
        &self,
        params: OAuthListParams,
        options: Option<RequestOptions>,
    ) -> Result<OAuthPage<ServiceAccountWorkspace>> {
        let query = workspace_query(params)?;
        self.client
            .request(
                HttpMethod::Get,
                &format!("{}{query}", self.path(None)?),
                None,
                options,
            )
            .await
    }

    /// Add an explicit membership, replacing its role if it already exists.
    pub async fn add(
        &self,
        account_id: &str,
        role: &str,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccountWorkspace> {
        validate_id(account_id)?;
        validate_workspace_role(role)?;
        self.client
            .request(
                HttpMethod::Post,
                &self.path(None)?,
                Some(serde_json::json!({"service_account_id":account_id,"workspace_role":role})),
                options,
            )
            .await
    }

    /// Update an explicit membership's role. Implicit memberships require `add` first.
    pub async fn update(
        &self,
        account_id: &str,
        role: &str,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccountWorkspace> {
        validate_workspace_role(role)?;
        self.client
            .request(
                HttpMethod::Post,
                &self.path(Some(account_id))?,
                Some(serde_json::json!({"workspace_role":role})),
                options,
            )
            .await
    }

    /// Remove membership idempotently; implicit default membership removal is a no-op.
    pub async fn remove(
        &self,
        account_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<ServiceAccountWorkspaceDeleted> {
        self.client
            .request(
                HttpMethod::Delete,
                &self.path(Some(account_id))?,
                None,
                options,
            )
            .await
    }

    /// Fetch pages lazily with bounded ceilings and preserved request options.
    pub fn pages(
        &self,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<ServiceAccountWorkspace>> {
        workspace_query(params.clone())?;
        let api = self.clone();
        let initial = params.page.clone();
        crate::api::utils::paginate(limits, initial, move |cursor| {
            let api = api.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let response = api.list(params, options).await?;
                let item_ids = response
                    .data
                    .iter()
                    .map(|member| member.service_account_id.clone())
                    .collect();
                Ok(crate::api::utils::TraversalPage {
                    data: response.data,
                    next_cursor: response.next_page,
                    item_ids,
                })
            }
        })
    }

    /// Collect explicit memberships within finite ceilings.
    pub async fn list_all_with_limits(
        &self,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<ServiceAccountWorkspace>> {
        self.pages(params, limits, options)?.collect_items().await
    }
}
