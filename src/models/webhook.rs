//! Webhook payload models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A webhook event envelope emitted by Managed Agents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    pub created_at: String,
    pub data: WebhookEventData,
    #[serde(rename = "type")]
    pub object_type: String,
}

/// Forward-compatible event data. The event `type` identifies the lifecycle
/// transition; optional ids cover session-thread and vault-credential events.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookEventData {
    pub id: String,
    pub organization_id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub workspace_id: String,
    #[serde(default)]
    pub session_thread_id: Option<String>,
    #[serde(default)]
    pub vault_id: Option<String>,
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl WebhookEvent {
    /// Parse a webhook JSON body. Signature verification is intentionally kept
    /// separate so applications can use their preferred webhook-signature crate.
    pub fn parse(body: &str) -> serde_json::Result<Self> {
        serde_json::from_str(body)
    }
}
