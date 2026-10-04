//! Federation rule lifecycle and workspace bindings.

use super::{models::*, validation::*};
use crate::{
    error::{AnthropicError, Result},
    types::{HttpMethod, RequestOptions},
};

use super::collection::OAuthResources;

impl OAuthResources<FederationRule> {
    /// Create a workspace developer/inference rule. Other scopes require the Console.
    pub async fn create(
        &self,
        request: RuleCreate,
        options: Option<RequestOptions>,
    ) -> Result<FederationRule> {
        validate_name(&request.name)?;
        validate_id(&request.issuer_id)?;
        validate_id(&request.target.service_account_id)?;
        validate_scope(&request.oauth_scope)?;
        validate_lifetime(request.token_lifetime_seconds)?;
        validate_target(&request.target)?;
        validate_match(&request.r#match)?;
        if request.workspace_id.is_none() && request.applies_to_all_workspaces != Some(true) {
            return Err(AnthropicError::invalid_input(
                "A workspace or applies_to_all_workspaces is required",
            ));
        }
        if let Some(workspace) = &request.workspace_id {
            validate_id(workspace)?;
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
    /// Update workspace-scoped fields. OAuth cannot modify broader-scope rules.
    pub async fn update(
        &self,
        id: &str,
        request: RuleUpdate,
        options: Option<RequestOptions>,
    ) -> Result<FederationRule> {
        if let Some(name) = &request.name {
            validate_name(name)?;
        }
        if let Some(scope) = &request.oauth_scope {
            validate_scope(scope)?;
        }
        if let Some(target) = &request.target {
            validate_target(target)?;
        }
        if let Some(matcher) = &request.r#match {
            validate_match(matcher)?;
        }
        if let Some(workspace) = &request.workspace_id {
            validate_id(workspace)?;
        }
        validate_lifetime(request.token_lifetime_seconds)?;
        self.client
            .request(
                HttpMethod::Post,
                &Self::path(Some(id))?,
                Some(serde_json::to_value(request)?),
                options,
            )
            .await
    }
    /// Retrieve the full workspace set (this endpoint is not paginated).
    pub async fn workspaces(
        &self,
        id: &str,
        options: Option<RequestOptions>,
    ) -> Result<OAuthPage<FederationRuleWorkspace>> {
        self.client
            .request(
                HttpMethod::Get,
                &format!("{}/workspaces", Self::path(Some(id))?),
                None,
                options,
            )
            .await
    }
    /// Enable a rule in a workspace.
    pub async fn add_workspace(
        &self,
        id: &str,
        workspace: &str,
        options: Option<RequestOptions>,
    ) -> Result<FederationRuleWorkspace> {
        validate_id(workspace)?;
        self.client
            .request(
                HttpMethod::Post,
                &format!("{}/workspaces", Self::path(Some(id))?),
                Some(serde_json::json!({"workspace_id":workspace})),
                options,
            )
            .await
    }
    /// Disable a rule in a workspace.
    pub async fn remove_workspace(
        &self,
        id: &str,
        workspace: &str,
        options: Option<RequestOptions>,
    ) -> Result<FederationRuleWorkspaceDeleted> {
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
