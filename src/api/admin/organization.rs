//! Organization Admin API implementation

use crate::{
    api::utils::{build_paginated_path, create_default_pagination},
    client::Client,
    error::Result,
    models::admin::{
        Member, MemberCreateRequest, MemberListResponse, MemberUpdateRequest, Organization,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};

/// API client for Organization admin endpoints
#[derive(Clone)]
pub struct OrganizationApi {
    client: Client,
}

impl OrganizationApi {
    /// Create a new Organization API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get organization information
    pub async fn get(&self, options: Option<RequestOptions>) -> Result<Organization> {
        self.client
            .request(HttpMethod::Get, "/organization", None, options)
            .await
    }

    /// List organization members
    pub async fn list_members(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<MemberListResponse> {
        let path = build_paginated_path("/organization/members", pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific member
    pub async fn get_member(
        &self,
        member_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Member> {
        let path = format!("/organization/members/{}", member_id);
        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Add a member to the organization
    pub async fn add_member(
        &self,
        request: MemberCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Member> {
        let body = serde_json::to_value(request)?;
        self.client
            .request(
                HttpMethod::Post,
                "/organization/members",
                Some(body),
                options,
            )
            .await
    }

    /// Update a member
    pub async fn update_member(
        &self,
        member_id: &str,
        request: MemberUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Member> {
        let path = format!("/organization/members/{}", member_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request(HttpMethod::Patch, &path, Some(body), options)
            .await
    }

    /// Remove a member from the organization
    pub async fn remove_member(
        &self,
        member_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<()> {
        let path = format!("/organization/members/{}", member_id);
        let _: serde_json::Value = self
            .client
            .request(HttpMethod::Delete, &path, None, options)
            .await?;
        Ok(())
    }

    /// List all members (convenience method)
    pub async fn list_all_members(&self, options: Option<RequestOptions>) -> Result<Vec<Member>> {
        let mut all_members = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self.list_members(Some(pagination), options.clone()).await?;

            all_members.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_members)
    }
}
