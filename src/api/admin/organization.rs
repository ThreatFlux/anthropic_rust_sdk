//! Organization Admin API implementation

use crate::{
    api::utils::{build_path_with_query, create_default_pagination},
    client::Client,
    error::{AnthropicError, Result},
    models::admin::{
        Invite, InviteCreateRequest, InviteCreateRole, InviteDeleteResponse, InviteListParams,
        InviteListResponse, InviteStatus, Member, MemberCreateRequest, MemberListResponse,
        MemberRole, MemberStatus, MemberUpdateRequest, Organization, User, UserDeleteResponse,
        UserListParams, UserListResponse, UserRole, UserUpdateRequest, UserUpdateRole,
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
            .request_admin(HttpMethod::Get, "/organizations/me", None, options)
            .await
    }

    /// List organization users.
    pub async fn list_users(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<UserListResponse> {
        let mut params = UserListParams::new();
        if let Some(pagination) = pagination {
            if let Some(limit) = pagination.limit {
                params = params.with_limit(limit);
            }
            if let Some(after) = pagination.after {
                params = params.with_after_id(after);
            }
            if let Some(before) = pagination.before {
                params = params.with_before_id(before);
            }
        }
        self.list_users_with_params(params, options).await
    }

    /// List organization users with full query controls.
    pub async fn list_users_with_params(
        &self,
        params: UserListParams,
        options: Option<RequestOptions>,
    ) -> Result<UserListResponse> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }
        if let Some(after_id) = params.after_id {
            query.push(format!("after_id={}", after_id));
        }
        if let Some(before_id) = params.before_id {
            query.push(format!("before_id={}", before_id));
        }
        if let Some(email) = params.email {
            query.push(format!("email={}", email));
        }

        let path = build_path_with_query("/organizations/users", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific organization user.
    pub async fn get_user(&self, user_id: &str, options: Option<RequestOptions>) -> Result<User> {
        let path = format!("/organizations/users/{}", user_id);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Update an organization user.
    pub async fn update_user(
        &self,
        user_id: &str,
        request: UserUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<User> {
        let path = format!("/organizations/users/{}", user_id);
        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(HttpMethod::Post, &path, Some(body), options)
            .await
    }

    /// Delete an organization user.
    pub async fn delete_user(
        &self,
        user_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<UserDeleteResponse> {
        let path = format!("/organizations/users/{}", user_id);
        self.client
            .request_admin(HttpMethod::Delete, &path, None, options)
            .await
    }

    /// List organization invites.
    pub async fn list_invites(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<InviteListResponse> {
        let mut params = InviteListParams::new();
        if let Some(pagination) = pagination {
            if let Some(limit) = pagination.limit {
                params = params.with_limit(limit);
            }
            if let Some(after) = pagination.after {
                params = params.with_after_id(after);
            }
            if let Some(before) = pagination.before {
                params = params.with_before_id(before);
            }
        }
        self.list_invites_with_params(params, options).await
    }

    /// List organization invites with full query controls.
    pub async fn list_invites_with_params(
        &self,
        params: InviteListParams,
        options: Option<RequestOptions>,
    ) -> Result<InviteListResponse> {
        let mut query = Vec::new();
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }
        if let Some(after_id) = params.after_id {
            query.push(format!("after_id={}", after_id));
        }
        if let Some(before_id) = params.before_id {
            query.push(format!("before_id={}", before_id));
        }

        let path = build_path_with_query("/organizations/invites", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get a specific invite.
    pub async fn get_invite(
        &self,
        invite_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Invite> {
        let path = format!("/organizations/invites/{}", invite_id);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Create an organization invite.
    pub async fn create_invite(
        &self,
        request: InviteCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Invite> {
        let body = serde_json::to_value(request)?;
        self.client
            .request_admin(
                HttpMethod::Post,
                "/organizations/invites",
                Some(body),
                options,
            )
            .await
    }

    /// Delete an organization invite.
    pub async fn delete_invite(
        &self,
        invite_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<InviteDeleteResponse> {
        let path = format!("/organizations/invites/{}", invite_id);
        self.client
            .request_admin(HttpMethod::Delete, &path, None, options)
            .await
    }

    /// List all users (convenience method).
    pub async fn list_all_users(&self, options: Option<RequestOptions>) -> Result<Vec<User>> {
        let mut all_users = Vec::new();
        let mut after = None;

        loop {
            let pagination = create_default_pagination(after);
            let response = self.list_users(Some(pagination), options.clone()).await?;

            all_users.extend(response.data);

            if !response.has_more {
                break;
            }

            after = response.last_id;
        }

        Ok(all_users)
    }

    /// List organization members (legacy compatibility wrapper).
    #[deprecated(note = "Use list_users/list_users_with_params for full Admin API parity")]
    pub async fn list_members(
        &self,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<MemberListResponse> {
        let response = self.list_users(pagination, options).await?;
        Ok(MemberListResponse {
            data: response
                .data
                .into_iter()
                .map(Self::user_to_member)
                .collect(),
            has_more: response.has_more,
            first_id: response.first_id,
            last_id: response.last_id,
        })
    }

    /// Get a specific member (legacy compatibility wrapper).
    #[deprecated(note = "Use get_user for full Admin API parity")]
    pub async fn get_member(
        &self,
        member_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Member> {
        let user = self.get_user(member_id, options).await?;
        Ok(Self::user_to_member(user))
    }

    /// Create an organization invite and return a compatibility `Member` projection.
    ///
    /// Prefer `create_invite` for full invite details.
    #[deprecated(note = "Use create_invite for full Admin API parity")]
    pub async fn add_member(
        &self,
        request: MemberCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Member> {
        let MemberCreateRequest { email, role, name } = request;
        let invite_role = Self::map_member_role_to_invite_create_role(role)?;
        let invite = self
            .create_invite(InviteCreateRequest::new(email, invite_role), options)
            .await?;

        let mut member = Self::invite_to_member(invite);
        member.name = name;
        Ok(member)
    }

    /// Update a member (legacy compatibility wrapper).
    #[deprecated(note = "Use update_user for full Admin API parity")]
    pub async fn update_member(
        &self,
        member_id: &str,
        request: MemberUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Member> {
        if request.name.is_some() || request.status.is_some() {
            return Err(AnthropicError::invalid_input(
                "Legacy member name/status updates are not supported by current /organizations/users endpoint",
            ));
        }

        let role = request.role.ok_or_else(|| {
            AnthropicError::invalid_input("role is required when updating a member")
        })?;
        let update_role = Self::map_member_role_to_user_update_role(role)?;
        let user = self
            .update_user(member_id, UserUpdateRequest::new(update_role), options)
            .await?;
        Ok(Self::user_to_member(user))
    }

    /// Remove a member from the organization (legacy compatibility wrapper).
    #[deprecated(note = "Use delete_user for full Admin API parity")]
    pub async fn remove_member(
        &self,
        member_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<()> {
        let _ = self.delete_user(member_id, options).await?;
        Ok(())
    }

    /// List all members (legacy compatibility wrapper).
    #[deprecated(note = "Use list_all_users for full Admin API parity")]
    pub async fn list_all_members(&self, options: Option<RequestOptions>) -> Result<Vec<Member>> {
        let users = self.list_all_users(options).await?;
        Ok(users.into_iter().map(Self::user_to_member).collect())
    }

    fn user_to_member(user: User) -> Member {
        Member {
            id: user.id,
            email: user.email,
            name: user.name,
            role: Self::map_user_role_to_member_role(user.role),
            status: MemberStatus::Active,
            invited_at: Some(user.added_at),
            joined_at: Some(user.added_at),
            last_active_at: None,
        }
    }

    fn invite_to_member(invite: Invite) -> Member {
        let status = match invite.status {
            InviteStatus::Accepted => MemberStatus::Active,
            InviteStatus::Pending => MemberStatus::Pending,
            InviteStatus::Expired | InviteStatus::Deleted => MemberStatus::Inactive,
        };

        Member {
            id: invite.id,
            email: invite.email,
            name: None,
            role: Self::map_user_role_to_member_role(invite.role),
            status,
            invited_at: Some(invite.invited_at),
            joined_at: None,
            last_active_at: None,
        }
    }

    fn map_user_role_to_member_role(role: UserRole) -> MemberRole {
        match role {
            UserRole::User => MemberRole::Member,
            UserRole::Developer => MemberRole::Developer,
            UserRole::Billing => MemberRole::Billing,
            UserRole::Admin => MemberRole::Admin,
            UserRole::ClaudeCodeUser => MemberRole::ClaudeCodeUser,
            UserRole::Managed => MemberRole::Member,
        }
    }

    fn map_member_role_to_invite_create_role(role: MemberRole) -> Result<InviteCreateRole> {
        match role {
            MemberRole::Member | MemberRole::Viewer => Ok(InviteCreateRole::User),
            MemberRole::Developer => Ok(InviteCreateRole::Developer),
            MemberRole::Billing => Ok(InviteCreateRole::Billing),
            MemberRole::ClaudeCodeUser => Ok(InviteCreateRole::ClaudeCodeUser),
            MemberRole::Owner | MemberRole::Admin => Err(AnthropicError::invalid_input(
                "Invites endpoint does not accept owner/admin role; use user management in Console",
            )),
        }
    }

    fn map_member_role_to_user_update_role(role: MemberRole) -> Result<UserUpdateRole> {
        match role {
            MemberRole::Member | MemberRole::Viewer => Ok(UserUpdateRole::User),
            MemberRole::Developer => Ok(UserUpdateRole::Developer),
            MemberRole::Billing => Ok(UserUpdateRole::Billing),
            MemberRole::ClaudeCodeUser => Ok(UserUpdateRole::ClaudeCodeUser),
            MemberRole::Owner | MemberRole::Admin => Err(AnthropicError::invalid_input(
                "Users update endpoint does not accept owner/admin role",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::OrganizationApi;
    use crate::{Client, Config};
    use serde_json::json;
    use wiremock::{
        matchers::{header, method, path, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_request_admin_uses_x_api_key_without_authorization_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/me"))
            .and(header("x-api-key", "admin-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "org_123",
                "name": "Example Org",
                "created_at": "2026-01-01T00:00:00Z",
                "updated_at": "2026-01-01T00:00:00Z"
            })))
            .mount(&server)
            .await;

        let config = Config::new("api-key")
            .unwrap()
            .with_admin_key("admin-key")
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = OrganizationApi::new(client);

        let _ = api.get(None).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let req = &requests[0];
        assert_eq!(
            req.headers.get("x-api-key").unwrap().to_str().unwrap(),
            "admin-key"
        );
        assert!(!req.headers.contains_key("authorization"));
    }

    #[tokio::test]
    async fn test_list_users_uses_after_id_before_id_query_names() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/users"))
            .and(query_param("after_id", "cursor_after"))
            .and(query_param("before_id", "cursor_before"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [],
                "has_more": false
            })))
            .mount(&server)
            .await;

        let config = Config::new("api-key")
            .unwrap()
            .with_admin_key("admin-key")
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = OrganizationApi::new(client);

        let pagination = crate::types::Pagination::new()
            .with_limit(5)
            .with_after("cursor_after")
            .with_before("cursor_before");
        let _ = api.list_users(Some(pagination), None).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let query_pairs: Vec<&str> = requests[0]
            .url
            .query()
            .unwrap_or_default()
            .split('&')
            .collect();
        assert!(query_pairs.contains(&"after_id=cursor_after"));
        assert!(query_pairs.contains(&"before_id=cursor_before"));
        assert!(!query_pairs.contains(&"after=cursor_after"));
        assert!(!query_pairs.contains(&"before=cursor_before"));
    }

    #[tokio::test]
    async fn test_list_invites_uses_after_id_before_id_query_names() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/organizations/invites"))
            .and(query_param("after_id", "inv_after"))
            .and(query_param("before_id", "inv_before"))
            .and(query_param("limit", "3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [],
                "has_more": false
            })))
            .mount(&server)
            .await;

        let config = Config::new("api-key")
            .unwrap()
            .with_admin_key("admin-key")
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = OrganizationApi::new(client);

        let pagination = crate::types::Pagination::new()
            .with_limit(3)
            .with_after("inv_after")
            .with_before("inv_before");
        let _ = api.list_invites(Some(pagination), None).await.unwrap();

        let requests = server.received_requests().await.unwrap();
        let query_pairs: Vec<&str> = requests[0]
            .url
            .query()
            .unwrap_or_default()
            .split('&')
            .collect();
        assert!(query_pairs.contains(&"after_id=inv_after"));
        assert!(query_pairs.contains(&"before_id=inv_before"));
        assert!(!query_pairs.contains(&"after=inv_after"));
        assert!(!query_pairs.contains(&"before=inv_before"));
    }
}
