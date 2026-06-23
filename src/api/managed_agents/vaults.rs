//! Managed Agents — Vaults & Credentials API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::vault::{
        Credential, CredentialCreateRequest, CredentialListResponse, CredentialUpdateRequest,
        Vault, VaultCreateRequest, VaultListResponse, VaultUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Vaults endpoints (`/v1/vaults`).
#[derive(Clone)]
pub struct VaultsApi {
    client: Client,
}

impl VaultsApi {
    /// Create a new Vaults API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a vault.
    pub async fn create(
        &self,
        request: VaultCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Vault> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/vaults",
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List vaults (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<VaultListResponse> {
        let path = build_paginated_path("/vaults", pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve a vault by id.
    pub async fn get(&self, vault_id: &str, options: Option<RequestOptions>) -> Result<Vault> {
        let path = format!("/vaults/{}", vault_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a vault.
    pub async fn update(
        &self,
        vault_id: &str,
        request: VaultUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Vault> {
        let path = format!("/vaults/{}", vault_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Delete a vault.
    pub async fn delete(&self, vault_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/vaults/{}", vault_id);
        let _: serde_json::Value = self
            .client
            .request(
                HttpMethod::Delete,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await?;
        Ok(())
    }

    /// Access the credentials sub-resource for a vault.
    pub fn credentials(&self, vault_id: &str) -> CredentialsApi {
        CredentialsApi::new(self.client.clone(), vault_id.to_string())
    }
}

/// API client for the Managed Agents — Credentials endpoints
/// (`/v1/vaults/{vault_id}/credentials`).
#[derive(Clone)]
pub struct CredentialsApi {
    client: Client,
    vault_id: String,
}

impl CredentialsApi {
    /// Create a new Credentials API client scoped to a vault.
    pub fn new(client: Client, vault_id: String) -> Self {
        Self { client, vault_id }
    }

    /// Create a credential in the vault.
    pub async fn create(
        &self,
        request: CredentialCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Credential> {
        let path = format!("/vaults/{}/credentials", self.vault_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List credentials in the vault (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<CredentialListResponse> {
        let base = format!("/vaults/{}/credentials", self.vault_id);
        let path = build_paginated_path(&base, pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve a credential by id.
    pub async fn get(
        &self,
        credential_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Credential> {
        let path = format!("/vaults/{}/credentials/{}", self.vault_id, credential_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a credential.
    pub async fn update(
        &self,
        credential_id: &str,
        request: CredentialUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Credential> {
        let path = format!("/vaults/{}/credentials/{}", self.vault_id, credential_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Delete a credential.
    pub async fn delete(&self, credential_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/vaults/{}/credentials/{}", self.vault_id, credential_id);
        let _: serde_json::Value = self
            .client
            .request(
                HttpMethod::Delete,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await?;
        Ok(())
    }
}
