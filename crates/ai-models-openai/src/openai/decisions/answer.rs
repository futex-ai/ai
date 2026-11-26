//! Decisions answer variants and validation of boolean choice distributions.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswer, JudgmentQuestion, JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage,
};
use ai_models_core::ErrorContext;
use serde::Deserialize;
use serde_json::value::RawValue;

use crate::PROVIDER_ID;

use super::error::{decode, invalid_output};

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Answer {
    Predicate {
        probability: f64,
    },
    Choice {
        choice: ChoiceValue,
        probabilities: Vec<Probability<ChoiceValue>>,
        confidence: f64,
    },
    Score {
        score: f64,
        probabilities: Vec<Probability<u32>>,
        confidence: f64,
    },
    Refusal,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ChoiceValue {
    Label(String),
    Boolean(bool),
}

#[derive(Deserialize)]
struct Probability<T> {
    value: T,
    probability: f64,
}

pub(super) fn normalize_answer(
    context: &ErrorContext,
    model_id: &str,
    id: &str,
    question: &JudgmentQuestion,
    body: &RawValue,
) -> JudgmentResult<JudgmentAnswer> {
    let answer: Answer = decode(context, body.get().as_bytes())?;
    let expects_boolean = matches!(
        question,
        JudgmentQuestion::Condition {
            criteria: Some(_),
            ..
        }
    );
    Ok(match answer {
        Answer::Predicate { .. } if expects_boolean => {
            return Err(invalid_output(
                context,
                "expected a choice answer for a condition with criteria",
            ));
        }
        Answer::Predicate { probability } => JudgmentAnswer::Condition { probability },
        Answer::Choice {
            choice,
            probabilities,
            confidence,
        } => {
            let selected = choice.into_label(context, expects_boolean)?;
            let probabilities = distribution(context, probabilities, |value| {
                value.into_label(context, expects_boolean)
            })?;
            if expects_boolean {
                return boolean_condition(
                    context,
                    model_id,
                    id,
                    selected,
                    probabilities,
                    confidence,
                );
            }
            JudgmentAnswer::Choice {
                selected,
                probabilities,
                confidence,
            }
        }
        Answer::Score {
            score,
            probabilities,
            confidence,
        } => JudgmentAnswer::Score {
            expected: score,
            probabilities: distribution(context, probabilities, Ok)?,
            confidence,
        },
        Answer::Refusal => JudgmentAnswer::Refused,
    })
}

impl ChoiceValue {
    fn into_label(self, context: &ErrorContext, expects_boolean: bool) -> JudgmentResult<String> {
        match (self, expects_boolean) {
            (Self::Label(label), false) => Ok(label),
            (Self::Boolean(value), true) => Ok(value.to_string()),
            (Self::Label(_), true) => {
                Err(invalid_output(context, "expected a boolean choice value"))
            }
            (Self::Boolean(_), false) => {
                Err(invalid_output(context, "expected a string choice value"))
            }
        }
    }
}

fn distribution<T, K: Ord>(
    context: &ErrorContext,
    entries: Vec<Probability<T>>,
    key: impl Fn(T) -> JudgmentResult<K>,
) -> JudgmentResult<BTreeMap<K, f64>> {
    let mut probabilities = BTreeMap::new();
    for entry in entries {
        if probabilities
            .insert(key(entry.value)?, entry.probability)
            .is_some()
        {
            return Err(invalid_output(context, "duplicate distribution value"));
        }
    }
    Ok(probabilities)
}

/// Uses the shared choice checks before discarding the boolean distribution.
fn boolean_condition(
    context: &ErrorContext,
    model_id: &str,
    id: &str,
    selected: String,
    probabilities: BTreeMap<String, f64>,
    confidence: f64,
) -> JudgmentResult<JudgmentAnswer> {
    let yes = probabilities.get("true").copied();
    let response = JudgmentResponse {
        provider: PROVIDER_ID.to_string(),
        model_id: model_id.to_owned(),
        resolved_model_id: String::new(),
        answers: BTreeMap::from([(
            id.to_owned(),
            JudgmentAnswer::Choice {
                selected,
                probabilities,
                confidence,
            },
        )]),
        usage: ModelUsage::default(),
    };
    response.validate_against(&JudgmentRequest {
        input: "".into(),
        questions: BTreeMap::from([(
            id.to_owned(),
            JudgmentQuestion::choice("", [("true", None), ("false", None)]),
        )]),
    })?;
    match yes {
        Some(probability) => Ok(JudgmentAnswer::Condition { probability }),
        None => Err(invalid_output(context, "missing probability for true")),
    }
}
