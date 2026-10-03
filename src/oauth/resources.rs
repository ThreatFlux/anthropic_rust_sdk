//! OAuth-only service-account and federation administration.
//!
//! Wire contracts are pinned to Anthropic Python SDK `18f25547f20cf5f01da69ac611e700e3bc9ebf21`.
//! Workspace-scoped federation rules can be changed here; broader-scope rules
//! must be bootstrapped in the Console. The server enforces caller permissions.

use super::OAuthAdminClient;
use crate::{
    error::{AnthropicError, Result},
    types::{HttpMethod, PageStream, PaginationLimits, RequestOptions},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashMap, marker::PhantomData};

/// Paginated OAuth resource response. Unknown metadata is retained.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
#[non_exhaustive]
pub struct OAuthPage<T> {
    /// Resource data.
    pub data: Vec<T>,
    /// Next page token; absent/null terminates iteration.
    #[serde(default)]
    pub next_page: Option<String>,
    /// Additional response metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Named workload identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ServiceAccount {
    /// Tagged service-account ID.
    pub id: String,
    /// Resource name.
    pub name: String,
    /// Organization role; unfamiliar response values remain readable.
    pub organization_role: String,
    /// Resource discriminator.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Optional description.
    #[serde(default)]
    pub description: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Update timestamp.
    pub updated_at: String,
    /// Archive timestamp.
    #[serde(default)]
    pub archived_at: Option<String>,
    /// Actor that created the resource, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_actor_id: Option<String>,
    /// Actor that last updated the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by_actor_id: Option<String>,
    /// Actor that archived the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_by_actor_id: Option<String>,
    /// Additional response fields, including actor identities.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// JWKS configuration, retaining future discriminator values and payload fields.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
#[non_exhaustive]
pub enum Jwks {
    /// Discover JWKS using OIDC metadata.
    Discovery(JwksDiscovery),
    /// Retrieve a specified JWKS URL.
    ExplicitUrl(JwksExplicitUrl),
    /// An inline key set.
    Inline(JwksInline),
    /// Future provider configuration, preserved as JSON.
    Unknown(Value),
}

/// OIDC discovery configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct JwksDiscovery {
    #[serde(rename = "type")]
    kind: String,
    /// Optional discovery base URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discovery_base: Option<String>,
    /// Optional PEM certificate bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_cert_pem: Option<String>,
    /// Extension fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Explicit JWKS endpoint configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct JwksExplicitUrl {
    #[serde(rename = "type")]
    kind: String,
    /// JWKS URL.
    pub url: String,
    /// Optional PEM certificate bundle.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ca_cert_pem: Option<String>,
    /// Extension fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Inline JWKS configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct JwksInline {
    #[serde(rename = "type")]
    kind: String,
    /// Public JSON Web Keys.
    pub keys: Vec<HashMap<String, Value>>,
    /// Extension fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

impl<'de> Deserialize<'de> for Jwks {
    fn deserialize<D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> std::result::Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        let kind = value
            .as_object()
            .and_then(|object| object.get("type"))
            .and_then(Value::as_str)
            .ok_or_else(|| {
                serde::de::Error::custom("JWKS requires an object with a string type")
            })?;
        match kind {
            "discovery" => serde_json::from_value(value).map(Self::Discovery),
            "explicit_url" => serde_json::from_value(value).map(Self::ExplicitUrl),
            "inline" => serde_json::from_value(value).map(Self::Inline),
            _ => return Ok(Self::Unknown(value)),
        }
        .map_err(serde::de::Error::custom)
    }
}

impl Jwks {
    /// Use OIDC discovery with default settings.
    pub fn discovery() -> Self {
        Self::Discovery(JwksDiscovery {
            kind: "discovery".into(),
            discovery_base: None,
            ca_cert_pem: None,
            extra: HashMap::new(),
        })
    }
    /// Retrieve JWKS from an explicit HTTPS URL.
    pub fn explicit_url(url: impl Into<String>) -> Self {
        Self::ExplicitUrl(JwksExplicitUrl {
            kind: "explicit_url".into(),
            url: url.into(),
            ca_cert_pem: None,
            extra: HashMap::new(),
        })
    }
    /// Supply an inline public key set.
    pub fn inline(keys: Vec<HashMap<String, Value>>) -> Self {
        Self::Inline(JwksInline {
            kind: "inline".into(),
            keys,
            extra: HashMap::new(),
        })
    }
}

/// Registered federation identity provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FederationIssuer {
    /// Tagged issuer ID.
    pub id: String,
    /// Resource name.
    pub name: String,
    /// OIDC issuer URL.
    pub issuer_url: String,
    /// Key retrieval configuration.
    pub jwks: Jwks,
    /// Check token replay identifiers.
    pub check_jti: bool,
    /// Maximum subject-token lifetime.
    pub max_jwt_lifetime_seconds: u64,
    /// Discriminator.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Update timestamp.
    pub updated_at: String,
    /// Archive timestamp.
    #[serde(default)]
    pub archived_at: Option<String>,
    /// Actor that created the resource, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_actor_id: Option<String>,
    /// Actor that last updated the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by_actor_id: Option<String>,
    /// Actor that archived the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_by_actor_id: Option<String>,
    /// Timestamp when automatic JWKS polling was paused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jwks_polling_disabled_at: Option<String>,
    /// Current JWKS polling state; inline issuers usually have none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poll_status: Option<FederationIssuerPollStatus>,
    /// Additional actor/poll metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Current JWKS poll health reported by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FederationIssuerPollStatus {
    /// Consecutive fetch failures since the last successful poll.
    pub consecutive_failures: u64,
    /// Last successful fetch timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_fetched_at: Option<String>,
    /// Next scheduled poll; absent while paused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_poll_at: Option<String>,
    /// Future polling metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Claims matched by a federation rule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FederationMatch {
    /// Required JWT audience.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audience: Option<String>,
    /// Required claim values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claims: Option<HashMap<String, String>>,
    /// Provider-side condition expression.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Subject prefix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_prefix: Option<String>,
    /// Extension fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Federation rule target.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ServiceAccountTarget {
    #[serde(rename = "type")]
    kind: String,
    /// Account receiving the minted identity.
    pub service_account_id: String,
    /// Optional account name in responses.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service_account_name: Option<String>,
    /// Future target metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

impl ServiceAccountTarget {
    /// Select a workload account.
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            kind: "service_account".into(),
            service_account_id: id.into(),
            service_account_name: None,
            extra: HashMap::new(),
        }
    }
}

/// Federation rule and workspace bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FederationRule {
    /// Tagged rule ID.
    pub id: String,
    /// Name.
    pub name: String,
    /// Referenced issuer.
    pub issuer_id: String,
    /// Subject-token matching conditions.
    pub r#match: FederationMatch,
    /// Target workload identity.
    pub target: ServiceAccountTarget,
    /// Granted scope.
    pub oauth_scope: String,
    /// Issued-token lifetime.
    pub token_lifetime_seconds: u32,
    /// Whether enabled in every workspace.
    pub applies_to_all_workspaces: bool,
    /// Explicit workspace bindings.
    pub workspace_ids: Vec<String>,
    /// Discriminator.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Update timestamp.
    pub updated_at: String,
    /// Archive timestamp.
    #[serde(default)]
    pub archived_at: Option<String>,
    /// Actor that created the resource, when supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_actor_id: Option<String>,
    /// Actor that last updated the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by_actor_id: Option<String>,
    /// Actor that archived the resource.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_by_actor_id: Option<String>,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Issuer name at read time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issuer_name: Option<String>,
    /// Additional metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Organization role accepted on service-account mutations.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum OrganizationRole {
    /// Developer role, available to workloads.
    Developer,
    /// Admin role, requiring an interactive user credential.
    Admin,
}

/// Create a service account.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct ServiceAccountCreate {
    /// Unique slug.
    pub name: String,
    /// Role (defaults to developer).
    pub organization_role: OrganizationRole,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
impl ServiceAccountCreate {
    /// Create a developer account with the given slug.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            organization_role: OrganizationRole::Developer,
            description: None,
        }
    }
}

/// Service-account mutable fields; `name` cannot be changed.
#[derive(Debug, Clone, Default, Serialize)]
#[non_exhaustive]
pub struct ServiceAccountUpdate {
    /// Set or explicitly clear the description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
    /// Set a role. Admin promotion requires an interactive user credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_role: Option<OrganizationRole>,
}

/// Create a federation issuer.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct IssuerCreate {
    /// Unique slug.
    pub name: String,
    /// HTTPS OIDC issuer URL.
    pub issuer_url: String,
    /// Signing key retrieval configuration.
    pub jwks: Jwks,
    /// Whether subject JWT replay checks are enabled.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_jti: Option<bool>,
    /// Maximum subject JWT lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_jwt_lifetime_seconds: Option<u64>,
}
impl IssuerCreate {
    /// Register an OIDC issuer using discovery.
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            issuer_url: url.into(),
            jwks: Jwks::discovery(),
            check_jti: None,
            max_jwt_lifetime_seconds: None,
        }
    }
}

/// Mutable issuer fields. The server rejects issuers backing broader-scope rules.
#[derive(Debug, Clone, Default, Serialize)]
#[non_exhaustive]
pub struct IssuerUpdate {
    /// New slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// OIDC URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issuer_url: Option<String>,
    /// Signing keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks: Option<Jwks>,
    /// Replay checks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_jti: Option<bool>,
    /// Stop polling keys.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_polling_disabled: Option<bool>,
    /// Maximum subject JWT lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_jwt_lifetime_seconds: Option<u64>,
}

/// Create a workspace-scoped federation rule.
#[derive(Debug, Clone, Serialize)]
#[non_exhaustive]
pub struct RuleCreate {
    /// Slug.
    pub name: String,
    /// Issuer ID.
    pub issuer_id: String,
    /// JWT conditions.
    pub r#match: FederationMatch,
    /// Workload account target.
    pub target: ServiceAccountTarget,
    /// `workspace:developer` or `workspace:inference` for API callers.
    pub oauth_scope: String,
    /// Optional expiry override, 60–86400 seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_lifetime_seconds: Option<u32>,
    /// Enable in every workspace instead of one workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applies_to_all_workspaces: Option<bool>,
    /// Explicit workspace binding, required unless enabled in every workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}
impl RuleCreate {
    /// Configure a workspace developer rule with an explicit account and workspace.
    pub fn new(
        name: impl Into<String>,
        issuer: impl Into<String>,
        account: impl Into<String>,
        workspace: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            issuer_id: issuer.into(),
            r#match: FederationMatch::default(),
            target: ServiceAccountTarget::new(account),
            oauth_scope: "workspace:developer".into(),
            token_lifetime_seconds: None,
            applies_to_all_workspaces: None,
            workspace_id: Some(workspace.into()),
            description: None,
        }
    }
}

/// Mutable federation-rule fields.
#[derive(Debug, Clone, Default, Serialize)]
#[non_exhaustive]
pub struct RuleUpdate {
    /// Slug.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Conditions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#match: Option<FederationMatch>,
    /// Target.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ServiceAccountTarget>,
    /// Workspace-scoped OAuth permission.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth_scope: Option<String>,
    /// Lifetime.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_lifetime_seconds: Option<u32>,
    /// Enable globally.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub applies_to_all_workspaces: Option<bool>,
    /// Enable in a workspace.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    /// Set or clear description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<Option<String>>,
}

/// Page-token list parameters, with optional rule issuer filter.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct OAuthListParams {
    /// Page size, 1–100.
    pub limit: Option<u32>,
    /// Token from the preceding response.
    pub page: Option<String>,
    /// Include archived resources.
    pub include_archived: Option<bool>,
    /// Filter rules by issuer; unsupported on other collection types.
    pub issuer_id: Option<String>,
}

/// Common collection behavior for the three organization resources.
pub trait OAuthResource: serde::de::DeserializeOwned + Send + Sync + 'static {
    /// API collection path under `/organizations`.
    const COLLECTION: &'static str;
    /// Stable identifier used to detect pagination without item progress.
    fn resource_id(&self) -> &str;
}
impl OAuthResource for ServiceAccount {
    const COLLECTION: &'static str = "service_accounts";
    fn resource_id(&self) -> &str {
        &self.id
    }
}
impl OAuthResource for FederationIssuer {
    const COLLECTION: &'static str = "federation_issuers";
    fn resource_id(&self) -> &str {
        &self.id
    }
}
impl OAuthResource for FederationRule {
    const COLLECTION: &'static str = "federation_rules";
    fn resource_id(&self) -> &str {
        &self.id
    }
}

/// Typed OAuth collection client.
pub struct OAuthResources<T> {
    client: OAuthAdminClient,
    marker: PhantomData<T>,
}
impl<T> Clone for OAuthResources<T> {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            marker: PhantomData,
        }
    }
}

/// Organization factories sharing one credential cache and transport.
#[derive(Clone)]
pub struct OAuthOrganizationApi {
    client: OAuthAdminClient,
}
impl OAuthOrganizationApi {
    pub(crate) fn new(client: OAuthAdminClient) -> Self {
        Self { client }
    }
    /// Service-account lifecycle and workspace membership.
    pub fn service_accounts(&self) -> OAuthResources<ServiceAccount> {
        OAuthResources::new(self.client.clone())
    }
    /// Federation issuer lifecycle.
    pub fn federation_issuers(&self) -> OAuthResources<FederationIssuer> {
        OAuthResources::new(self.client.clone())
    }
    /// Federation rule lifecycle and workspace binding.
    pub fn federation_rules(&self) -> OAuthResources<FederationRule> {
        OAuthResources::new(self.client.clone())
    }
    /// Workspace-side service-account membership clients.
    pub fn workspaces(&self) -> OAuthWorkspacesApi {
        OAuthWorkspacesApi {
            client: self.client.clone(),
        }
    }
}

impl<T: OAuthResource> OAuthResources<T> {
    fn new(client: OAuthAdminClient) -> Self {
        Self {
            client,
            marker: PhantomData,
        }
    }
    fn path(id: Option<&str>) -> Result<String> {
        let mut path = format!("/organizations/{}", T::COLLECTION);
        if let Some(id) = id {
            validate_id(id)?;
            path.push('/');
            path.push_str(id);
        }
        Ok(path)
    }
    /// Retrieve one resource.
    pub async fn get(&self, id: &str, options: Option<RequestOptions>) -> Result<T> {
        self.client
            .request(HttpMethod::Get, &Self::path(Some(id))?, None, options)
            .await
    }
    /// Soft-archive a resource. Archive referencing rules before accounts/issuers.
    pub async fn archive(&self, id: &str, options: Option<RequestOptions>) -> Result<T> {
        self.client
            .request(
                HttpMethod::Post,
                &format!("{}/archive", Self::path(Some(id))?),
                None,
                options,
            )
            .await
    }
    /// Retrieve a page, preserving endpoint filters.
    pub async fn list(
        &self,
        params: OAuthListParams,
        options: Option<RequestOptions>,
    ) -> Result<OAuthPage<T>> {
        if params.page.as_deref() == Some("") {
            return Err(AnthropicError::invalid_input(
                "OAuth page token must not be empty",
            ));
        }
        if params
            .limit
            .is_some_and(|limit| !(1..=100).contains(&limit))
        {
            return Err(AnthropicError::invalid_input(
                "OAuth page limit must be 1–100",
            ));
        }
        if params.issuer_id.is_some() && T::COLLECTION != "federation_rules" {
            return Err(AnthropicError::invalid_input(
                "Issuer filtering is only supported for federation rules",
            ));
        }
        let query = {
            let mut query = url::form_urlencoded::Serializer::new(String::new());
            if let Some(limit) = params.limit {
                query.append_pair("limit", &limit.to_string());
            }
            if let Some(page) = params.page {
                query.append_pair("page", &page);
            }
            if let Some(value) = params.include_archived {
                query.append_pair("include_archived", &value.to_string());
            }
            if let Some(issuer) = params.issuer_id {
                query.append_pair("issuer_id", &issuer);
            }
            query.finish()
        };
        let path = Self::path(None)?;
        let path = if query.is_empty() {
            path
        } else {
            format!("{path}?{query}")
        };
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }
    /// Incremental bounded page traversal. Cursor cycles and limits return an error.
    pub fn pages(
        &self,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<PageStream<T>> {
        let api = self.clone();
        let initial_cursor = params.page.clone();
        crate::api::utils::paginate(limits, initial_cursor, move |cursor| {
            let api = api.clone();
            let mut params = params.clone();
            let options = options.clone();
            params.page = cursor;
            async move {
                let page = api.list(params, options).await?;
                let item_ids = page
                    .data
                    .iter()
                    .map(|item| item.resource_id().to_owned())
                    .collect();
                Ok(crate::api::utils::TraversalPage {
                    data: page.data,
                    next_cursor: page.next_page,
                    item_ids,
                })
            }
        })
    }
    /// Collect bounded traversal with explicit limits.
    pub async fn list_all_with_limits(
        &self,
        params: OAuthListParams,
        limits: PaginationLimits,
        options: Option<RequestOptions>,
    ) -> Result<Vec<T>> {
        self.pages(params, limits, options)?.collect_items().await
    }

    /// Collect resources with the finite default traversal ceilings.
    pub async fn list_all(&self, options: Option<RequestOptions>) -> Result<Vec<T>> {
        self.list_all_with_limits(
            OAuthListParams::default(),
            PaginationLimits::default(),
            options,
        )
        .await
    }
}

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

impl OAuthResources<FederationIssuer> {
    /// Register an issuer. Only discovery/JWKS fetch URLs require public HTTPS on port 443.
    pub async fn create(
        &self,
        request: IssuerCreate,
        options: Option<RequestOptions>,
    ) -> Result<FederationIssuer> {
        validate_name(&request.name)?;
        validate_issuer_url(&request.issuer_url, &request.jwks)?;
        validate_jwks(&request.jwks)?;
        validate_jwt_lifetime(request.max_jwt_lifetime_seconds)?;
        self.client
            .request(
                HttpMethod::Post,
                &Self::path(None)?,
                Some(serde_json::to_value(request)?),
                options,
            )
            .await
    }
    /// Update an issuer. Broader-scope rule references are checked by the server.
    pub async fn update(
        &self,
        id: &str,
        request: IssuerUpdate,
        options: Option<RequestOptions>,
    ) -> Result<FederationIssuer> {
        if let Some(name) = &request.name {
            validate_name(name)?;
        }
        if let Some(url) = &request.issuer_url {
            if let Some(jwks) = &request.jwks {
                validate_issuer_url(url, jwks)?;
            } else if url.is_empty() {
                return Err(AnthropicError::invalid_input(
                    "Issuer claim value must not be empty",
                ));
            }
            // The server knows the existing JWKS mode when an update omits it.
        }
        validate_jwt_lifetime(request.max_jwt_lifetime_seconds)?;
        if request.jwks_polling_disabled == Some(true) {
            return Err(AnthropicError::invalid_input(
                "jwks_polling_disabled accepts only false to resume polling",
            ));
        }
        if let Some(jwks) = &request.jwks {
            validate_jwks(jwks)?;
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
}

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

/// Service-account workspace membership.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct ServiceAccountWorkspace {
    /// Account ID.
    pub service_account_id: String,
    /// Workspace ID.
    pub workspace_id: String,
    /// Workspace role.
    pub workspace_role: String,
    /// Membership discriminator.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Optional implicit membership flag.
    #[serde(default)]
    pub implicit: Option<bool>,
    /// Actor that created this explicit membership, when available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_actor_id: Option<String>,
    /// Additional metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Federation rule workspace binding.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[non_exhaustive]
pub struct FederationRuleWorkspace {
    /// Rule ID.
    pub federation_rule_id: String,
    /// Workspace ID.
    pub workspace_id: String,
    /// Discriminator.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Creation timestamp.
    pub created_at: String,
    /// Actor that enabled the rule in this workspace.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by_actor_id: Option<String>,
    /// Workspace display name, populated when listing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    /// Additional metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

fn validate_id(id: &str) -> Result<()> {
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
    {
        return Err(AnthropicError::invalid_input("Invalid resource identifier"));
    }
    Ok(())
}
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || name.len() > 255
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(AnthropicError::invalid_input(
            "Resource names require 1–255 lowercase letters, digits or hyphens",
        ));
    }
    Ok(())
}
fn validate_scope(scope: &str) -> Result<()> {
    if !matches!(scope, "workspace:developer" | "workspace:inference") {
        return Err(AnthropicError::invalid_input("OAuth API callers can only modify workspace developer/inference rules; use the Console for other scopes"));
    }
    Ok(())
}
fn validate_lifetime(lifetime: Option<u32>) -> Result<()> {
    if lifetime.is_some_and(|value| !(60..=86400).contains(&value)) {
        return Err(AnthropicError::invalid_input(
            "Token lifetime must be 60–86400 seconds",
        ));
    }
    Ok(())
}
fn validate_provider_url(value: &str) -> Result<()> {
    let url = url::Url::parse(value)
        .map_err(|_| AnthropicError::invalid_input("Invalid provider URL"))?;
    if url.scheme() != "https"
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.host(),Some(url::Host::Domain(domain)) if domain.contains('.') && domain != "localhost")
    {
        return Err(AnthropicError::invalid_input(
            "Provider URLs require HTTPS public DNS names on port 443",
        ));
    }
    Ok(())
}
fn validate_jwks(jwks: &Jwks) -> Result<()> {
    match jwks {
        Jwks::Discovery(value) => {
            validate_kind_and_extra(
                &value.kind,
                "discovery",
                &value.extra,
                &["type", "discovery_base", "ca_cert_pem"],
            )?;
            if let Some(url) = &value.discovery_base {
                validate_provider_url(url)?;
            }
        }
        Jwks::ExplicitUrl(value) => {
            validate_kind_and_extra(
                &value.kind,
                "explicit_url",
                &value.extra,
                &["type", "url", "ca_cert_pem"],
            )?;
            validate_provider_url(&value.url)?;
        }
        Jwks::Inline(value) => {
            validate_kind_and_extra(&value.kind, "inline", &value.extra, &["type", "keys"])?;
        }
        Jwks::Unknown(_) => {
            return Err(AnthropicError::invalid_input(
                "Unknown JWKS configuration cannot be sent by typed mutation methods",
            ))
        }
    }
    Ok(())
}
fn workspace_query(params: OAuthListParams) -> Result<String> {
    if params.include_archived.is_some()
        || params.issuer_id.is_some()
        || params
            .limit
            .is_some_and(|limit| !(1..=100).contains(&limit))
        || params.page.as_deref() == Some("")
    {
        return Err(AnthropicError::invalid_input(
            "Unsupported workspace list options",
        ));
    }
    let mut query = url::form_urlencoded::Serializer::new(String::new());
    if let Some(page) = params.page {
        query.append_pair("page", &page);
    }
    if let Some(limit) = params.limit {
        query.append_pair("limit", &limit.to_string());
    }
    let query = query.finish();
    Ok(if query.is_empty() {
        query
    } else {
        format!("?{query}")
    })
}

fn validate_workspace_role(role: &str) -> Result<()> {
    if !matches!(
        role,
        "workspace_admin"
            | "workspace_developer"
            | "workspace_restricted_developer"
            | "workspace_user"
    ) {
        return Err(AnthropicError::invalid_input(
            "Unsupported non-billing workspace role",
        ));
    }
    Ok(())
}

fn validate_kind_and_extra(
    kind: &str,
    expected: &str,
    extra: &HashMap<String, Value>,
    reserved: &[&str],
) -> Result<()> {
    if kind != expected || reserved.iter().any(|key| extra.contains_key(*key)) {
        return Err(AnthropicError::invalid_input(
            "Typed federation payload has a conflicting discriminator or reserved extension key",
        ));
    }
    Ok(())
}

fn validate_target(target: &ServiceAccountTarget) -> Result<()> {
    validate_kind_and_extra(
        &target.kind,
        "service_account",
        &target.extra,
        &["type", "service_account_id", "service_account_name"],
    )?;
    validate_id(&target.service_account_id)
}

fn validate_match(matcher: &FederationMatch) -> Result<()> {
    if ["audience", "claims", "condition", "subject_prefix"]
        .iter()
        .any(|key| matcher.extra.contains_key(*key))
    {
        return Err(AnthropicError::invalid_input(
            "Federation matcher extension uses a reserved key",
        ));
    }
    let subject = matcher
        .subject_prefix
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty() && value.trim().chars().any(|ch| ch != '*'));
    let claims = matcher
        .claims
        .as_ref()
        .is_some_and(|claims| !claims.is_empty());
    let condition = matcher
        .condition
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty() && value.trim() != "true");
    if !subject && !claims && !condition {
        return Err(AnthropicError::invalid_input("Federation matching requires restrictive subject_prefix, claims, or condition; audience alone is insufficient"));
    }
    Ok(())
}

fn validate_jwt_lifetime(lifetime: Option<u64>) -> Result<()> {
    if lifetime.is_some_and(|value| !(1..=176_400).contains(&value)) {
        return Err(AnthropicError::invalid_input(
            "Maximum JWT lifetime must be between 1 and 176400 seconds",
        ));
    }
    Ok(())
}

fn validate_issuer_url(issuer: &str, jwks: &Jwks) -> Result<()> {
    if issuer.is_empty() {
        return Err(AnthropicError::invalid_input(
            "Issuer claim value must not be empty",
        ));
    }
    if let Jwks::Discovery(discovery) = jwks {
        if discovery.discovery_base.is_none() {
            validate_provider_url(issuer)?;
        }
    }
    Ok(())
}

/// Deleted membership response, shared by both workspace/account addressing paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ServiceAccountWorkspaceDeleted {
    /// Service-account ID named in the request.
    pub service_account_id: String,
    /// Workspace ID named in the request.
    pub workspace_id: String,
    /// `service_account_workspace_member_deleted`, or a future response value.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Future response fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Deleted federation workspace binding response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FederationRuleWorkspaceDeleted {
    /// Federation rule named in the request.
    pub federation_rule_id: String,
    /// Workspace named in the request.
    pub workspace_id: String,
    /// Deletion discriminator, retained verbatim.
    #[serde(rename = "type")]
    pub resource_type: String,
    /// Future response fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, Value>,
}

/// Factories for workspace-side OAuth administration.
#[derive(Clone)]
pub struct OAuthWorkspacesApi {
    client: OAuthAdminClient,
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
    client: OAuthAdminClient,
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
