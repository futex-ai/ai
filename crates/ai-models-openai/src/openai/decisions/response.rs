//! OpenAI Decisions response mapping.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswer, JudgmentError, JudgmentRequest, JudgmentResponse, JudgmentResult,
};
use ai_models_core::malformed_judgment_payload;
use serde::Deserialize;
use serde_json::value::RawValue;
use thiserror::Error;

use super::usage::{DecisionsUsage, normalize_usage};

const PROVIDER: &str = "openai";

#[derive(Debug, Deserialize)]
struct DecisionsResponse {
    model: String,
    answers: Vec<Box<RawValue>>,
    #[serde(default)]
    usage: Option<DecisionsUsage>,
}

/// Name peeked from an answer fragment before the fragment is decoded.
#[derive(Debug, Deserialize)]
struct AnswerName {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum DecisionsAnswer {
    Predicate {
        probability: f64,
    },
    Choice {
        choice: String,
        probabilities: Vec<ChoiceProbability>,
        confidence: f64,
    },
    Score {
        score: f64,
        probabilities: Vec<ScoreProbability>,
        confidence: f64,
    },
    Refusal,
}

#[derive(Debug, Deserialize)]
struct ChoiceProbability {
    value: String,
    probability: f64,
}

#[derive(Debug, Deserialize)]
struct ScoreProbability {
    value: u32,
    probability: f64,
}

/// A decoded answer for one requested question.
enum DecodedAnswer {
    Answered(JudgmentAnswer),
    Refused,
}

/// Typed failures while decoding a successful Decisions body.
#[derive(Debug, Error)]
enum DecodeError {
    #[error("[ai_models_openai/decisions] {source}")]
    Json { source: serde_json::Error },
    #[error("[ai_models_openai/decisions] duplicate answer name `{name}`")]
    DuplicateAnswerName { name: String },
    #[error("[ai_models_openai/decisions] duplicate choice value `{value}`")]
    DuplicateChoiceValue { value: String },
    #[error("[ai_models_openai/decisions] duplicate score level index `{index}`")]
    DuplicateScoreIndex { index: u32 },
}

/// Maps a successful Decisions body into the shared response contract.
///
/// Requested answers are decoded first so a malformed payload is reported
/// before a refusal; a refusal is reported before the shared postconditions.
pub(super) fn parse_response(
    model_id: &str,
    request: &JudgmentRequest,
    body: &[u8],
) -> JudgmentResult<JudgmentResponse> {
    let response = match serde_json::from_slice::<DecisionsResponse>(body) {
        Ok(response) => response,
        Err(source) => {
            return Err(malformed_judgment_payload(
                PROVIDER,
                model_id,
                &DecodeError::Json { source },
            ));
        }
    };
    let decoded = match decode_requested_answers(request, &response.answers) {
        Ok(decoded) => decoded,
        Err(error) => return Err(malformed_judgment_payload(PROVIDER, model_id, &error)),
    };
    let mut answers = BTreeMap::new();
    for (id, answer) in decoded {
        match answer {
            DecodedAnswer::Answered(answer) => {
                answers.insert(id, answer);
            }
            DecodedAnswer::Refused => return Err(JudgmentError::refused(PROVIDER, model_id, id)),
        }
    }
    let response = JudgmentResponse {
        provider: PROVIDER.to_owned(),
        model_id: model_id.to_owned(),
        resolved_model_id: response.model,
        answers,
        usage: normalize_usage(response.usage),
    };
    response.validate_against(request)?;
    Ok(response)
}

/// Decodes only the fragments whose `name` is a requested question id.
///
/// Fragments without a readable string name, or with an unrequested name,
/// are ignored even when they would not decode as answers.
fn decode_requested_answers(
    request: &JudgmentRequest,
    fragments: &[Box<RawValue>],
) -> Result<BTreeMap<String, DecodedAnswer>, DecodeError> {
    let mut decoded = BTreeMap::new();
    for fragment in fragments {
        let Ok(AnswerName { name: Some(name) }) =
            serde_json::from_str::<AnswerName>(fragment.get())
        else {
            continue;
        };
        if !request.questions.contains_key(&name) {
            continue;
        }
        if decoded.contains_key(&name) {
            return Err(DecodeError::DuplicateAnswerName { name });
        }
        let answer = match serde_json::from_str::<DecisionsAnswer>(fragment.get()) {
            Ok(answer) => answer,
            Err(source) => return Err(DecodeError::Json { source }),
        };
        decoded.insert(name, answer.normalize()?);
    }
    Ok(decoded)
}

impl DecisionsAnswer {
    fn normalize(self) -> Result<DecodedAnswer, DecodeError> {
        let answer = match self {
            Self::Predicate { probability } => JudgmentAnswer::Condition { probability },
            Self::Choice {
                choice,
                probabilities,
                confidence,
            } => JudgmentAnswer::Choice {
                selected: choice,
                probabilities: choice_probabilities(probabilities)?,
                confidence,
            },
            Self::Score {
                score,
                probabilities,
                confidence,
            } => JudgmentAnswer::Score {
                expected: score,
                probabilities: score_probabilities(probabilities)?,
                confidence,
            },
            Self::Refusal => return Ok(DecodedAnswer::Refused),
        };
        Ok(DecodedAnswer::Answered(answer))
    }
}

fn choice_probabilities(
    entries: Vec<ChoiceProbability>,
) -> Result<BTreeMap<String, f64>, DecodeError> {
    let mut probabilities = BTreeMap::new();
    for entry in entries {
        if probabilities.contains_key(&entry.value) {
            return Err(DecodeError::DuplicateChoiceValue { value: entry.value });
        }
        probabilities.insert(entry.value, entry.probability);
    }
    Ok(probabilities)
}

fn score_probabilities(entries: Vec<ScoreProbability>) -> Result<BTreeMap<u32, f64>, DecodeError> {
    let mut probabilities = BTreeMap::new();
    for entry in entries {
        if probabilities.contains_key(&entry.value) {
            return Err(DecodeError::DuplicateScoreIndex { index: entry.value });
        }
        probabilities.insert(entry.value, entry.probability);
    }
    Ok(probabilities)
}

#[cfg(test)]
#[path = "_tests_/response_tests.rs"]
mod response_tests;

#[cfg(test)]
#[path = "_tests_/response_validation_tests.rs"]
mod response_validation_tests;

#[cfg(test)]
#[path = "_tests_/refusal_tests.rs"]
mod refusal_tests;
