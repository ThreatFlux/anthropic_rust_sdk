//! OAuth-only service-account and federation administration.
//!
//! Wire contracts are pinned to Anthropic Python SDK `18f25547f20cf5f01da69ac611e700e3bc9ebf21`.
//! Workspace-scoped federation rules can be changed here; broader-scope rules
//! must be bootstrapped in the Console. The server enforces caller permissions.

mod accounts;
mod collection;
mod issuers;
mod memberships;
mod models;
mod rules;
mod validation;

pub use collection::{OAuthOrganizationApi, OAuthResource, OAuthResources};
pub use memberships::{OAuthWorkspaceServiceAccountsApi, OAuthWorkspacesApi};
pub use models::*;
