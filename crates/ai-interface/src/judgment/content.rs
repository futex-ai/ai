//! Content accepted by judgment requests and questions.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{JudgmentError, JudgmentResult};

/// An ordered list of parts evaluated by a judgment model.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(transparent)]
pub struct JudgmentContent {
    /// The content parts in caller order.
    pub parts: Vec<JudgmentPart>,
}

impl JudgmentContent {
    pub(super) fn is_empty(&self) -> bool {
        self.parts.iter().all(|part| match part {
            JudgmentPart::Text { text } => text.trim().is_empty(),
            JudgmentPart::Json {
                value: JudgmentJson::Object(object),
            } => object.is_empty(),
            JudgmentPart::Json {
                value: JudgmentJson::Array(array),
            } => array.is_empty(),
            JudgmentPart::Image { .. } => false,
        })
    }
}

/// One part of judgment input or instructions.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JudgmentPart {
    /// Plain text.
    Text {
        /// Text supplied by the caller.
        text: String,
    },
    /// One structured JSON value.
    Json {
        /// The JSON object or array.
        value: JudgmentJson,
    },
    /// One inline image.
    Image {
        /// MIME type of the image.
        mime_type: String,
        /// Base64-encoded image bytes.
        data_base64: String,
    },
}

/// A JSON object or array in a judgment part or description.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JudgmentJson {
    /// JSON object content.
    Object(Map<String, Value>),
    /// JSON array content.
    Array(Vec<Value>),
}

/// Text or structured JSON that describes a condition outcome, option, or level.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JudgmentDescription {
    /// Plain text description.
    Text(String),
    /// JSON object or array description.
    Json(JudgmentJson),
}

impl From<&str> for JudgmentContent {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<String> for JudgmentContent {
    fn from(value: String) -> Self {
        Self {
            parts: vec![JudgmentPart::Text { text: value }],
        }
    }
}

impl TryFrom<Value> for JudgmentContent {
    type Error = JudgmentError;

    fn try_from(value: Value) -> JudgmentResult<Self> {
        let part = match JudgmentDescription::try_from(value)? {
            JudgmentDescription::Text(text) => JudgmentPart::Text { text },
            JudgmentDescription::Json(value) => JudgmentPart::Json { value },
        };
        Ok(Self { parts: vec![part] })
    }
}

impl From<&str> for JudgmentDescription {
    fn from(value: &str) -> Self {
        Self::from(value.to_owned())
    }
}

impl From<String> for JudgmentDescription {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl TryFrom<Value> for JudgmentDescription {
    type Error = JudgmentError;

    fn try_from(value: Value) -> JudgmentResult<Self> {
        match value {
            Value::String(text) => Ok(Self::Text(text)),
            Value::Object(object) => Ok(Self::Json(JudgmentJson::Object(object))),
            Value::Array(array) => Ok(Self::Json(JudgmentJson::Array(array))),
            Value::Null => Err(JudgmentError::unsupported_content(JudgmentJsonType::Null)),
            Value::Bool(_) => Err(JudgmentError::unsupported_content(
                JudgmentJsonType::Boolean,
            )),
            Value::Number(_) => Err(JudgmentError::unsupported_content(JudgmentJsonType::Number)),
        }
    }
}

/// Scalar JSON kinds rejected by judgment content and description conversions.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum JudgmentJsonType {
    /// JSON null.
    Null,
    /// JSON boolean.
    Boolean,
    /// JSON number.
    Number,
}

#[cfg(test)]
#[path = "_tests_/content_tests.rs"]
mod content_tests;
