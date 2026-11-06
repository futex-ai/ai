//! Private version one payloads for stateless Responses replay.

use ai_interface::ProviderState;
use serde::{Deserialize, Serialize, ser::Error};
use serde_json::Value;

use crate::PROVIDER_ID;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub(super) enum ReplayState {
    FunctionCall(FunctionCallState),
    Reasoning(ReasoningState),
    Message(MessageState),
    Skip(SkipState),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SkipState {
    pub(super) replay: SkipReplay,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum SkipReplay {
    Skip,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MessageState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) phase: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReasoningState {
    pub(super) id: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(super) summary: Vec<ReasoningSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) encrypted_content: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FunctionCallState {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    pub(super) call_id: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct ReasoningSummary {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) text: String,
}

impl ReplayState {
    pub(super) fn encode(&self, model_id: &str) -> serde_json::Result<ProviderState> {
        let payload = match serde_json::to_value(self) {
            Ok(Value::Object(payload)) => payload,
            Ok(_) => {
                return Err(serde_json::Error::custom("invalid provider state payload"));
            }
            Err(source) => return Err(source),
        };
        Ok(ProviderState {
            provider: PROVIDER_ID,
            model: model_id.to_owned(),
            version: 1,
            payload,
        })
    }
}
