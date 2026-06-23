//! Managed Agents — Agent data models (beta: managed-agents-2026-04-01)

use crate::types::PaginatedResponse;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Model spec for an agent.
///
/// The wire format is either a bare model id string (`"claude-..."`) or an
/// object `{ "id": "...", "speed": "..." }`. Modeled as an untagged enum so
/// both shapes deserialize.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AgentModel {
    /// Bare model id string.
    Id(String),
    /// Model spec object with optional speed hint.
    Spec {
        /// Model identifier.
        id: String,
        /// Optional speed hint (e.g. `"fast"`).
        #[serde(skip_serializing_if = "Option::is_none")]
        speed: Option<String>,
    },
}

impl AgentModel {
    /// Returns the model id regardless of representation.
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
            Self::Spec { id, .. } => id.as_str(),
        }
    }
}

impl From<String> for AgentModel {
    fn from(value: String) -> Self {
        Self::Id(value)
    }
}

impl From<&str> for AgentModel {
    fn from(value: &str) -> Self {
        Self::Id(value.to_string())
    }
}

/// A tool attachable to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTool {
    /// Built-in agent toolset (versioned by date).
    #[serde(rename = "agent_toolset_20260401")]
    AgentToolset {
        /// Built-in toolset flags / configuration.
        #[serde(flatten, default)]
        extra: HashMap<String, serde_json::Value>,
    },
    /// A toolset exposed by an MCP server.
    McpToolset {
        /// MCP server name this toolset is drawn from.
        name: String,
        /// Subset of tool names that are allowed (empty = all).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        allowed_tools: Vec<String>,
    },
    /// A custom client-defined tool.
    Custom {
        /// Tool name.
        name: String,
        /// Tool description.
        description: String,
        /// JSON Schema for the tool input.
        input_schema: serde_json::Value,
    },
}

/// MCP server reference attached to an agent: `{ type: "url", name, url }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct McpServer {
    /// Server type discriminator (e.g. `"url"`).
    #[serde(rename = "type")]
    pub server_type: String,
    /// Human-friendly server name.
    pub name: String,
    /// Server URL.
    pub url: String,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl McpServer {
    /// Create a new URL-typed MCP server reference.
    pub fn url(name: impl Into<String>, url: impl Into<String>) -> Self {
        Self {
            server_type: "url".to_string(),
            name: name.into(),
            url: url.into(),
            extra: HashMap::new(),
        }
    }
}

/// Reference to a skill (by id + version) attached to an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentSkillRef {
    /// Skill identifier.
    pub skill_id: String,
    /// Skill version identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl AgentSkillRef {
    /// Create a skill reference with an explicit version.
    pub fn new(skill_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            skill_id: skill_id.into(),
            version: Some(version.into()),
            extra: HashMap::new(),
        }
    }

    /// Create a skill reference without a pinned version.
    pub fn latest(skill_id: impl Into<String>) -> Self {
        Self {
            skill_id: skill_id.into(),
            version: None,
            extra: HashMap::new(),
        }
    }
}

/// A sub-agent member of a multiagent coordinator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiagentMember {
    /// Sub-agent identifier.
    pub id: String,
    /// Optional pinned version of the sub-agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Multiagent coordinator config: `{ type: "coordinator", agents: [...] }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Multiagent {
    /// Coordinator kind (e.g. `"coordinator"`).
    #[serde(rename = "type")]
    pub kind: String,
    /// Sub-agent members.
    pub agents: Vec<MultiagentMember>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A versioned managed agent.
///
/// Create once, then reference it from sessions by id (and optional version).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agent {
    /// Object type (always `"agent"`).
    #[serde(rename = "type")]
    pub object_type: String,
    /// Unique agent identifier.
    pub id: String,
    /// Version identifier (each update mints a new version).
    pub version: String,
    /// Human-friendly name.
    pub name: String,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Model spec.
    pub model: AgentModel,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Attached tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AgentTool>,
    /// Attached MCP servers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServer>,
    /// Attached skill references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<AgentSkillRef>,
    /// Optional multiagent coordinator config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiagent: Option<Multiagent>,
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

/// Request body for creating an agent.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AgentCreateRequest {
    /// Human-friendly name.
    pub name: String,
    /// Model spec.
    pub model: AgentModel,
    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Optional description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Attached tools.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<AgentTool>,
    /// Attached MCP servers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_servers: Vec<McpServer>,
    /// Attached skill references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<AgentSkillRef>,
    /// Optional multiagent coordinator config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiagent: Option<Multiagent>,
    /// Arbitrary metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl AgentCreateRequest {
    /// Create a new agent create request with the required name and model.
    pub fn new(name: impl Into<String>, model: impl Into<AgentModel>) -> Self {
        Self {
            name: name.into(),
            model: model.into(),
            system: None,
            description: None,
            tools: Vec::new(),
            mcp_servers: Vec::new(),
            skills: Vec::new(),
            multiagent: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add a tool.
    pub fn add_tool(mut self, tool: AgentTool) -> Self {
        self.tools.push(tool);
        self
    }

    /// Add an MCP server.
    pub fn add_mcp_server(mut self, server: McpServer) -> Self {
        self.mcp_servers.push(server);
        self
    }

    /// Add a skill reference.
    pub fn add_skill(mut self, skill: AgentSkillRef) -> Self {
        self.skills.push(skill);
        self
    }

    /// Set the multiagent coordinator config.
    pub fn multiagent(mut self, multiagent: Multiagent) -> Self {
        self.multiagent = Some(multiagent);
        self
    }

    /// Insert a metadata entry.
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Request body for updating an agent (each update mints a new version).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AgentUpdateRequest {
    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// New model spec.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<AgentModel>,
    /// New system prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// New description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Replacement tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<AgentTool>>,
    /// Replacement MCP servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mcp_servers: Option<Vec<McpServer>>,
    /// Replacement skill references.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<AgentSkillRef>>,
    /// Replacement multiagent config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiagent: Option<Multiagent>,
    /// Replacement metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

impl AgentUpdateRequest {
    /// Create an empty update request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the name.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the model.
    pub fn model(mut self, model: impl Into<AgentModel>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Response when listing agents (cursor-style pagination).
pub type AgentListResponse = PaginatedResponse<Agent>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_model_untagged_bare_string() {
        let parsed: AgentModel = serde_json::from_str("\"claude-opus-4-8\"").unwrap();
        assert!(matches!(parsed, AgentModel::Id(_)));
        assert_eq!(parsed.id(), "claude-opus-4-8");
        // round-trip
        let json = serde_json::to_string(&parsed).unwrap();
        assert_eq!(json, "\"claude-opus-4-8\"");
    }

    #[test]
    fn agent_model_untagged_spec_object() {
        let parsed: AgentModel =
            serde_json::from_str(r#"{"id":"claude-opus-4-8","speed":"fast"}"#).unwrap();
        match &parsed {
            AgentModel::Spec { id, speed } => {
                assert_eq!(id, "claude-opus-4-8");
                assert_eq!(speed.as_deref(), Some("fast"));
            }
            _ => panic!("expected spec variant"),
        }
        let value = serde_json::to_value(&parsed).unwrap();
        assert_eq!(value["id"], "claude-opus-4-8");
        assert_eq!(value["speed"], "fast");
    }

    #[test]
    fn agent_create_request_builder() {
        let req = AgentCreateRequest::new("triage", "claude-opus-4-8")
            .system("be helpful")
            .add_mcp_server(McpServer::url("forge", "https://mcp.example/forge"))
            .add_skill(AgentSkillRef::new("skill_123", "1"))
            .metadata("team", "secops");
        assert_eq!(req.name, "triage");
        assert_eq!(req.model.id(), "claude-opus-4-8");
        assert_eq!(req.mcp_servers.len(), 1);
        assert_eq!(req.skills.len(), 1);
        let value = serde_json::to_value(&req).unwrap();
        assert_eq!(value["system"], "be helpful");
    }
}
