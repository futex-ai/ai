//! Private typed Gemini parts and versioned continuation payloads.

use ai_interface::{ModelError, ModelResult, ProviderState};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::PROVIDER_ID;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GoogleAssistantPart {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) thought: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) function_call: Option<GoogleFunctionCallContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) thought_signature: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct GoogleFunctionCallContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    pub(super) name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) args: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct GooglePartState {
    #[serde(default)]
    pub(super) scope: GoogleStateScope,
    pub(super) parts: Vec<GoogleAssistantPart>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) unsigned_call: Option<GoogleFunctionCallContext>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum GoogleStateScope {
    #[default]
    Message,
    Part,
}

impl GooglePartState {
    pub(super) fn new(parts: Vec<GoogleAssistantPart>) -> Self {
        Self {
            scope: GoogleStateScope::Part,
            parts,
            unsigned_call: None,
        }
    }

    pub(super) fn has_signature(&self) -> bool {
        self.parts
            .iter()
            .any(|part| part.thought_signature.is_some())
    }

    pub(super) fn encode(&self, model: &str) -> Result<ProviderState, serde_json::Error> {
        let value = serde_json::to_value(self)?;
        Ok(ProviderState {
            provider: PROVIDER_ID,
            model: model.to_owned(),
            version: 1,
            payload: match value {
                Value::Object(payload) => payload,
                _ => Default::default(),
            },
        })
    }

    pub(super) fn decode(state: &ProviderState, model: &str) -> ModelResult<Option<Self>> {
        if state.provider != PROVIDER_ID {
            return Ok(None);
        }
        if state.version != 1 {
            return Err(ModelError::provider(
                "google",
                model,
                format!(
                    "Google provider state has unsupported version {}",
                    state.version
                ),
            ));
        }
        match serde_json::from_value(Value::Object(state.payload.clone())) {
            Ok(payload) => Ok(Some(payload)),
            Err(_) => Err(ModelError::provider(
                "google",
                model,
                "Google provider state has an invalid payload",
            )),
        }
    }
}
