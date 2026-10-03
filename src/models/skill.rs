//! Skills API data models

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current Skills source object. Future discriminator strings and fields are retained.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentSkillSource {
    /// `custom`, `anthropic`, `anthropic_example`, `plugin`, or a future source.
    #[serde(rename = "type")]
    pub source_type: String,
    /// Additional source metadata.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Current Skills schema, selected by omitting the historical Skills beta header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentSkill {
    /// Skill identifier.
    pub id: String,
    /// Object discriminator, `skill`.
    #[serde(rename = "type")]
    pub object_type: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Human-readable display name.
    pub display_name: String,
    /// Newest version's ID, used in version paths.
    pub latest_version_id: String,
    /// Source object; distinct from the legacy string source.
    pub source: CurrentSkillSource,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Future response fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Current version schema. `id` addresses this version; epoch timestamps are legacy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentSkillVersion {
    /// Version identifier.
    pub id: String,
    /// Object discriminator, `skill_version`.
    #[serde(rename = "type")]
    pub object_type: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Description from SKILL.md.
    pub description: String,
    /// Immutable skill slug.
    pub name: String,
    /// Parent skill ID.
    pub skill_id: String,
    /// Future response fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Current token-cursor Skills page; `next_page` alone determines continuation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentSkillListResponse {
    /// Skills on this page.
    pub data: Vec<CurrentSkill>,
    /// Continuation token, absent at the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
    /// Future response fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Current token-cursor version page.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CurrentSkillVersionListResponse {
    /// Versions on this page.
    pub data: Vec<CurrentSkillVersion>,
    /// Continuation token, absent at the end.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
    /// Future response fields.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Create a current-schema skill without changing the legacy request type.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct CurrentSkillCreateRequest {
    /// Single-line display name; derives from SKILL.md if omitted.
    pub display_name: Option<String>,
    /// Files sharing one root directory and containing its SKILL.md.
    pub files: Vec<SkillFileUpload>,
}

impl CurrentSkillCreateRequest {
    /// Construct an empty upload request.
    pub fn new() -> Self {
        Self::default()
    }
    /// Set a human-readable display name.
    pub fn display_name(mut self, name: impl Into<String>) -> Self {
        self.display_name = Some(name.into());
        self
    }
    /// Add one upload part.
    pub fn add_file(mut self, file: SkillFileUpload) -> Self {
        self.files.push(file);
        self
    }
    /// Validate the current request shape and safe upload filenames.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self
            .display_name
            .as_ref()
            .is_some_and(|name| name.chars().count() > 255 || name.contains(['\n', '\r']))
        {
            return Err(crate::error::AnthropicError::invalid_input(
                "Skill display_name must be single-line and at most 255 characters",
            ));
        }
        validate_current_skill_files(&self.files)
    }
}

/// Current version upload request with validated file layout.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct CurrentSkillVersionCreateRequest {
    /// Skill files to upload.
    pub files: Vec<SkillFileUpload>,
}

impl CurrentSkillVersionCreateRequest {
    /// Construct an empty version upload request.
    pub fn new() -> Self {
        Self::default()
    }
    /// Add one upload part.
    pub fn add_file(mut self, file: SkillFileUpload) -> Self {
        self.files.push(file);
        self
    }
    /// Check the same documented layout as current skill creation.
    pub fn validate(&self) -> crate::error::Result<()> {
        validate_current_skill_files(&self.files)
    }
}

fn validate_current_skill_files(files: &[SkillFileUpload]) -> crate::error::Result<()> {
    use crate::error::AnthropicError;
    let mut root = None;
    let mut skill_file = false;
    let mut names = std::collections::HashSet::new();
    for file in files {
        let parts: Vec<_> = file.filename.split('/').collect();
        if parts.len() < 2
            || file.filename.contains('\\')
            || parts.iter().any(|part| matches!(*part, "" | "." | ".."))
        {
            return Err(AnthropicError::invalid_input(
                "Skill filenames must be relative paths under one root directory",
            ));
        }
        if root.is_some_and(|name| name != parts[0]) || !names.insert(&file.filename) {
            return Err(AnthropicError::invalid_input(
                "Skill filenames must be unique and share one root directory",
            ));
        }
        root = Some(parts[0]);
        skill_file |= parts.len() == 2 && parts[1] == "SKILL.md";
    }
    if !skill_file {
        return Err(AnthropicError::invalid_input(
            "Skill upload must include root/SKILL.md",
        ));
    }
    Ok(())
}

/// Latest skill version reference.
///
/// The API may return either a version ID string or an embedded version object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SkillLatestVersion {
    /// Version identifier string
    Id(String),
    /// Embedded version object
    Version(Box<SkillVersion>),
}

impl SkillLatestVersion {
    /// Returns the version identifier when available.
    pub fn version_id(&self) -> Option<&str> {
        match self {
            Self::Id(id) => Some(id.as_str()),
            Self::Version(version) => version.version.as_deref(),
        }
    }
}

/// A reusable skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Skill {
    /// Object type, usually `skill`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// Skill ID.
    pub id: String,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Human-friendly display title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_title: Option<String>,
    /// Latest version reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<SkillLatestVersion>,
    /// Source of the skill (`custom`, `anthropic`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// ISO 8601 update timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// A specific skill version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillVersion {
    /// Object type, usually `skill_version`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// Unique identifier for the skill version.
    pub id: String,
    /// ISO 8601 creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<DateTime<Utc>>,
    /// Description extracted from `SKILL.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Top-level directory name extracted from uploaded files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    /// Human-readable skill name extracted from `SKILL.md`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Parent skill ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_id: Option<String>,
    /// Version identifier (epoch timestamp string for custom skills).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Skills list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillListResponse {
    /// List of skills.
    #[serde(default)]
    pub data: Vec<Skill>,
    /// Whether more pages are available.
    #[serde(default)]
    pub has_more: bool,
    /// Pagination token for next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Skill versions list response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillVersionListResponse {
    /// List of skill versions.
    #[serde(default)]
    pub data: Vec<SkillVersion>,
    /// Whether more pages are available.
    #[serde(default)]
    pub has_more: bool,
    /// Pagination token for next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Query parameters for listing skills.
#[derive(Debug, Clone, Default)]
pub struct SkillListParams {
    /// Max items to return.
    pub limit: Option<u32>,
    /// Page token.
    pub page: Option<String>,
    /// Source filter, e.g. `custom` or `anthropic`.
    pub source: Option<String>,
}

impl SkillListParams {
    /// Create empty list params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page size.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set page token.
    pub fn with_page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    /// Filter by source.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

/// Query parameters for listing skill versions.
#[derive(Debug, Clone, Default)]
pub struct SkillVersionListParams {
    /// Max items to return.
    pub limit: Option<u32>,
    /// Page token.
    pub page: Option<String>,
}

impl SkillVersionListParams {
    /// Create empty list params.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set page size.
    pub fn with_limit(mut self, limit: u32) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set page token.
    pub fn with_page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }
}

/// A single uploaded skill file for create/version operations.
#[derive(Debug, Clone, PartialEq)]
pub struct SkillFileUpload {
    /// Path-like filename submitted to the API (must include the top-level skill directory).
    pub filename: String,
    /// File bytes.
    pub content: Vec<u8>,
    /// MIME type for the upload part.
    pub mime_type: String,
}

impl SkillFileUpload {
    /// Create a new upload part.
    pub fn new(
        filename: impl Into<String>,
        content: Vec<u8>,
        mime_type: impl Into<String>,
    ) -> Self {
        Self {
            filename: filename.into(),
            content,
            mime_type: mime_type.into(),
        }
    }
}

/// Request body for creating a skill.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkillCreateRequest {
    /// Display title for the skill.
    pub display_title: Option<String>,
    /// Files to upload.
    pub files: Vec<SkillFileUpload>,
}

impl SkillCreateRequest {
    /// Create an empty create request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set display title.
    pub fn display_title(mut self, title: impl Into<String>) -> Self {
        self.display_title = Some(title.into());
        self
    }

    /// Add a file part.
    pub fn add_file(mut self, file: SkillFileUpload) -> Self {
        self.files.push(file);
        self
    }

    /// Validate request state.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.files.is_empty() {
            return Err(crate::error::AnthropicError::invalid_input(
                "Skill create request must include at least one file",
            ));
        }
        Ok(())
    }
}

/// Request body for creating a new version of an existing skill.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkillVersionCreateRequest {
    /// Files to upload.
    pub files: Vec<SkillFileUpload>,
}

impl SkillVersionCreateRequest {
    /// Create an empty version create request.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a file part.
    pub fn add_file(mut self, file: SkillFileUpload) -> Self {
        self.files.push(file);
        self
    }

    /// Validate request state.
    pub fn validate(&self) -> crate::error::Result<()> {
        if self.files.is_empty() {
            return Err(crate::error::AnthropicError::invalid_input(
                "Skill version create request must include at least one file",
            ));
        }
        Ok(())
    }
}

/// Response for deleting a skill.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillDeleteResponse {
    /// Deleted skill ID.
    pub id: String,
    /// Object type, usually `skill_deleted`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

/// Response for deleting a skill version.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkillVersionDeleteResponse {
    /// Deleted version ID.
    pub id: String,
    /// Object type, usually `skill_version_deleted`.
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub object_type: Option<String>,
    /// Additional fields not yet modeled explicitly.
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_create_request_validation() {
        let empty = SkillCreateRequest::new();
        assert!(empty.validate().is_err());

        let valid = SkillCreateRequest::new().add_file(SkillFileUpload::new(
            "my_skill/SKILL.md",
            b"# My Skill".to_vec(),
            "text/markdown",
        ));
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_skill_version_create_request_validation() {
        let empty = SkillVersionCreateRequest::new();
        assert!(empty.validate().is_err());

        let valid = SkillVersionCreateRequest::new().add_file(SkillFileUpload::new(
            "my_skill/SKILL.md",
            b"# My Skill".to_vec(),
            "text/markdown",
        ));
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn test_skill_latest_version_deserialization() {
        let id_value: SkillLatestVersion = serde_json::from_str("\"1746821713\"").unwrap();
        assert!(matches!(id_value, SkillLatestVersion::Id(_)));

        let obj_value: SkillLatestVersion = serde_json::from_str(
            r#"{
                "id": "skv_123",
                "type": "skill_version",
                "version": "1746821713"
            }"#,
        )
        .unwrap();
        assert!(matches!(obj_value, SkillLatestVersion::Version(_)));
    }
}
