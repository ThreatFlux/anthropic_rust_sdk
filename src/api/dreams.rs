//! Dreams API implementation (research preview).

use crate::{
    api::utils::build_query_path,
    client::{beta_headers, Client},
    error::Result,
    models::dream::{Dream, DreamCreateRequest, DreamListParams, DreamListResponse},
    types::{HttpMethod, RequestOptions},
};

fn with_dreaming_beta(options: Option<RequestOptions>) -> Option<RequestOptions> {
    Some(
        options
            .unwrap_or_default()
            .with_beta_feature(beta_headers::DREAMING),
    )
}

/// API client for the Dreams research-preview endpoints.
#[derive(Clone)]
pub struct DreamsApi {
    client: Client,
}

impl DreamsApi {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Create a memory-consolidation Dream.
    pub async fn create(
        &self,
        request: DreamCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Dream> {
        self.client
            .request(
                HttpMethod::Post,
                "/dreams?beta=true",
                Some(serde_json::to_value(request)?),
                with_dreaming_beta(options),
            )
            .await
    }

    /// List Dreams with optional cursor and lifecycle filters.
    pub async fn list(
        &self,
        params: Option<DreamListParams>,
        options: Option<RequestOptions>,
    ) -> Result<DreamListResponse> {
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
        if let Some(value) = params.created_at_gt {
            query.push(("created_at[gt]".to_string(), value));
        }
        if let Some(value) = params.created_at_lt {
            query.push(("created_at[lt]".to_string(), value));
        }
        if let Some(value) = params.include_archived {
            query.push(("include_archived".to_string(), value.to_string()));
        }
        for status in params.statuses {
            query.push((
                "statuses".to_string(),
                serde_json::to_value(status)?
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
            ));
        }

        let path = build_query_path("/dreams?beta=true", query);
        self.client
            .request(HttpMethod::Get, &path, None, with_dreaming_beta(options))
            .await
    }

    /// List Dreams with the SDK's standard pagination only.
    pub async fn list_paginated(
        &self,
        pagination: Option<crate::types::Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<DreamListResponse> {
        self.list(
            Some(DreamListParams {
                pagination,
                ..Default::default()
            }),
            options,
        )
        .await
    }

    pub async fn get(&self, dream_id: &str, options: Option<RequestOptions>) -> Result<Dream> {
        let path = format!("/dreams/{}?beta=true", dream_id);
        self.client
            .request(HttpMethod::Get, &path, None, with_dreaming_beta(options))
            .await
    }

    pub async fn archive(&self, dream_id: &str, options: Option<RequestOptions>) -> Result<Dream> {
        let path = format!("/dreams/{}/archive?beta=true", dream_id);
        self.client
            .request(HttpMethod::Post, &path, None, with_dreaming_beta(options))
            .await
    }

    pub async fn cancel(&self, dream_id: &str, options: Option<RequestOptions>) -> Result<Dream> {
        let path = format!("/dreams/{}/cancel?beta=true", dream_id);
        self.client
            .request(HttpMethod::Post, &path, None, with_dreaming_beta(options))
            .await
    }
}
