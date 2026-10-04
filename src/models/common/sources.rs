use super::*;

/// Image source types.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ImageSource {
    /// Base64 encoded image.
    Base64 {
        media_type: String,
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Publicly accessible image URL.
    Url {
        url: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Previously uploaded file reference.
    File {
        file_id: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_imagesource")] RawContentBlock),
}

impl ImageSource {
    /// Create a base64 image source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create from image bytes.
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let data = BASE64_STANDARD.encode(bytes);
        Self::base64(media_type, data)
    }

    /// Create a URL image source.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url {
            url: url.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create a file-id image source.
    pub fn file(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: file_id.into(),
            extra: ExtraFields::new(),
        }
    }
}

/// Document source types.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum DocumentSource {
    /// Base64 encoded document bytes.
    Base64 {
        media_type: String,
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Publicly accessible document URL.
    Url {
        url: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Previously uploaded file reference.
    File {
        file_id: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Inline text document source.
    Text {
        media_type: String,
        data: String,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Inline content-based document source.
    Content {
        #[serde(deserialize_with = "deserialize_document_content")]
        content: Vec<serde_json::Value>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_documentsource")] RawContentBlock),
}

impl DocumentSource {
    /// Create a base64 document source.
    pub fn base64(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            media_type: media_type.into(),
            data: data.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create from bytes using base64 encoding.
    pub fn from_bytes(media_type: impl Into<String>, bytes: &[u8]) -> Self {
        use base64::prelude::*;
        let data = BASE64_STANDARD.encode(bytes);
        Self::base64(media_type, data)
    }

    /// Create a URL document source.
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url {
            url: url.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create a file-id document source.
    pub fn file(file_id: impl Into<String>) -> Self {
        Self::File {
            file_id: file_id.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create an inline text document source.
    pub fn text(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Text {
            media_type: media_type.into(),
            data: data.into(),
            extra: ExtraFields::new(),
        }
    }

    /// Create an inline content document source.
    pub fn content(content: Vec<serde_json::Value>) -> Self {
        Self::Content {
            content,
            extra: ExtraFields::new(),
        }
    }
}
pub(super) fn deserialize_unknown_imagesource<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(deserializer, &["base64", "url", "file"])
}

impl Serialize for ImageSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Base64 {
                media_type,
                data,
                extra,
            } => {
                serialize_variant!(serializer, "base64", extra; media_type, data; )
            }
            Self::Url { url, extra } => {
                serialize_variant!(serializer, "url", extra; url; )
            }
            Self::File { file_id, extra } => {
                serialize_variant!(serializer, "file", extra; file_id; )
            }
            Self::Unknown(raw) => {
                if ["base64", "url", "file"].contains(&raw.block_type()) {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}

pub(super) fn deserialize_unknown_documentsource<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(deserializer, &["base64", "url", "file", "text", "content"])
}

impl Serialize for DocumentSource {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Base64 {
                media_type,
                data,
                extra,
            } => {
                serialize_variant!(serializer, "base64", extra; media_type, data; )
            }
            Self::Url { url, extra } => {
                serialize_variant!(serializer, "url", extra; url; )
            }
            Self::File { file_id, extra } => {
                serialize_variant!(serializer, "file", extra; file_id; )
            }
            Self::Text {
                media_type,
                data,
                extra,
            } => {
                serialize_variant!(serializer, "text", extra; media_type, data; )
            }
            Self::Content { content, extra } => {
                if !document_content_is_valid(content) {
                    return Err(serde::ser::Error::custom(
                        "inline document requires text or image content blocks",
                    ));
                }
                serialize_variant!(serializer, "content", extra; content; )
            }
            Self::Unknown(raw) => {
                if ["base64", "url", "file", "text", "content"].contains(&raw.block_type()) {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}
fn document_content_is_valid(content: &[Value]) -> bool {
    content.iter().all(|value| {
        matches!(
            ContentBlock::raw(value.clone()),
            Ok(ContentBlock::Text { .. } | ContentBlock::Image { .. } | ContentBlock::Unknown(_))
        )
    })
}
pub(super) fn deserialize_document_content<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<Value>, D::Error> {
    let content = Vec::<Value>::deserialize(deserializer)?;
    if document_content_is_valid(&content) {
        Ok(content)
    } else {
        Err(serde::de::Error::custom(
            "inline document requires text or image content blocks",
        ))
    }
}
