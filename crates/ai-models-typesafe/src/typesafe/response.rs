//! TypeSafe judgment response mapping.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswer, JudgmentError, JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage,
    deserialize_score_probabilities,
};
use serde::Deserialize;
use serde_json::Value;

const PROVIDER: &str = "typesafe";
const MALFORMED_PROVIDER_PAYLOAD: &str = "malformed provider payload";

#[derive(Debug, Deserialize)]
struct TypeSafeResponse {
    model: String,
    answers: BTreeMap<String, TypeSafeAnswer>,
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
    body: Value,
) -> JudgmentResult<JudgmentResponse> {
    let response = match serde_json::from_value::<TypeSafeResponse>(body) {
        Ok(response) => response,
        Err(_) => return Err(malformed_payload(model_id)),
    };
    let TypeSafeResponse {
        model,
        answers,
        usage,
    } = response;
    let mut provider_answers = answers;
    let answers = request
        .questions
        .keys()
        .filter_map(|id| {
            provider_answers
                .remove(id)
                .map(|answer| (id.clone(), answer.normalize()))
        })
        .collect();
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

fn malformed_payload(model_id: &str) -> JudgmentError {
    JudgmentError::provider(PROVIDER, model_id, MALFORMED_PROVIDER_PAYLOAD)
}

#[cfg(test)]
#[path = "_tests_/response_tests.rs"]
mod response_tests;

#[cfg(test)]
#[path = "_tests_/usage_tests.rs"]
mod usage_tests;
