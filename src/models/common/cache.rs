use super::*;

/// Cache control for prompt caching.
///
/// Attach to a content block, tool, or system block to mark a cache breakpoint,
/// or set [`crate::models::message::MessageRequest::cache_control`] to auto-cache
/// the last cacheable block.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CacheControl {
    /// Type of cache control (always `"ephemeral"`).
    #[serde(rename = "type")]
    pub cache_type: String,
    /// Time-to-live for the cache entry: `"5m"` (default) or `"1h"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl CacheControl {
    /// Create ephemeral cache control with the default 5-minute TTL.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
            extra: ExtraFields::new(),
        }
    }

    /// Create ephemeral cache control with a 1-hour TTL.
    pub fn ephemeral_1h() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
            extra: ExtraFields::new(),
        }
    }
}
impl Serialize for CacheControl {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![(
            "type",
            serde_json::to_value(&self.cache_type).map_err(serde::ser::Error::custom)?,
        )];
        if let Some(value) = &self.ttl {
            fields.push((
                "ttl",
                serde_json::to_value(value).map_err(serde::ser::Error::custom)?,
            ));
        }
        serialize_object(serializer, None, fields, &self.extra, &["type", "ttl"])
    }
}
