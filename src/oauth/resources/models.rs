//! OAuth organization resource wire models and mutation parameters.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

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
    pub(super) kind: String,
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
    pub(super) kind: String,
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
    pub(super) kind: String,
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
    pub(super) kind: String,
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
