use super::*;

type Validator = Arc<dyn Fn(&Value) -> Result<()> + Send + Sync>;
type Callback = Arc<dyn Fn(Value) -> BoxFuture<'static, Result<ToolResultContent>> + Send + Sync>;

#[derive(Clone)]
pub(super) struct RegisteredTool {
    pub(super) definition: Tool,
    pub(super) validator: Validator,
    pub(super) callback: Callback,
}

/// Explicit callbacks and input validators for custom client tools.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    pub(super) tools: BTreeMap<String, RegisteredTool>,
}
impl ToolRegistry {
    /// Create an empty registry; no tool executes without registration.
    pub fn new() -> Self {
        Self::default()
    }
    /// Register a custom tool, a synchronous decoder/schema validator, and an
    /// async callback. Validators run for every call in a turn before any callback.
    pub fn register<V, F, Fut>(&mut self, definition: Tool, validator: V, callback: F) -> Result<()>
    where
        V: Fn(&Value) -> Result<()> + Send + Sync + 'static,
        F: Fn(Value) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolResultContent>> + Send + 'static,
    {
        if definition.name.is_empty()
            || !definition.is_client()
            || !definition
                .input_schema
                .as_ref()
                .is_some_and(Value::is_object)
        {
            return Err(AnthropicError::invalid_input(
                "register a named custom tool with an object input schema",
            ));
        }
        if self.tools.contains_key(&definition.name) {
            return Err(AnthropicError::invalid_input(
                "duplicate registered tool name",
            ));
        }
        // Check flattened reserved keys before saving a definition.
        serde_json::to_value(&definition)?;
        self.tools.insert(
            definition.name.clone(),
            RegisteredTool {
                definition,
                validator: Arc::new(validator),
                callback: Arc::new(move |input| callback(input).boxed()),
            },
        );
        Ok(())
    }
    /// Register a typed input decoder. The caller supplies the corresponding
    /// schema; use `register` to add validation beyond serde's field checks.
    pub fn register_typed<I, F, Fut>(&mut self, definition: Tool, callback: F) -> Result<()>
    where
        I: DeserializeOwned + Send + 'static,
        F: Fn(I) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<ToolResultContent>> + Send + 'static,
    {
        let callback = Arc::new(callback);
        self.register(
            definition,
            |value| {
                serde_json::from_value::<I>(value.clone())
                    .map(|_| ())
                    .map_err(|_| {
                        AnthropicError::invalid_input(
                            "tool input does not match registered decoder",
                        )
                    })
            },
            move |value| {
                let callback = callback.clone();
                async move {
                    let input = serde_json::from_value::<I>(value).map_err(|_| {
                        AnthropicError::invalid_input(
                            "tool input does not match registered decoder",
                        )
                    })?;
                    callback(input).await
                }
            },
        )
    }
    /// Definitions offered by the registry, in deterministic name order.
    pub fn definitions(&self) -> Vec<Tool> {
        self.tools
            .values()
            .map(|tool| tool.definition.clone())
            .collect()
    }
    /// Number of explicitly registered callbacks.
    pub fn len(&self) -> usize {
        self.tools.len()
    }
    /// Whether the registry has no callbacks.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}
