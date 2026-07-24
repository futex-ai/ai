//! MCP content blocks with exact unknown-variant preservation.

use std::collections::BTreeMap;

use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Optional annotations attached to MCP content.
pub struct McpAnnotations {
    /// Intended recipients for this content.
    pub audience: Option<Vec<McpRole>>,
    /// Relative priority represented by a JSON number.
    pub priority: Option<serde_json::Number>,
    /// Optional ISO 8601 modification timestamp.
    pub last_modified: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
/// Intended audience for annotated MCP content.
pub enum McpRole {
    /// Human user.
    User,
    /// Model assistant.
    Assistant,
}

#[derive(Clone, Debug, PartialEq)]
/// One MCP content block.
pub enum McpContentBlock {
    /// UTF-8 text content.
    Text {
        /// Text payload.
        text: String,
        /// Optional content annotations.
        annotations: Option<McpAnnotations>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Base64-encoded image content.
    Image {
        /// Base64-encoded bytes.
        data: String,
        /// Image media type.
        mime_type: String,
        /// Optional content annotations.
        annotations: Option<McpAnnotations>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Base64-encoded audio content.
    Audio {
        /// Base64-encoded bytes.
        data: String,
        /// Audio media type.
        mime_type: String,
        /// Optional content annotations.
        annotations: Option<McpAnnotations>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Link to a resource the server can describe.
    ResourceLink {
        /// Programmatic resource name.
        name: String,
        /// Optional human-readable title.
        title: Option<String>,
        /// Resource URI.
        uri: String,
        /// Optional description.
        description: Option<String>,
        /// Optional media type.
        mime_type: Option<String>,
        /// Optional content annotations.
        annotations: Option<McpAnnotations>,
        /// Optional resource size.
        size: Option<serde_json::Number>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Resource contents embedded directly in the result.
    EmbeddedResource {
        /// Text or blob resource payload.
        resource: McpResourceContents,
        /// Optional content annotations.
        annotations: Option<McpAnnotations>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
    },
    /// Unrecognized future content object preserved byte-semantically as JSON.
    Unknown(Value),
}

#[derive(Clone, Debug, PartialEq)]
/// Contents nested inside an embedded MCP resource.
pub enum McpResourceContents {
    /// Text resource.
    Text {
        /// Resource URI.
        uri: String,
        /// Optional media type.
        mime_type: Option<String>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
        /// Text payload.
        text: String,
    },
    /// Base64-encoded binary resource.
    Blob {
        /// Resource URI.
        uri: String,
        /// Optional media type.
        mime_type: Option<String>,
        /// Optional extension metadata.
        meta: Option<BTreeMap<String, Value>>,
        /// Base64-encoded bytes.
        blob: String,
    },
}

#[derive(Deserialize)]
struct CommonContent {
    #[serde(default)]
    annotations: Option<McpAnnotations>,
    #[serde(default, rename = "_meta")]
    meta: Option<BTreeMap<String, Value>>,
}

#[derive(Deserialize)]
struct TextContent {
    text: String,
    #[serde(flatten)]
    common: CommonContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaContent {
    data: String,
    mime_type: String,
    #[serde(flatten)]
    common: CommonContent,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResourceLinkContent {
    name: String,
    title: Option<String>,
    uri: String,
    description: Option<String>,
    mime_type: Option<String>,
    #[serde(default)]
    annotations: Option<McpAnnotations>,
    size: Option<serde_json::Number>,
    #[serde(default, rename = "_meta")]
    meta: Option<BTreeMap<String, Value>>,
}

#[derive(Deserialize)]
struct EmbeddedResourceContent {
    resource: McpResourceContents,
    #[serde(flatten)]
    common: CommonContent,
}

impl<'de> Deserialize<'de> for McpContentBlock {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let Some(kind) = value.get("type").and_then(Value::as_str) else {
            return Err(D::Error::custom("MCP content block requires a string type"));
        };
        match kind {
            "text" => decode_text(value),
            "image" => decode_image(value),
            "audio" => decode_audio(value),
            "resource_link" => decode_resource_link(value),
            "resource" => decode_embedded_resource(value),
            _ => Ok(Self::Unknown(value)),
        }
    }
}

fn decode_text<E: serde::de::Error>(value: Value) -> Result<McpContentBlock, E> {
    let decoded: TextContent = decode_known(value)?;
    Ok(McpContentBlock::Text {
        text: decoded.text,
        annotations: decoded.common.annotations,
        meta: decoded.common.meta,
    })
}

fn decode_image<E: serde::de::Error>(value: Value) -> Result<McpContentBlock, E> {
    let decoded: MediaContent = decode_known(value)?;
    Ok(McpContentBlock::Image {
        data: decoded.data,
        mime_type: decoded.mime_type,
        annotations: decoded.common.annotations,
        meta: decoded.common.meta,
    })
}

fn decode_audio<E: serde::de::Error>(value: Value) -> Result<McpContentBlock, E> {
    let decoded: MediaContent = decode_known(value)?;
    Ok(McpContentBlock::Audio {
        data: decoded.data,
        mime_type: decoded.mime_type,
        annotations: decoded.common.annotations,
        meta: decoded.common.meta,
    })
}

fn decode_resource_link<E: serde::de::Error>(value: Value) -> Result<McpContentBlock, E> {
    let decoded: ResourceLinkContent = decode_known(value)?;
    Ok(McpContentBlock::ResourceLink {
        name: decoded.name,
        title: decoded.title,
        uri: decoded.uri,
        description: decoded.description,
        mime_type: decoded.mime_type,
        annotations: decoded.annotations,
        size: decoded.size,
        meta: decoded.meta,
    })
}

fn decode_embedded_resource<E: serde::de::Error>(value: Value) -> Result<McpContentBlock, E> {
    let decoded: EmbeddedResourceContent = decode_known(value)?;
    Ok(McpContentBlock::EmbeddedResource {
        resource: decoded.resource,
        annotations: decoded.common.annotations,
        meta: decoded.common.meta,
    })
}

fn decode_known<T, E>(value: Value) -> Result<T, E>
where
    T: for<'de> Deserialize<'de>,
    E: serde::de::Error,
{
    match serde_json::from_value(value) {
        Ok(decoded) => Ok(decoded),
        Err(source) => Err(E::custom(source)),
    }
}
