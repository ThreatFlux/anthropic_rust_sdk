use super::*;

/// Citation information attached to text content.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[non_exhaustive]
pub enum TextCitation {
    /// Character span citation inside a document.
    CharLocation {
        cited_text: String,
        document_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_title: Option<String>,
        start_char_index: u32,
        end_char_index: u32,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Page range citation inside a document.
    PageLocation {
        cited_text: String,
        document_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_title: Option<String>,
        start_page_number: u32,
        end_page_number: u32,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Content block index citation for content-based documents.
    ContentBlockLocation {
        cited_text: String,
        document_index: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        file_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        document_title: Option<String>,
        start_block_index: u32,
        end_block_index: u32,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Citation that references a built-in search result.
    SearchResultLocation {
        search_result_index: u32,
        source: String,
        title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cited_text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        start_block_index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        end_block_index: Option<u32>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// Citation that references a web-search result.
    WebSearchResultLocation {
        #[serde(skip_serializing_if = "Option::is_none")]
        cited_text: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        url: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        encrypted_index: Option<String>,
        /// Additional fields retained for replay.
        #[serde(flatten, default)]
        extra: ExtraFields,
    },
    /// An unfamiliar discriminator, preserved as a complete object.
    #[serde(untagged)]
    Unknown(#[serde(deserialize_with = "deserialize_unknown_textcitation")] RawContentBlock),
}

/// Citation settings for a document input block.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DocumentCitations {
    /// Whether citations are enabled for this document.
    pub enabled: bool,
    /// Forward-compatible protocol fields. Reserved typed keys cannot be overridden.
    #[serde(flatten, default)]
    pub extra: ExtraFields,
}

impl DocumentCitations {
    /// Enable citations for this document.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            extra: ExtraFields::new(),
        }
    }

    /// Disable citations for this document.
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            extra: ExtraFields::new(),
        }
    }
}
pub(super) fn deserialize_unknown_textcitation<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RawContentBlock, D::Error> {
    deserialize_unknown(
        deserializer,
        &[
            "char_location",
            "page_location",
            "content_block_location",
            "search_result_location",
            "web_search_result_location",
        ],
    )
}

impl Serialize for TextCitation {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::CharLocation { .. }
            | Self::PageLocation { .. }
            | Self::ContentBlockLocation { .. } => self.serialize_document_location(serializer),
            Self::SearchResultLocation {
                search_result_index,
                source,
                title,
                cited_text,
                start_block_index,
                end_block_index,
                extra,
            } => {
                serialize_variant!(serializer, "search_result_location", extra; search_result_index, source, title; cited_text, start_block_index, end_block_index)
            }
            Self::WebSearchResultLocation {
                cited_text,
                title,
                url,
                encrypted_index,
                extra,
            } => {
                serialize_variant!(serializer, "web_search_result_location", extra; ; cited_text, title, url, encrypted_index)
            }
            Self::Unknown(raw) => {
                if [
                    "char_location",
                    "page_location",
                    "content_block_location",
                    "search_result_location",
                    "web_search_result_location",
                ]
                .contains(&raw.block_type())
                {
                    return Err(serde::ser::Error::custom(
                        "recognized types must use a typed variant",
                    ));
                }
                raw.serialize(serializer)
            }
        }
    }
}

impl TextCitation {
    fn serialize_document_location<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match self {
            Self::CharLocation {
                cited_text,
                document_index,
                file_id,
                document_title,
                start_char_index,
                end_char_index,
                extra,
            } => {
                serialize_variant!(serializer, "char_location", extra; cited_text, document_index, start_char_index, end_char_index; file_id, document_title)
            }
            Self::PageLocation {
                cited_text,
                document_index,
                file_id,
                document_title,
                start_page_number,
                end_page_number,
                extra,
            } => {
                serialize_variant!(serializer, "page_location", extra; cited_text, document_index, start_page_number, end_page_number; file_id, document_title)
            }
            Self::ContentBlockLocation {
                cited_text,
                document_index,
                file_id,
                document_title,
                start_block_index,
                end_block_index,
                extra,
            } => {
                serialize_variant!(serializer, "content_block_location", extra; cited_text, document_index, start_block_index, end_block_index; file_id, document_title)
            }
            _ => unreachable!("document citation selected by serializer"),
        }
    }
}

impl Serialize for DocumentCitations {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[allow(unused_mut)]
        let mut fields = vec![(
            "enabled",
            serde_json::to_value(self.enabled).map_err(serde::ser::Error::custom)?,
        )];
        serialize_object(serializer, None, fields, &self.extra, &["enabled"])
    }
}
