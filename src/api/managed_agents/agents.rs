//! Managed Agents — Agents API implementation (beta: managed-agents-2026-04-01)

use crate::{
    api::managed_agents::with_managed_agents_beta,
    api::utils::build_paginated_path,
    client::Client,
    error::Result,
    models::managed_agents::agent::{
        Agent, AgentCreateRequest, AgentListResponse, AgentUpdateRequest,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for the Managed Agents — Agents endpoints (`/v1/agents`).
#[derive(Clone)]
pub struct AgentsApi {
    client: Client,
}

impl AgentsApi {
    /// Create a new Agents API client.
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create an agent.
    ///
    /// # Example
    /// ```rust,no_run
    /// use threatflux_anthropic_sdk::{Client, models::managed_agents::AgentCreateRequest};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = Client::from_env()?;
    /// let request = AgentCreateRequest::new("triage", "claude-opus-4-8")
    ///     .system("You are a security triage assistant.");
    ///
    /// let agent = client.agents().create(request, None).await?;
    /// println!("Created agent: {} (version {})", agent.id, agent.version);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn create(
        &self,
        request: AgentCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Agent> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/agents",
                Some(body),
                with_managed_agents_beta(options),
            )
            .await
    }

    /// List agents (cursor-style pagination).
    pub async fn list(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<AgentListResponse> {
        let path = build_paginated_path("/agents", pagination.as_ref());
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve an agent by id (latest version).
    pub async fn get(&self, agent_id: &str, options: Option<RequestOptions>) -> Result<Agent> {
        let path = format!("/agents/{}", agent_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Retrieve a specific version of an agent.
    pub async fn get_version(
        &self,
        agent_id: &str,
        version: &str,
        options: Option<RequestOptions>,
    ) -> Result<Agent> {
        let path = format!("/agents/{}/versions/{}", agent_id, version);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }

    /// Update an agent. Each update mints a new version.
    pub async fn update(
        &self,
        agent_id: &str,
        request: AgentUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Agent> {
        let path = format!("/agents/{}", agent_id);
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

    /// Archive an agent.
    pub async fn archive(&self, agent_id: &str, options: Option<RequestOptions>) -> Result<Agent> {
        let path = format!("/agents/{}/archive", agent_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                None,
                with_managed_agents_beta(options),
            )
            .await
    }
}
