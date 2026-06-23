//! Managed Agents API (beta: managed-agents-2026-04-01).
//!
//! This module groups the full Managed Agents surface: agents, environments,
//! sessions (with events, resources, and threads), vaults/credentials, memory
//! stores, and deployments. Every request forces the
//! `managed-agents-2026-04-01` beta header via the internal
//! `with_managed_agents_beta` helper.

pub mod agents;
pub mod deployments;
pub mod environments;
pub mod memory_stores;
pub mod session_events;
pub mod session_resources;
pub mod session_threads;
pub mod sessions;
pub mod vaults;

pub use agents::AgentsApi;
pub use deployments::{DeploymentRunsApi, DeploymentsApi};
pub use environments::EnvironmentsApi;
pub use memory_stores::{MemoriesApi, MemoryStoresApi};
pub use session_events::SessionEventsApi;
pub use session_resources::SessionResourcesApi;
pub use session_threads::SessionThreadsApi;
pub use sessions::SessionsApi;
pub use vaults::{CredentialsApi, VaultsApi};

use crate::types::RequestOptions;

/// Force the managed-agents beta header onto a request.
///
/// Mirrors `SkillsApi::with_skills_beta`: it guarantees the beta feature is
/// present even when the caller passes `None`, and de-dups so the feature is
/// never added twice if the caller already enabled it.
pub(crate) fn with_managed_agents_beta(options: Option<RequestOptions>) -> Option<RequestOptions> {
    let mut options = options.unwrap_or_default();
    if !options
        .beta_features
        .iter()
        .any(|f| f == crate::client::beta_headers::MANAGED_AGENTS)
    {
        options = options.with_managed_agents();
    }
    Some(options)
}
