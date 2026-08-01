//! User Profiles API implementation (beta).

use crate::{
    api::utils::build_query_path,
    client::{beta_headers, Client},
    error::Result,
    models::user_profile::{
        EnrollmentUrl, UserProfile, UserProfileCreateRequest, UserProfileListParams,
        UserProfileListResponse, UserProfileUpdateRequest,
    },
    types::{HttpMethod, RequestOptions},
};

fn with_user_profiles_beta(options: Option<RequestOptions>) -> Option<RequestOptions> {
    Some(
        options
            .unwrap_or_default()
            .with_beta_feature(beta_headers::USER_PROFILES),
    )
}

/// API client for user profile attribution and enrollment.
#[derive(Clone)]
pub struct UserProfilesApi {
    client: Client,
}

impl UserProfilesApi {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn create(
        &self,
        request: UserProfileCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<UserProfile> {
        self.client
            .request(
                HttpMethod::Post,
                "/user_profiles?beta=true",
                Some(serde_json::to_value(request)?),
                with_user_profiles_beta(options),
            )
            .await
    }

    pub async fn get(
        &self,
        profile_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<UserProfile> {
        let path = format!("/user_profiles/{}?beta=true", profile_id);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_user_profiles_beta(options),
            )
            .await
    }

    pub async fn update(
        &self,
        profile_id: &str,
        request: UserProfileUpdateRequest,
        options: Option<RequestOptions>,
    ) -> Result<UserProfile> {
        let path = format!("/user_profiles/{}?beta=true", profile_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(serde_json::to_value(request)?),
                with_user_profiles_beta(options),
            )
            .await
    }

    pub async fn list(
        &self,
        params: Option<UserProfileListParams>,
        options: Option<RequestOptions>,
    ) -> Result<UserProfileListResponse> {
        let params = params.unwrap_or_default();
        let mut query = Vec::new();
        if let Some(pagination) = params.pagination {
            if let Some(limit) = pagination.limit {
                query.push(("limit".to_string(), limit.to_string()));
            }
            if let Some(after) = pagination.after {
                query.push(("after".to_string(), after));
            }
            if let Some(before) = pagination.before {
                query.push(("before".to_string(), before));
            }
        }
        if let Some(order) = params.order {
            query.push(("order".to_string(), order));
        }
        let path = build_query_path("/user_profiles?beta=true", query);
        self.client
            .request(
                HttpMethod::Get,
                &path,
                None,
                with_user_profiles_beta(options),
            )
            .await
    }

    pub async fn create_enrollment_url(
        &self,
        profile_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<EnrollmentUrl> {
        let path = format!("/user_profiles/{}/enrollment_url?beta=true", profile_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                None,
                with_user_profiles_beta(options),
            )
            .await
    }
}
