//! Usage Admin API implementation

use crate::{
    api::utils::build_path_with_query,
    client::Client,
    error::{AnthropicError, Result},
    models::admin::{
        ApiKeyUsage, ClaudeCodeUsageReportParams, ClaudeCodeUsageReportResponse,
        MessageCostReportParams, MessageCostReportResponse, MessageUsageReportParams,
        MessageUsageReportResponse, UsageQuery, UsageReport, UsageReportListResponse,
    },
    types::{HttpMethod, Pagination, RequestOptions},
};
use chrono::{DateTime, Utc};

/// API client for Usage admin endpoints
#[derive(Clone)]
pub struct UsageApi {
    client: Client,
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::UsageApi;
    use crate::{
        models::admin::{
            ClaudeCodeUsageReportParams, MessageCostReportParams, MessageUsageReportParams,
        },
        Client, Config,
    };
    use chrono::{NaiveDate, TimeZone, Utc};
    use serde_json::json;
    use wiremock::{
        matchers::{method, query_param},
        Mock, MockServer, ResponseTemplate,
    };

    #[test]
    fn test_build_message_usage_report_query() {
        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 2, 0, 0, 0).unwrap();
        let params = MessageUsageReportParams::new(start)
            .ending_at(end)
            .bucket_width("1h")
            .group_by(["model", "workspace_id"])
            .models(["claude-sonnet-4-5"])
            .workspace_ids(["ws_123"])
            .page("page_1")
            .limit(100);

        let query = UsageApi::build_message_usage_report_query(params);
        assert!(query.iter().any(|p| p == "bucket_width=1h"));
        assert!(query.iter().any(|p| p == "group_by[]=model"));
        assert!(query.iter().any(|p| p == "group_by[]=workspace_id"));
        assert!(query.iter().any(|p| p == "models[]=claude-sonnet-4-5"));
        assert!(query.iter().any(|p| p == "workspace_ids[]=ws_123"));
        assert!(query.iter().any(|p| p == "page=page_1"));
        assert!(query.iter().any(|p| p == "limit=100"));
    }

    #[tokio::test]
    async fn test_get_message_usage_report_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("bucket_width", "1h"))
            .and(query_param("page", "page_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [],
                "has_more": false,
                "next_page": null
            })))
            .mount(&server)
            .await;

        let config = Config::new("test-key")
            .unwrap()
            .with_admin_key("admin-key")
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = UsageApi::new(client);

        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let _ = api
            .get_message_usage_report(
                MessageUsageReportParams::new(start)
                    .bucket_width("1h")
                    .page("page_1"),
                None,
            )
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].url.path(),
            "/v1/organizations/usage_report/messages"
        );
    }

    #[tokio::test]
    async fn test_get_message_cost_report_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("invoice_id", "inv_123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [],
                "has_more": false,
                "next_page": null
            })))
            .mount(&server)
            .await;

        let config = Config::new("test-key")
            .unwrap()
            .with_admin_key("admin-key")
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = UsageApi::new(client);

        let start = Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap();
        let _ = api
            .get_message_cost_report(
                MessageCostReportParams::new(start).invoice_id("inv_123"),
                None,
            )
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests[0].url.path(), "/v1/organizations/cost_report");
    }

    #[tokio::test]
    async fn test_get_claude_code_usage_report_endpoint() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(query_param("starting_at", "2026-01-01"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "data": [],
                "has_more": false,
                "next_page": null
            })))
            .mount(&server)
            .await;

        let config = Config::new("test-key")
            .unwrap()
            .with_admin_key("admin-key")
            .with_base_url(server.uri().parse().unwrap());
        let client = Client::new(config);
        let api = UsageApi::new(client);

        let params = ClaudeCodeUsageReportParams::new(
            NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
        )
        .limit(20);

        let _ = api
            .get_claude_code_usage_report(params, None)
            .await
            .unwrap();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].url.path(),
            "/v1/organizations/usage_report/claude_code"
        );
    }
}

impl UsageApi {
    /// Create a new Usage API client
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    fn push_vec_query(query: &mut Vec<String>, key: &str, values: Option<Vec<String>>) {
        if let Some(values) = values {
            query.extend(
                values
                    .into_iter()
                    .map(|value| format!("{}[]={}", key, value)),
            );
        }
    }

    fn build_message_usage_report_query(params: MessageUsageReportParams) -> Vec<String> {
        let mut query = vec![format!("starting_at={}", params.starting_at.to_rfc3339())];

        if let Some(ending_at) = params.ending_at {
            query.push(format!("ending_at={}", ending_at.to_rfc3339()));
        }
        if let Some(bucket_width) = params.bucket_width {
            query.push(format!("bucket_width={}", bucket_width));
        }
        if let Some(page) = params.page {
            query.push(format!("page={}", page));
        }
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }

        Self::push_vec_query(&mut query, "api_key_ids", params.api_key_ids);
        Self::push_vec_query(&mut query, "context_window", params.context_window);
        Self::push_vec_query(&mut query, "group_by", params.group_by);
        Self::push_vec_query(&mut query, "inference_geos", params.inference_geos);
        Self::push_vec_query(&mut query, "models", params.models);
        Self::push_vec_query(&mut query, "service_tiers", params.service_tiers);
        Self::push_vec_query(&mut query, "speeds", params.speeds);
        Self::push_vec_query(&mut query, "workspace_ids", params.workspace_ids);

        query
    }

    fn build_message_cost_report_query(params: MessageCostReportParams) -> Vec<String> {
        let mut query = vec![format!("starting_at={}", params.starting_at.to_rfc3339())];

        if let Some(ending_at) = params.ending_at {
            query.push(format!("ending_at={}", ending_at.to_rfc3339()));
        }
        if let Some(bucket_width) = params.bucket_width {
            query.push(format!("bucket_width={}", bucket_width));
        }
        if let Some(invoice_id) = params.invoice_id {
            query.push(format!("invoice_id={}", invoice_id));
        }
        if let Some(page) = params.page {
            query.push(format!("page={}", page));
        }
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }

        Self::push_vec_query(&mut query, "group_by", params.group_by);
        Self::push_vec_query(&mut query, "service_tiers", params.service_tiers);
        Self::push_vec_query(&mut query, "workspace_ids", params.workspace_ids);

        query
    }

    fn build_claude_code_usage_report_query(params: ClaudeCodeUsageReportParams) -> Vec<String> {
        let mut query = vec![format!(
            "starting_at={}",
            params.starting_at.format("%Y-%m-%d")
        )];

        if let Some(ending_at) = params.ending_at {
            query.push(format!("ending_at={}", ending_at.format("%Y-%m-%d")));
        }
        if let Some(limit) = params.limit {
            query.push(format!("limit={}", limit));
        }
        if let Some(page) = params.page {
            query.push(format!("page={}", page));
        }
        if let Some(organization_id) = params.organization_id {
            query.push(format!("organization_id={}", organization_id));
        }
        if let Some(customer_type) = params.customer_type {
            query.push(format!("customer_type={}", customer_type));
        }
        if let Some(terminal_type) = params.terminal_type {
            query.push(format!("terminal_type={}", terminal_type));
        }

        query
    }

    /// Get messages usage report (current Admin API endpoint).
    pub async fn get_message_usage_report(
        &self,
        params: MessageUsageReportParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageUsageReportResponse> {
        let query = Self::build_message_usage_report_query(params);
        let path = build_path_with_query("/organizations/usage_report/messages", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get messages cost report (current Admin API endpoint).
    pub async fn get_message_cost_report(
        &self,
        params: MessageCostReportParams,
        options: Option<RequestOptions>,
    ) -> Result<MessageCostReportResponse> {
        let query = Self::build_message_cost_report_query(params);
        let path = build_path_with_query("/organizations/cost_report", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    /// Get Claude Code usage report.
    pub async fn get_claude_code_usage_report(
        &self,
        params: ClaudeCodeUsageReportParams,
        options: Option<RequestOptions>,
    ) -> Result<ClaudeCodeUsageReportResponse> {
        let query = Self::build_claude_code_usage_report_query(params);
        let path = build_path_with_query("/organizations/usage_report/claude_code", query);
        self.client
            .request_admin(HttpMethod::Get, &path, None, options)
            .await
    }

    fn legacy_usage_endpoint_error(endpoint: &str) -> AnthropicError {
        AnthropicError::invalid_input(format!(
            "Legacy {} endpoint has been hard-gated. Use get_message_usage_report, get_message_cost_report, or get_claude_code_usage_report instead.",
            endpoint
        ))
    }

    /// Get usage report for the organization.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn get_organization_usage(
        &self,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity: Option<&str>, // "day", "hour"
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let _ = start_date;
        let _ = end_date;
        let _ = granularity;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error("/organizations/usage"))
    }

    /// Get usage report for a specific workspace.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn get_workspace_usage(
        &self,
        workspace_id: &str,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let _ = workspace_id;
        let _ = start_date;
        let _ = end_date;
        let _ = granularity;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/workspaces/{workspace_id}/usage",
        ))
    }

    /// Get usage report for a specific API key.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn get_api_key_usage(
        &self,
        api_key_id: &str,
        workspace_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        granularity: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let _ = api_key_id;
        let _ = workspace_id;
        let _ = start_date;
        let _ = end_date;
        let _ = granularity;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/api_keys/{api_key_id}/usage",
        ))
    }

    /// Get usage history with pagination.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn list_usage_history(
        &self,
        workspace_id: Option<&str>,
        pagination: Option<Pagination>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReportListResponse> {
        let _ = workspace_id;
        let _ = pagination;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/usage/history",
        ))
    }

    /// Query usage with custom filters.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn query_usage(
        &self,
        query: UsageQuery,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let _ = query;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/usage/query",
        ))
    }

    /// Get current billing period usage.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn get_current_billing_usage(
        &self,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let _ = workspace_id;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/usage/current",
        ))
    }

    /// Get usage summary for a date range.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn get_usage_summary(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        workspace_id: Option<&str>,
        options: Option<RequestOptions>,
    ) -> Result<UsageReport> {
        let _ = start_date;
        let _ = end_date;
        let _ = workspace_id;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/usage/summary",
        ))
    }

    /// Get top consuming API keys.
    #[deprecated(
        note = "Legacy /organizations/usage endpoints are hard-gated. Use get_message_usage_report/get_message_cost_report."
    )]
    pub async fn get_top_api_keys(
        &self,
        limit: Option<u32>,
        workspace_id: Option<&str>,
        start_date: Option<DateTime<Utc>>,
        end_date: Option<DateTime<Utc>>,
        options: Option<RequestOptions>,
    ) -> Result<Vec<ApiKeyUsage>> {
        let _ = limit;
        let _ = workspace_id;
        let _ = start_date;
        let _ = end_date;
        let _ = options;
        Err(Self::legacy_usage_endpoint_error(
            "/organizations/usage/top_keys",
        ))
    }
}
