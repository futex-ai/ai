//! TypeSafe judgment response mapping.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswer, JudgmentError, JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage,
    deserialize_score_probabilities,
};
use serde::Deserialize;
use serde_json::value::RawValue;

use super::redaction::redact_secrets;

const PROVIDER: &str = "typesafe";
const MALFORMED_PROVIDER_PAYLOAD: &str = "malformed provider payload";

#[derive(Debug, Deserialize)]
struct TypeSafeResponse {
    model: String,
    answers: BTreeMap<String, Box<RawValue>>,
    #[serde(default)]
    usage: Option<TypeSafeUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum TypeSafeAnswer {
    #[serde(rename = "noul")]
    Condition { noul: f64 },
    #[serde(rename = "choice")]
    Choice {
        choice: String,
        probabilities: BTreeMap<String, f64>,
        confidence: f64,
    },
    #[serde(rename = "score")]
    Score {
        score: f64,
        #[serde(deserialize_with = "deserialize_score_probabilities")]
        probabilities: BTreeMap<u32, f64>,
        confidence: f64,
    },
}

#[derive(Debug, Default, Deserialize)]
struct TypeSafeUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
}

/// Maps a successful System One body into the shared response contract.
pub(super) fn parse_response(
    model_id: &str,
    request: &JudgmentRequest,
    body: &[u8],
) -> JudgmentResult<JudgmentResponse> {
    let response = match serde_json::from_slice::<TypeSafeResponse>(body) {
        Ok(response) => response,
        Err(source) => return Err(malformed_payload(model_id, &source)),
    };
    let TypeSafeResponse {
        model,
        answers,
        usage,
    } = response;
    let mut provider_answers = answers;
    let mut answers = BTreeMap::new();
    for id in request.questions.keys() {
        let Some(fragment) = provider_answers.remove(id) else {
            continue;
        };
        let answer = match serde_json::from_str::<TypeSafeAnswer>(fragment.get()) {
            Ok(answer) => answer,
            Err(source) => return Err(malformed_payload(model_id, &source)),
        };
        answers.insert(id.clone(), answer.normalize());
    }
    let response = JudgmentResponse {
        provider: PROVIDER.to_owned(),
        model_id: model_id.to_owned(),
        resolved_model_id: model,
        answers,
        usage: normalize_usage(usage),
    };
    response.validate_against(request)?;
    Ok(response)
}

impl TypeSafeAnswer {
    fn normalize(self) -> JudgmentAnswer {
        match self {
            Self::Condition { noul } => JudgmentAnswer::Condition { probability: noul },
            Self::Choice {
                choice,
                probabilities,
                confidence,
            } => JudgmentAnswer::Choice {
                selected: choice,
                probabilities,
                confidence,
            },
            Self::Score {
                score,
                probabilities,
                confidence,
            } => JudgmentAnswer::Score {
                expected: score,
                probabilities,
                confidence,
            },
        }
    }
}

fn normalize_usage(usage: Option<TypeSafeUsage>) -> ModelUsage {
    let Some(usage) = usage else {
        return ModelUsage::default();
    };
    ModelUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage.input_tokens.saturating_add(usage.output_tokens),
        ..ModelUsage::default()
    }
}

fn malformed_payload(model_id: &str, source: &serde_json::Error) -> JudgmentError {
    JudgmentError::provider(PROVIDER, model_id, malformed_payload_message(source))
}

fn malformed_payload_message(source: &serde_json::Error) -> String {
    let detail = source.to_string();
    [MALFORMED_PROVIDER_PAYLOAD, ": ", &detail].concat()
}

/// Redacts provider diagnostics produced during successful-body decoding.
pub(super) fn redact_response_error(error: JudgmentError, secrets: &[String]) -> JudgmentError {
    match error {
        JudgmentError::Provider {
            provider,
            model_id,
            message,
        } => JudgmentError::provider(provider, model_id, redact_secrets(&message, secrets)),
        error => error,
    }
}

#[cfg(test)]
#[path = "_tests_/response_tests.rs"]
mod response_tests;

#[cfg(test)]
#[path = "_tests_/usage_tests.rs"]
mod usage_tests;
