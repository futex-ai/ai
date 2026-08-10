//! Serialization helpers for MCP content blocks and embedded resources.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use serde_json::Value;

use super::{McpAnnotations, McpContentBlock, McpResourceContents};

#[derive(Serialize)]
#[serde(tag = "type")]
enum ContentRef<'a> {
    #[serde(rename = "text")]
    Text {
        text: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<&'a McpAnnotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
    },
    #[serde(rename = "image")]
    Image {
        data: &'a str,
        #[serde(rename = "mimeType")]
        mime_type: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<&'a McpAnnotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
    },
    #[serde(rename = "audio")]
    Audio {
        data: &'a str,
        #[serde(rename = "mimeType")]
        mime_type: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<&'a McpAnnotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
    },
    #[serde(rename = "resource_link")]
    ResourceLink {
        name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<&'a str>,
        uri: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        description: Option<&'a str>,
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<&'a str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<&'a McpAnnotations>,
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<&'a serde_json::Number>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
    },
    #[serde(rename = "resource")]
    EmbeddedResource {
        resource: &'a McpResourceContents,
        #[serde(skip_serializing_if = "Option::is_none")]
        annotations: Option<&'a McpAnnotations>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
    },
}

impl Serialize for McpContentBlock {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Text {
                text,
                annotations,
                meta,
            } => ContentRef::Text {
                text,
                annotations: annotations.as_ref(),
                meta: meta.as_ref(),
            }
            .serialize(serializer),
            Self::Image {
                data,
                mime_type,
                annotations,
                meta,
            } => ContentRef::Image {
                data,
                mime_type,
                annotations: annotations.as_ref(),
                meta: meta.as_ref(),
            }
            .serialize(serializer),
            Self::Audio {
                data,
                mime_type,
                annotations,
                meta,
            } => ContentRef::Audio {
                data,
                mime_type,
                annotations: annotations.as_ref(),
                meta: meta.as_ref(),
            }
            .serialize(serializer),
            Self::ResourceLink {
                name,
                title,
                uri,
                description,
                mime_type,
                annotations,
                size,
                meta,
            } => ContentRef::ResourceLink {
                name,
                title: title.as_deref(),
                uri,
                description: description.as_deref(),
                mime_type: mime_type.as_deref(),
                annotations: annotations.as_ref(),
                size: size.as_ref(),
                meta: meta.as_ref(),
            }
            .serialize(serializer),
            Self::EmbeddedResource {
                resource,
                annotations,
                meta,
            } => ContentRef::EmbeddedResource {
                resource,
                annotations: annotations.as_ref(),
                meta: meta.as_ref(),
            }
            .serialize(serializer),
            Self::Unknown(value) => value.serialize(serializer),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceWire {
    uri: String,
    mime_type: Option<String>,
    #[serde(default, rename = "_meta")]
    meta: Option<BTreeMap<String, Value>>,
    text: Option<String>,
    blob: Option<String>,
}

impl<'de> Deserialize<'de> for McpResourceContents {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ResourceWire::deserialize(deserializer)?;
        match (wire.text, wire.blob) {
            (Some(text), None) => Ok(Self::Text {
                uri: wire.uri,
                mime_type: wire.mime_type,
                meta: wire.meta,
                text,
            }),
            (None, Some(blob)) => Ok(Self::Blob {
                uri: wire.uri,
                mime_type: wire.mime_type,
                meta: wire.meta,
                blob,
            }),
            _ => Err(D::Error::custom(
                "embedded resource requires exactly one of text or blob",
            )),
        }
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum ResourceRef<'a> {
    Text {
        uri: &'a str,
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<&'a str>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
        text: &'a str,
    },
    Blob {
        uri: &'a str,
        #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
        mime_type: Option<&'a str>,
        #[serde(rename = "_meta", skip_serializing_if = "Option::is_none")]
        meta: Option<&'a BTreeMap<String, Value>>,
        blob: &'a str,
    },
}

impl Serialize for McpResourceContents {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Text {
                uri,
                mime_type,
                meta,
                text,
            } => ResourceRef::Text {
                uri,
                mime_type: mime_type.as_deref(),
                meta: meta.as_ref(),
                text,
            }
            .serialize(serializer),
            Self::Blob {
                uri,
                mime_type,
                meta,
                blob,
            } => ResourceRef::Blob {
                uri,
                mime_type: mime_type.as_deref(),
                meta: meta.as_ref(),
                blob,
            }
            .serialize(serializer),
        }
    }
}
