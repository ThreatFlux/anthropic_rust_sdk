//! Managed Agents — Memory Stores & Memories API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::memory::{
        Memory, MemoryCreateRequest, MemoryListResponse, MemoryRedactRequest, MemoryStore,
        MemoryStoreCreateRequest, MemoryStoreListResponse, MemoryStoreUpdateRequest,
        MemoryUpdateRequest, MemoryVersion, MemoryVersionListResponse,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Memory Stores endpoints
/// (`/v1/memory_stores`).
#[derive(Clone)]
pub struct MemoryStoresApi {
    client: Client,
}

impl MemoryStoresApi {
    /// Create a new Memory Stores API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a memory store.
    pub async fn create(
        &self,
        request: MemoryStoreCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<MemoryStore> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/memory_stores",
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List memory stores (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<MemoryStoreListResponse> {
        let path = build_paginated_path("/memory_stores", pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve a memory store by id.
    pub async fn get(
        &self,
        store_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<MemoryStore> {
        let path = format!("/memory_stores/{}", store_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a memory store.
    pub async fn update(
        &self,
        store_id: &str,
        request: MemoryStoreUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<MemoryStore> {
        let path = format!("/memory_stores/{}", store_id);
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

    /// Delete a memory store.
    pub async fn delete(&self, store_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/memory_stores/{}", store_id);
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

    /// Access the memories sub-resource for a store.
    pub fn memories(&self, store_id: &str) -> MemoriesApi {
        MemoriesApi::new(self.client.clone(), store_id.to_string())
    }
}

/// API client for the Managed Agents — Memories endpoints
/// (`/v1/memory_stores/{store_id}/memories`).
#[derive(Clone)]
pub struct MemoriesApi {
    client: Client,
    store_id: String,
}

impl MemoriesApi {
    /// Create a new Memories API client scoped to a memory store.
    pub fn new(client: Client, store_id: String) -> Self {
        Self { client, store_id }
    }

    /// Create a memory entry.
    pub async fn create(
        &self,
        request: MemoryCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Memory> {
        let path = format!("/memory_stores/{}/memories", self.store_id);
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

    /// List memory entries (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<MemoryListResponse> {
        let base = format!("/memory_stores/{}/memories", self.store_id);
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

    /// Retrieve a memory entry by id.
    pub async fn get(&self, memory_id: &str, options: Option<RequestOptions>) -> Result<Memory> {
        let path = format!("/memory_stores/{}/memories/{}", self.store_id, memory_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update a memory entry.
    pub async fn update(
        &self,
        memory_id: &str,
        request: MemoryUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Memory> {
        let path = format!("/memory_stores/{}/memories/{}", self.store_id, memory_id);
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

    /// Delete a memory entry.
    pub async fn delete(&self, memory_id: &str, options: Option<RequestOptions>) -> Result<()> {
        let path = format!("/memory_stores/{}/memories/{}", self.store_id, memory_id);
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

    /// Redact a memory entry.
    pub async fn redact(
        &self,
        memory_id: &str,
        request: MemoryRedactRequest,
        options: Option<RequestOptions>,
    ) -> Result<Memory> {
        let path = format!(
            "/memory_stores/{}/memories/{}/redact",
            self.store_id, memory_id
        );
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

    /// List versions of a memory entry (cursor-style pagination).
    pub async fn list_versions(
        &self,
        memory_id: &str,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<MemoryVersionListResponse> {
        let base = format!(
            "/memory_stores/{}/memories/{}/versions",
            self.store_id, memory_id
        );
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

    /// Retrieve a specific version of a memory entry.
    pub async fn get_version(
        &self,
        memory_id: &str,
        version_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<MemoryVersion> {
        let path = format!(
            "/memory_stores/{}/memories/{}/versions/{}",
            self.store_id, memory_id, version_id
        );
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }
}
