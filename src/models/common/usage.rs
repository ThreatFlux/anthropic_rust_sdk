use super::*;

/// Usage statistics.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct Usage {
    /// Number of input tokens.
    #[serde(default)]
    pub input_tokens: u32,
    /// Number of output tokens.
    #[serde(default)]
    pub output_tokens: u32,
    /// Input tokens written into cache. Missing/null normalize to zero.
    #[serde(default, deserialize_with = "deserialize_nullable_counter")]
    pub cache_creation_input_tokens: u32,
    /// Input tokens read from cache. Missing/null normalize to zero.
    #[serde(default, deserialize_with = "deserialize_nullable_counter")]
    pub cache_read_input_tokens: u32,
    /// Cache creation breakdown by TTL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation: Option<CacheCreationUsage>,
    /// Built-in server-tool usage information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_tool_use: Option<ServerToolUsage>,
    /// Inference geography used for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inference_geo: Option<String>,
    /// Service tier used for the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
    /// Outcome of a fallback-credit redemption, when requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_credit: Option<serde_json::Value>,
    /// Inference speed used for the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<String>,
    /// Breakdown of output tokens by reasoning category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens_details: Option<OutputTokensDetails>,
    /// Iteration-level usage details, when returned by the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub iterations: Option<Vec<serde_json::Value>>,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

/// Output token breakdown returned by newer reasoning models.
#[derive(Debug, Clone, PartialEq, Deserialize, Default)]
pub struct OutputTokensDetails {
    /// Number of tokens used for model thinking.
    #[serde(default)]
    pub thinking_tokens: u32,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

/// Cache-creation usage breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct CacheCreationUsage {
    /// Input tokens cached with 5-minute TTL.
    #[serde(default)]
    pub ephemeral_5m_input_tokens: u32,
    /// Input tokens cached with 1-hour TTL.
    #[serde(default)]
    pub ephemeral_1h_input_tokens: u32,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

/// Built-in server-tool usage stats.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
pub struct ServerToolUsage {
    /// Number of web-search requests made by the model.
    #[serde(default)]
    pub web_search_requests: u32,
    /// Number of web-fetch requests made by the model.
    #[serde(default)]
    pub web_fetch_requests: u32,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl Usage {
    /// Create new usage stats.
    pub fn new(input_tokens: u32, output_tokens: u32) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
            cache_creation: None,
            server_tool_use: None,
            inference_geo: None,
            service_tier: None,
            fallback_credit: None,
            speed: None,
            output_tokens_details: None,
            iterations: None,
            extra: ExtraFields::new(),
        }
    }

    /// Get total input tokens across uncached and cache-related token usage.
    pub fn total_input_tokens(&self) -> u32 {
        self.input_tokens + self.cache_creation_input_tokens + self.cache_read_input_tokens
    }

    /// Get total tokens.
    pub fn total_tokens(&self) -> u32 {
        self.total_input_tokens() + self.output_tokens
    }
}
impl Serialize for Usage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let Self {
            input_tokens,
            output_tokens,
            cache_creation_input_tokens,
            cache_read_input_tokens,
            cache_creation,
            server_tool_use,
            inference_geo,
            service_tier,
            fallback_credit,
            speed,
            output_tokens_details,
            iterations,
            extra,
        } = self;
        serialize_fields!(serializer, None, extra;
            input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens;
            cache_creation, server_tool_use, inference_geo, service_tier, fallback_credit,
            speed, output_tokens_details, iterations)
    }
}

impl Serialize for OutputTokensDetails {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![(
            "thinking_tokens",
            serde_json::to_value(self.thinking_tokens).map_err(serde::ser::Error::custom)?,
        )];
        serialize_object(serializer, None, fields, &self.extra, &["thinking_tokens"])
    }
}

impl Serialize for CacheCreationUsage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![
            (
                "ephemeral_5m_input_tokens",
                serde_json::to_value(self.ephemeral_5m_input_tokens)
                    .map_err(serde::ser::Error::custom)?,
            ),
            (
                "ephemeral_1h_input_tokens",
                serde_json::to_value(self.ephemeral_1h_input_tokens)
                    .map_err(serde::ser::Error::custom)?,
            ),
        ];
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &["ephemeral_5m_input_tokens", "ephemeral_1h_input_tokens"],
        )
    }
}

impl Serialize for ServerToolUsage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![
            (
                "web_search_requests",
                serde_json::to_value(self.web_search_requests)
                    .map_err(serde::ser::Error::custom)?,
            ),
            (
                "web_fetch_requests",
                serde_json::to_value(self.web_fetch_requests).map_err(serde::ser::Error::custom)?,
            ),
        ];
        serialize_object(
            serializer,
            None,
            fields,
            &self.extra,
            &["web_search_requests", "web_fetch_requests"],
        )
    }
}

pub(super) fn deserialize_nullable_counter<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<u32, D::Error> {
    Ok(Option::<u32>::deserialize(deserializer)?.unwrap_or_default())
}
