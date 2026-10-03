use super::*;

/// Tool definition for client-side function calling and server-side tools.
///
/// Custom tools set `name`, `description`, and `input_schema`. Server tools
/// (web search, code execution, bash, text editor, memory, ...) set `tool_type`
/// to a versioned identifier and a fixed `name`; use the dedicated constructors.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Tool {
    /// Tool type. Omitted or `"custom"` for client tools; a versioned identifier for server
    /// tools (e.g. `web_search_20260209`, `code_execution_20260120`).
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    /// Tool name.
    pub name: String,
    /// Tool description (custom tools).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input schema as JSON Schema (custom tools).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<serde_json::Value>,
    /// Require schema-valid tool arguments (strict tool use).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
    /// Cache control breakpoint for prompt caching.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
    /// Extra server-tool configuration fields (e.g. `max_uses`, `allowed_domains`).
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl Tool {
    /// Whether this is a client tool. An omitted type and the explicit `custom`
    /// type both represent a client tool; server type tags require server execution.
    pub fn is_client(&self) -> bool {
        self.tool_type
            .as_deref()
            .is_none_or(|kind| kind == "custom")
    }

    /// Create a new custom tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            tool_type: None,
            name: name.into(),
            description: Some(description.into()),
            input_schema: Some(input_schema),
            strict: None,
            cache_control: None,
            extra: HashMap::new(),
        }
    }

    /// Create a server-side tool by type + name.
    pub fn server(tool_type: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            tool_type: Some(tool_type.into()),
            name: name.into(),
            description: None,
            input_schema: None,
            strict: None,
            cache_control: None,
            extra: HashMap::new(),
        }
    }

    /// Built-in web search tool (`web_search_20260209`).
    pub fn web_search() -> Self {
        Self::server("web_search_20260209", "web_search")
    }

    /// Built-in web fetch tool (`web_fetch_20260209`).
    pub fn web_fetch() -> Self {
        Self::server("web_fetch_20260209", "web_fetch")
    }

    /// Built-in code execution tool (`code_execution_20260120`).
    pub fn code_execution() -> Self {
        Self::server("code_execution_20260120", "code_execution")
    }

    /// Built-in bash tool (`bash_20250124`).
    pub fn bash() -> Self {
        Self::server("bash_20250124", "bash")
    }

    /// Built-in text editor tool (`text_editor_20250728`).
    pub fn text_editor() -> Self {
        Self::server("text_editor_20250728", "str_replace_based_edit_tool")
    }

    /// Built-in memory tool (`memory_20250818`).
    pub fn memory() -> Self {
        Self::server("memory_20250818", "memory")
    }

    /// Enable strict tool use (schema-valid arguments).
    pub fn with_strict(mut self, strict: bool) -> Self {
        self.strict = Some(strict);
        self
    }

    /// Attach a cache-control breakpoint to this tool.
    pub fn with_cache_control(mut self, cache_control: CacheControl) -> Self {
        self.cache_control = Some(cache_control);
        self
    }

    /// Set an extra server-tool configuration field.
    pub fn with_config(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.extra.insert(key.into(), value);
        self
    }
}

/// Tool-selection policy encoded as a tagged API object.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub enum ToolChoice {
    /// Let the model decide whether to invoke a tool.
    #[default]
    Auto,
    /// Require some tool.
    Any,
    /// Require the named tool.
    Tool { name: String },
    /// Disable tool use.
    None,
    /// Automatic selection with an explicitly supplied parallel flag.
    AutoWithOptions { disable_parallel_tool_use: bool },
    /// Required selection with an explicitly supplied parallel flag.
    AnyWithOptions { disable_parallel_tool_use: bool },
    /// Named selection with an explicitly supplied parallel flag.
    ToolWithOptions {
        name: String,
        disable_parallel_tool_use: bool,
    },
}

impl ToolChoice {
    /// Disable tool use.
    pub fn none() -> Self {
        Self::None
    }
    /// The canonical API discriminator, independent of optional controls.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Auto | Self::AutoWithOptions { .. } => "auto",
            Self::Any | Self::AnyWithOptions { .. } => "any",
            Self::Tool { .. } | Self::ToolWithOptions { .. } => "tool",
            Self::None => "none",
        }
    }
    /// The forced tool's name, if any.
    pub fn forced_tool_name(&self) -> Option<&str> {
        match self {
            Self::Tool { name } | Self::ToolWithOptions { name, .. } => Some(name),
            _ => None,
        }
    }
    /// Whether parallel tool use was explicitly disabled or enabled.
    pub fn disable_parallel_tool_use(&self) -> Option<bool> {
        match self {
            Self::AutoWithOptions {
                disable_parallel_tool_use,
            }
            | Self::AnyWithOptions {
                disable_parallel_tool_use,
            }
            | Self::ToolWithOptions {
                disable_parallel_tool_use,
                ..
            } => Some(*disable_parallel_tool_use),
            _ => None,
        }
    }
    /// Set the parallel control; `none` does not accept this field.
    pub fn with_disable_parallel_tool_use(self, disabled: bool) -> crate::Result<Self> {
        Ok(match self {
            Self::Auto | Self::AutoWithOptions { .. } => Self::AutoWithOptions {
                disable_parallel_tool_use: disabled,
            },
            Self::Any | Self::AnyWithOptions { .. } => Self::AnyWithOptions {
                disable_parallel_tool_use: disabled,
            },
            Self::Tool { name } | Self::ToolWithOptions { name, .. } => Self::ToolWithOptions {
                name,
                disable_parallel_tool_use: disabled,
            },
            Self::None => {
                return Err(crate::AnthropicError::invalid_input(
                    "none tool choice cannot specify parallel tool use",
                ));
            }
        })
    }
}

impl Serialize for ToolChoice {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serde_json::Map::new();
        object.insert("type".into(), self.kind().into());
        if let Some(name) = self.forced_tool_name() {
            if name.is_empty() {
                return Err(serde::ser::Error::custom(
                    "forced tool name cannot be empty",
                ));
            }
            object.insert("name".into(), name.into());
        }
        if let Some(disabled) = self.disable_parallel_tool_use() {
            object.insert("disable_parallel_tool_use".into(), disabled.into());
        }
        object.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for ToolChoice {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = Value::deserialize(deserializer)?;
        // Historic null cannot distinguish Auto from Any; Auto is deterministic.
        if value.is_null() {
            return Ok(Self::Auto);
        }
        let object = value
            .as_object()
            .ok_or_else(|| serde::de::Error::custom("tool choice must be an object"))?;
        let choice = parse_tool_choice(object).map_err(serde::de::Error::custom)?;
        match object.get("disable_parallel_tool_use") {
            None => Ok(choice),
            Some(Value::Bool(disabled)) => choice
                .with_disable_parallel_tool_use(*disabled)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "parallel tool-use flag must be a boolean",
            )),
        }
    }
}

fn parse_tool_choice(object: &serde_json::Map<String, Value>) -> Result<ToolChoice, &'static str> {
    if object
        .keys()
        .any(|key| !["type", "name", "disable_parallel_tool_use"].contains(&key.as_str()))
    {
        return Err("unexpected tool-choice field");
    }
    let kind = match object.get("type") {
        Some(Value::String(kind)) => kind.as_str(),
        None if object.len() == 1 && object.contains_key("name") => "tool",
        _ => return Err("tool choice requires a string type"),
    };
    let name = object.get("name");
    match kind {
        "auto" if name.is_none() => Ok(ToolChoice::Auto),
        "any" if name.is_none() => Ok(ToolChoice::Any),
        "none" if name.is_none() => Ok(ToolChoice::None),
        "tool" => Ok(ToolChoice::Tool {
            name: name
                .and_then(Value::as_str)
                .filter(|name| !name.is_empty())
                .ok_or("tool choice requires a nonempty name")?
                .to_owned(),
        }),
        _ => Err("invalid tool choice type or fields"),
    }
}

impl Serialize for Tool {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut fields = vec![("name", Value::String(self.name.clone()))];
        if let Some(value) = &self.tool_type {
            fields.push(("type", Value::String(value.clone())));
        }
        if let Some(value) = &self.description {
            fields.push(("description", Value::String(value.clone())));
        }
        if let Some(value) = &self.input_schema {
            fields.push(("input_schema", value.clone()));
        }
        if let Some(value) = self.strict {
            fields.push(("strict", Value::Bool(value)));
        }
        if let Some(value) = &self.cache_control {
            fields.push((
                "cache_control",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &[
                "type",
                "name",
                "description",
                "input_schema",
                "strict",
                "cache_control",
            ],
        )
    }
}
