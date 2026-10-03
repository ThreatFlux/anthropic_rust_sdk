//! Common data types shared across API models.
//!
//! Content and usage schemas were verified 2026-10-03 against Anthropic's
//! [stable content parameters](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/content_block_param.py)
//! and [beta response blocks](https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/beta/beta_content_block.py).
//! Unknown objects round-trip unchanged; known optional null/missing fields may
//! normalize to omission, and nullable cache counters normalize to zero.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Additional protocol fields retained without interpretation.
pub type ExtraFields = HashMap<String, Value>;

/// A complete object with a string discriminator. The payload cannot be mutated
/// independently of its discriminator, so unknown protocol blocks round-trip intact.
#[derive(Debug, Clone, PartialEq)]
pub struct RawContentBlock(Value);

impl RawContentBlock {
    /// Validate a raw protocol object. Service support for an unfamiliar type is unknown.
    pub fn new(value: Value) -> crate::Result<Self> {
        if !value.is_object() || value.get("type").and_then(Value::as_str).is_none() {
            return Err(crate::AnthropicError::invalid_input(
                "raw block requires an object with a string type",
            ));
        }
        Ok(Self(value))
    }
    /// The original discriminator.
    pub fn block_type(&self) -> &str {
        self.0["type"].as_str().expect("validated discriminator")
    }
    /// The complete immutable payload.
    pub fn as_value(&self) -> &Value {
        &self.0
    }
    /// Consume the wrapper without changing any payload values.
    pub fn into_value(self) -> Value {
        self.0
    }
}

impl Serialize for RawContentBlock {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}
impl<'de> Deserialize<'de> for RawContentBlock {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Self::new(Value::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

fn deserialize_unknown<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
    known: &[&str],
) -> Result<RawContentBlock, D::Error> {
    let raw = RawContentBlock::deserialize(deserializer)?;
    if known.contains(&raw.block_type()) {
        return Err(serde::de::Error::custom(
            "malformed recognized protocol block",
        ));
    }
    Ok(raw)
}

fn serialize_object<S: serde::Serializer>(
    serializer: S,
    tag: Option<&str>,
    fields: Vec<(&str, Value)>,
    extra: &ExtraFields,
    reserved: &[&str],
) -> Result<S::Ok, S::Error> {
    let mut object = serde_json::Map::new();
    if let Some(tag) = tag {
        object.insert("type".into(), Value::String(tag.into()));
    }
    for (key, value) in fields {
        object.insert(key.into(), value);
    }
    for (key, value) in extra {
        if reserved.contains(&key.as_str()) || (tag.is_some() && key == "type") {
            return Err(serde::ser::Error::custom(format!(
                "extra field collides with reserved field {key}"
            )));
        }
        object.insert(key.clone(), value.clone());
    }
    object.serialize(serializer)
}

macro_rules! serialize_fields {
    ($serializer:expr, $tag:expr, $extra:expr; $($required:ident),* ; $($optional:ident),*) => {{
        #[allow(unused_mut)]
        let mut fields = vec![$((stringify!($required), serde_json::to_value($required).map_err(serde::ser::Error::custom)?)),*];
        $(if let Some(value) = $optional { fields.push((stringify!($optional), serde_json::to_value(value).map_err(serde::ser::Error::custom)?)); })*
        serialize_object($serializer, $tag, fields, $extra, &[$(stringify!($required),)* $(stringify!($optional),)*])
    }};
}
macro_rules! serialize_variant {
    ($serializer:expr, $tag:expr, $extra:expr; $($required:ident),* ; $($optional:ident),*) => {
        serialize_fields!($serializer, Some($tag), $extra; $($required),* ; $($optional),*)
    };
}

fn deserialize_object_value<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    let value = Value::deserialize(deserializer)?;
    if value.is_object() {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("tool input must be an object"))
    }
}

mod cache;
mod citations;
mod content;
mod content_wire;
mod replay;
mod roles;
mod sources;
mod stops;
#[cfg(test)]
mod tests;
mod tools;
mod usage;

pub use cache::*;
pub use citations::*;
pub use content::*;
use content_wire::*;
pub use replay::*;
pub use roles::*;
pub use sources::*;
pub use stops::*;
pub use tools::*;
pub use usage::*;
