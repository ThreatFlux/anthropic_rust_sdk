//! Admin API modules

pub mod api_keys;
pub mod organization;
pub mod usage;
pub mod workspace;

use crate::client::Client;

/// Admin API client (requires admin key)
#[derive(Clone)]
pub struct AdminApi {
    client: Client,
}

impl AdminApi {
    /// Create a new Admin API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Access organization endpoints
    pub fn organization(&self) -> organization::OrganizationApi {
        organization::OrganizationApi::new(self.client.clone())
    }

    /// Access organization user/invite endpoints.
    pub fn users(&self) -> organization::OrganizationApi {
        self.organization()
    }

    /// Backwards-compatible alias for user/member management.
    pub fn members(&self) -> organization::OrganizationApi {
        self.organization()
    }

    /// Access workspace endpoints  
    pub fn workspaces(&self) -> workspace::WorkspaceApi {
        workspace::WorkspaceApi::new(self.client.clone())
    }

    /// Access API keys endpoints
    pub fn api_keys(&self) -> api_keys::ApiKeysApi {
        api_keys::ApiKeysApi::new(self.client.clone())
    }

    /// Access usage endpoints
    pub fn usage(&self) -> usage::UsageApi {
        usage::UsageApi::new(self.client.clone())
    }
}
