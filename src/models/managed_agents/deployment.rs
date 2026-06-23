//! Managed Agents — Deployment data models (beta: managed-agents-2026-04-01)

use crate::models::managed_agents::session::{SessionAgentRef, SessionResourceSpec};
use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A schedule (cron expression) governing automatic deployment runs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentSchedule {
    /// Cron expression.
    pub cron: String,
    /// Optional timezone for the cron expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl DeploymentSchedule {
    /// Create a new schedule from a cron expression.
    pub fn cron(expr: impl Into<String>) -> Self {
        Self {
            cron: expr.into(),
            timezone: None,
            extra: HashMap::new(),
        }
    }

    /// Set the timezone.
    pub fn timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }
}

/// A deployment that schedules sessions for an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deployment {
    /// Object type (always `"deployment"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique deployment identifier.
    pub id: String,
    /// Human-friendly name.
    pub name: String,
    /// The agent to run.
    pub agent: SessionAgentRef,
    /// Environment to execute in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    /// Optional schedule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<DeploymentSchedule>,
    /// Resources to mount on each run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<SessionResourceSpec>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A single execution of a deployment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentRun {
    /// Object type (always `"deployment_run"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique run identifier.
    pub id: String,
    /// Parent deployment identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_id: Option<String>,
    /// Session created for this run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Run status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Request body for creating a deployment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeploymentCreateRequest {
    /// Human-friendly name.
    pub name: String,
    /// The agent to run.
    pub agent: SessionAgentRef,
    /// Environment to execute in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment_id: Option<String>,
    /// Optional schedule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<DeploymentSchedule>,
    /// Resources to mount on each run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resources: Vec<SessionResourceSpec>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl DeploymentCreateRequest {
    /// Create a new deployment create request.
    pub fn new(name: impl Into<String>, agent: impl Into<SessionAgentRef>) -> Self {
        Self {
            name: name.into(),
            agent: agent.into(),
            environment_id: None,
            schedule: None,
            resources: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the environment.
    pub fn environment(mut self, environment_id: impl Into<String>) -> Self {
        self.environment_id = Some(environment_id.into());
        self
    }

    /// Set the schedule.
    pub fn schedule(mut self, schedule: DeploymentSchedule) -> Self {
        self.schedule = Some(schedule);
        self
    }

    /// Add a resource.
    pub fn add_resource(mut self, resource: SessionResourceSpec) -> Self {
        self.resources.push(resource);
        self
    }
}

/// Request body for updating a deployment.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DeploymentUpdateRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New schedule.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schedule: Option<DeploymentSchedule>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl DeploymentUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the schedule.
    pub fn schedule(mut self, schedule: DeploymentSchedule) -> Self {
        self.schedule = Some(schedule);
        self
    }
}

/// Response when listing deployments (cursor-style pagination).
pub type DeploymentListResponse = PaginatedResponse<Deployment>;

/// Response when listing deployment runs (cursor-style pagination).
pub type DeploymentRunListResponse = PaginatedResponse<DeploymentRun>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_create_request_builder() {
        let req = DeploymentCreateRequest::new("nightly", "agent_1")
            .environment("env_1")
            .schedule(DeploymentSchedule::cron("0 0 * * *").timezone("UTC"));
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["name"], "nightly");
        assert_eq!(value["environment_id"], "env_1");
        assert_eq!(value["schedule"]["cron"], "0 0 * * *");
        assert_eq!(value["agent"], "agent_1");
    }
}
