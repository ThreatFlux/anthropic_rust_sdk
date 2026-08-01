//! MCP Tunnels API models (research preview: mcp-tunnels-2026-06-22).

use crate::types::{PaginatedResponse, Pagination};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// An MCP tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tunnel {
    pub id: String,
    #[serde(default)]
    pub archived_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub domain: String,
    #[serde(rename = "type")]
    pub object_type: String,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A tunnel connector token. Treat the token as a credential.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelToken {
    pub id: String,
    pub tunnel_token: String,
    #[serde(rename = "type")]
    pub object_type: String,
}

/// A public CA certificate registered on a tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelCertificate {
    pub id: String,
    #[serde(default)]
    pub archived_at: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub expires_at: Option<String>,
    pub fingerprint: String,
    pub tunnel_id: String,
    #[serde(rename = "type")]
    pub object_type: String,
}

/// Request body for creating a tunnel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TunnelCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

impl TunnelCreateRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }
}

/// Request body for rotating a tunnel token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TunnelRotateTokenRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

impl TunnelRotateTokenRequest {
    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }
}

/// Request body for registering a CA certificate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TunnelCertificateCreateRequest {
    pub ca_certificate_pem: String,
}

impl TunnelCertificateCreateRequest {
    pub fn new(ca_certificate_pem: impl Into<String>) -> Self {
        Self {
            ca_certificate_pem: ca_certificate_pem.into(),
        }
    }
}

/// Parameters for listing tunnels.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TunnelListParams {
    pub pagination: Option<Pagination>,
    pub include_archived: Option<bool>,
}

impl TunnelListParams {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn pagination(mut self, pagination: Pagination) -> Self {
        self.pagination = Some(pagination);
        self
    }

    pub fn include_archived(mut self, include_archived: bool) -> Self {
        self.include_archived = Some(include_archived);
        self
    }
}

pub type TunnelListResponse = PaginatedResponse<Tunnel>;
pub type TunnelCertificateListResponse = PaginatedResponse<TunnelCertificate>;
