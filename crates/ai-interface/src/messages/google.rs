//! Typed Gemini assistant parts retained unchanged for continuation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// One original Gemini model content part, including signature-only parts.
pub struct GoogleAssistantPart {
    /// Original text, including empty text on signed stream parts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Whether this part contains private thought text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought: Option<bool>,
    /// Original function call without runtime-synthesized identifiers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function_call: Option<GoogleFunctionCallContext>,
    /// Opaque thought signature, retained on its original part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Original Gemini function call before runtime normalization.
pub struct GoogleFunctionCallContext {
    /// Provider identifier, absent when the provider did not send one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Provider function name.
    pub name: String,
    /// Original arguments, absent when the provider did not send them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
}
