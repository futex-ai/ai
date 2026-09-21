//! Content accepted by judgment requests and questions.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{JudgmentError, JudgmentResult};

/// Text or structured JSON content evaluated by a judgment model.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum JudgmentContent {
    /// Plain text content.
    Text(String),
    /// JSON object content.
    Object(Map<String, Value>),
    /// JSON array content.
    Array(Vec<Value>),
}

impl From<&str> for JudgmentContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<String> for JudgmentContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl TryFrom<Value> for JudgmentContent {
    type Error = JudgmentError;

    fn try_from(value: Value) -> JudgmentResult<Self> {
        match value {
            Value::String(text) => Ok(Self::Text(text)),
            Value::Object(object) => Ok(Self::Object(object)),
            Value::Array(array) => Ok(Self::Array(array)),
            Value::Null => Err(JudgmentError::UnsupportedContent {
                json_type: JudgmentJsonType::Null,
            }),
            Value::Bool(_) => Err(JudgmentError::UnsupportedContent {
                json_type: JudgmentJsonType::Boolean,
            }),
            Value::Number(_) => Err(JudgmentError::UnsupportedContent {
                json_type: JudgmentJsonType::Number,
            }),
        }
    }
}

/// Unsupported scalar JSON kinds rejected by [`JudgmentContent`].
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
