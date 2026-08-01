//! MCP Tunnels API implementation (research preview).

use crate::{
    api::utils::build_query_path,
    client::{beta_headers, Client},
    error::Result,
    models::tunnel::{
        Tunnel, TunnelCertificate, TunnelCertificateCreateRequest, TunnelCertificateListResponse,
        TunnelCreateRequest, TunnelListParams, TunnelListResponse, TunnelRotateTokenRequest,
        TunnelToken,
    },
    types::{HttpMethod, RequestOptions},
};

fn with_tunnels_beta(options: Option<RequestOptions>) -> Option<RequestOptions> {
    Some(
        options
            .unwrap_or_default()
            .with_beta_feature(beta_headers::MCP_TUNNELS),
    )
}

/// API client for MCP tunnel management.
#[derive(Clone)]
pub struct TunnelsApi {
    client: Client,
}

impl TunnelsApi {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn create(
        &self,
        request: TunnelCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<Tunnel> {
        self.client
            .request(
                HttpMethod::Post,
                "/tunnels?beta=true",
                Some(serde_json::to_value(request)?),
                with_tunnels_beta(options),
            )
            .await
    }

    pub async fn get(&self, tunnel_id: &str, options: Option<RequestOptions>) -> Result<Tunnel> {
        let path = format!("/tunnels/{}?beta=true", tunnel_id);
        self.client
            .request(HttpMethod::Get, &path, None, with_tunnels_beta(options))
            .await
    }

    pub async fn list(
        &self,
        params: Option<TunnelListParams>,
        options: Option<RequestOptions>,
    ) -> Result<TunnelListResponse> {
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
        if let Some(include_archived) = params.include_archived {
            query.push(("include_archived".to_string(), include_archived.to_string()));
        }
        let path = build_query_path("/tunnels?beta=true", query);
        self.client
            .request(HttpMethod::Get, &path, None, with_tunnels_beta(options))
            .await
    }

    pub async fn archive(
        &self,
        tunnel_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<Tunnel> {
        let path = format!("/tunnels/{}/archive?beta=true", tunnel_id);
        self.client
            .request(HttpMethod::Post, &path, None, with_tunnels_beta(options))
            .await
    }

    pub async fn reveal_token(
        &self,
        tunnel_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<TunnelToken> {
        let path = format!("/tunnels/{}/reveal_token?beta=true", tunnel_id);
        self.client
            .request(HttpMethod::Post, &path, None, with_tunnels_beta(options))
            .await
    }

    pub async fn rotate_token(
        &self,
        tunnel_id: &str,
        request: TunnelRotateTokenRequest,
        options: Option<RequestOptions>,
    ) -> Result<TunnelToken> {
        let path = format!("/tunnels/{}/rotate_token?beta=true", tunnel_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(serde_json::to_value(request)?),
                with_tunnels_beta(options),
            )
            .await
    }

    /// Access certificates for a tunnel.
    pub fn certificates(&self, tunnel_id: impl Into<String>) -> TunnelCertificatesApi {
        TunnelCertificatesApi {
            client: self.client.clone(),
            tunnel_id: tunnel_id.into(),
        }
    }
}

/// API client for a tunnel's CA certificates.
#[derive(Clone)]
pub struct TunnelCertificatesApi {
    client: Client,
    tunnel_id: String,
}

impl TunnelCertificatesApi {
    pub async fn create(
        &self,
        request: TunnelCertificateCreateRequest,
        options: Option<RequestOptions>,
    ) -> Result<TunnelCertificate> {
        let path = format!("/tunnels/{}/certificates?beta=true", self.tunnel_id);
        self.client
            .request(
                HttpMethod::Post,
                &path,
                Some(serde_json::to_value(request)?),
                with_tunnels_beta(options),
            )
            .await
    }

    pub async fn get(
        &self,
        certificate_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<TunnelCertificate> {
        let path = format!(
            "/tunnels/{}/certificates/{}?beta=true",
            self.tunnel_id, certificate_id
        );
        self.client
            .request(HttpMethod::Get, &path, None, with_tunnels_beta(options))
            .await
    }

    pub async fn list(
        &self,
        pagination: Option<crate::types::Pagination>,
        include_archived: Option<bool>,
        options: Option<RequestOptions>,
    ) -> Result<TunnelCertificateListResponse> {
        let mut query = Vec::new();
        if let Some(pagination) = pagination {
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
        if let Some(include_archived) = include_archived {
            query.push(("include_archived".to_string(), include_archived.to_string()));
        }
        let path = build_query_path(
            &format!("/tunnels/{}/certificates?beta=true", self.tunnel_id),
            query,
        );
        self.client
            .request(HttpMethod::Get, &path, None, with_tunnels_beta(options))
            .await
    }

    pub async fn archive(
        &self,
        certificate_id: &str,
        options: Option<RequestOptions>,
    ) -> Result<TunnelCertificate> {
        let path = format!(
            "/tunnels/{}/certificates/{}/archive?beta=true",
            self.tunnel_id, certificate_id
        );
        self.client
            .request(HttpMethod::Post, &path, None, with_tunnels_beta(options))
            .await
    }
}
