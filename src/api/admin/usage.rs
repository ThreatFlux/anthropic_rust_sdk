//! Usage Admin API implementation

use crate::{
    api::utils::{build_paginated_path, build_path_with_query},
    client::Client,
    error::Result,
    models::admin::{UsageQuery, UsageReport, UsageReportListResponse},
    types::{HttpMethod, Pagination, RequestOptions},
};
use chrono::{DateTime, Utc};

/// API client for Usage admin endpoints
#[derive(Clone)]
pub struct UsageApi {
    client: Client,
}

impl UsageApi {
    /// Create a new Usage API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Get usage report for the organization
    pub async fn get_organization_usage(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity: Option<&str>, // "day", "hour"
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let mut query_params = Vec::new();

        if let Some(start) = start_date {
            query_params.push(format!("start_date={}", start.to_rfc3339()));
        }

        if let Some(end) = end_date {
            query_params.push(format!("end_date={}", end.to_rfc3339()));
        }

        if let Some(gran) = granularity {
            query_params.push(format!("granularity={}", gran));
        }

        let path = build_path_with_query("/organization/usage", query_params);

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get usage report for a specific workspace
    pub async fn get_workspace_usage(
        &self,
        workspace_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let mut path = format!("/organization/workspaces/{}/usage", workspace_id);
        let mut query_params = Vec::new();

        if let Some(start) = start_date {
            query_params.push(format!("start_date={}", start.to_rfc3339()));
        }

        if let Some(end) = end_date {
            query_params.push(format!("end_date={}", end.to_rfc3339()));
        }

        if let Some(gran) = granularity {
            query_params.push(format!("granularity={}", gran));
        }

        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get usage report for a specific API key
    pub async fn get_api_key_usage(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let base_path = if let Some(workspace_id) = workspace_id {
            format!(
                "/organization/workspaces/{}/api_keys/{}/usage",
                workspace_id, api_key_id
            )
        } else {
            format!("/organization/api_keys/{}/usage", api_key_id)
        };

        let mut path = base_path;
        let mut query_params = Vec::new();

        if let Some(start) = start_date {
            query_params.push(format!("start_date={}", start.to_rfc3339()));
        }

        if let Some(end) = end_date {
            query_params.push(format!("end_date={}", end.to_rfc3339()));
        }

        if let Some(gran) = granularity {
            query_params.push(format!("granularity={}", gran));
        }

        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get usage history with pagination
    pub async fn list_usage_history(
        &self,
        workspace_id: Option<&str>,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReportListResponse> {
        let base_path = if let Some(workspace_id) = workspace_id {
            format!("/organization/workspaces/{}/usage/history", workspace_id)
        } else {
            "/organization/usage/history".to_string()
        };

        let path = build_paginated_path(&base_path, pagination.as_ref());

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Query usage with custom filters
    pub async fn query_usage(
        &self,
        query: UsageQuery,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let body = serde_json::to_value(query)?;
        self.client
            .request(
                HttpMethod::Post,
                "/organization/usage/query",
                Some(body),
                options,
            )
            .await
    }

    /// Get current billing period usage
    pub async fn get_current_billing_usage(
        &self,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let path = if let Some(workspace_id) = workspace_id {
            format!("/organization/workspaces/{}/usage/current", workspace_id)
        } else {
            "/organization/usage/current".to_string()
        };

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get usage summary for a date range
    pub async fn get_usage_summary(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let base_path = if let Some(workspace_id) = workspace_id {
            format!("/organization/workspaces/{}/usage/summary", workspace_id)
        } else {
            "/organization/usage/summary".to_string()
        };

        let path = format!(
            "{}?start_date={}&end_date={}",
            base_path,
            start_date.to_rfc3339(),
            end_date.to_rfc3339()
        );

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get top consuming API keys
    pub async fn get_top_api_keys(
        &self,
        limit: Option<u32>,
        workspace_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<crate::models::admin::ApiKeyUsage>> {
        let base_path = if let Some(workspace_id) = workspace_id {
            format!("/organization/workspaces/{}/usage/top_keys", workspace_id)
        } else {
            "/organization/usage/top_keys".to_string()
        };

        let mut path = base_path;
        let mut query_params = Vec::new();

        if let Some(limit) = limit {
            query_params.push(format!("limit={}", limit));
        }

        if let Some(start) = start_date {
            query_params.push(format!("start_date={}", start.to_rfc3339()));
        }

        if let Some(end) = end_date {
            query_params.push(format!("end_date={}", end.to_rfc3339()));
        }

        if !query_params.is_empty() {
            path.push('?');
            path.push_str(&query_params.join("&"));
        }

        self.client
            .request(HttpMethod::Get, &path, None, options)
            .await
    }
}
