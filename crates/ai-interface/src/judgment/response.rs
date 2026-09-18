//! Normalized judgment response DTO.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ModelUsage;

use super::{
    JudgmentAnswer, JudgmentAnswerProblem, JudgmentContent, JudgmentError, JudgmentQuestion,
    JudgmentRequest, JudgmentResult,
};

/// Maximum absolute deviation from one accepted for a probability distribution sum.
pub const PROBABILITY_SUM_TOLERANCE: f64 = 0.02;

/// Maximum amount by which the selected option's probability may fall below
/// the highest option probability before the selection is inconsistent.
///
/// Ties and provider rounding of reported probabilities stay acceptable.
pub const SELECTED_PROBABILITY_TOLERANCE: f64 = 0.02;

/// Maximum absolute difference accepted between a reported expected score and
/// the probability-weighted level computed from its reported distribution.
///
/// The tolerance is a tenth of one level step, which absorbs provider rounding
/// of both the score and the distribution.
pub const EXPECTED_SCORE_TOLERANCE: f64 = 0.1;

/// Normalized response from one judgment call.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct JudgmentResponse {
    /// Provider that evaluated the request.
    pub provider: String,
    /// Configured provider model identifier.
    pub model_id: String,
    /// Provider-reported model identifier used for the call.
    pub resolved_model_id: String,
    /// Typed answers keyed by caller-provided question ids.
    pub answers: BTreeMap<String, JudgmentAnswer>,
    /// Provider-reported normalized token usage.
    pub usage: ModelUsage,
}

impl JudgmentResponse {
    /// Pure postcondition check of this response against the request that produced it.
    pub fn validate_against(&self, request: &JudgmentRequest) -> JudgmentResult<()> {
        for (id, question) in &request.questions {
            let problem = match self.answers.get(id) {
                Some(answer) => answer_problem(question, answer),
                None => Some(JudgmentAnswerProblem::Missing),
            };
            if let Some(problem) = problem {
                return Err(JudgmentError::invalid_answer(
                    self.provider.as_str(),
                    self.model_id.as_str(),
                    id.as_str(),
                    problem,
                ));
            }
        }
        Ok(())
    }
}

fn answer_problem(
    question: &JudgmentQuestion,
    answer: &JudgmentAnswer,
) -> Option<JudgmentAnswerProblem> {
    match (question, answer) {
        (JudgmentQuestion::Condition { .. }, JudgmentAnswer::Condition { probability }) => {
            probability_problem(*probability)
        }
        (
            JudgmentQuestion::Choice { options, .. },
            JudgmentAnswer::Choice {
                selected,
                probabilities,
                confidence,
            },
        ) => choice_problem(options, selected, probabilities, *confidence),
        (
            JudgmentQuestion::Score { levels, .. },
            JudgmentAnswer::Score {
                expected,
                probabilities,
                confidence,
            },
        ) => score_problem(levels, *expected, probabilities, *confidence),
        _ => Some(JudgmentAnswerProblem::KindMismatch {
            expected: question.kind(),
            actual: answer.kind(),
        }),
    }
}

fn choice_problem(
    options: &BTreeMap<String, Option<JudgmentContent>>,
    selected: &str,
    probabilities: &BTreeMap<String, f64>,
    confidence: f64,
) -> Option<JudgmentAnswerProblem> {
    for label in probabilities.keys() {
        if !options.contains_key(label) {
            return Some(JudgmentAnswerProblem::UnknownOption {
                label: label.clone(),
            });
        }
    }
    for label in options.keys() {
        if !probabilities.contains_key(label) {
            return Some(JudgmentAnswerProblem::MissingOption {
                label: label.clone(),
            });
        }
    }
    if !options.contains_key(selected) {
        return Some(JudgmentAnswerProblem::UnknownOption {
            label: selected.to_owned(),
        });
    }
    for probability in probabilities.values() {
        if let Some(problem) = probability_problem(*probability) {
            return Some(problem);
        }
    }
    let sum = probabilities.values().sum::<f64>();
    if let Some(problem) = distribution_sum_problem(sum) {
        return Some(problem);
    }
    if let Some(problem) = selected_problem(selected, probabilities) {
        return Some(problem);
    }
    confidence_problem(confidence)
}

fn selected_problem(
    selected: &str,
    probabilities: &BTreeMap<String, f64>,
) -> Option<JudgmentAnswerProblem> {
    let selected_probability = probabilities.get(selected).copied()?;
    let (maximal, maximal_probability) = probabilities
        .iter()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))?;
    if selected_probability + SELECTED_PROBABILITY_TOLERANCE >= *maximal_probability {
        return None;
    }
    Some(JudgmentAnswerProblem::SelectedNotMaximal {
        selected: selected.to_owned(),
        maximal: maximal.clone(),
    })
}

fn score_problem(
    levels: &[Option<JudgmentContent>],
    expected: f64,
    probabilities: &BTreeMap<u32, f64>,
    confidence: f64,
) -> Option<JudgmentAnswerProblem> {
    for index in probabilities.keys() {
        if *index as usize >= levels.len() {
            return Some(JudgmentAnswerProblem::UnknownLevel { index: *index });
        }
    }
    for index in 0..levels.len() {
        let index = index as u32;
        if !probabilities.contains_key(&index) {
            return Some(JudgmentAnswerProblem::MissingLevel { index });
        }
    }
    for probability in probabilities.values() {
        if let Some(problem) = probability_problem(*probability) {
            return Some(problem);
        }
    }
    let sum = probabilities.values().sum::<f64>();
    if let Some(problem) = distribution_sum_problem(sum) {
        return Some(problem);
    }
    let maximum = levels.len().saturating_sub(1) as f64;
    if value_out_of_range(expected, maximum) {
        return Some(JudgmentAnswerProblem::ExpectedOutOfRange { value: expected });
    }
    let weighted = weighted_level(probabilities, sum);
    if (expected - weighted).abs() > EXPECTED_SCORE_TOLERANCE {
        return Some(JudgmentAnswerProblem::ExpectedInconsistent { expected, weighted });
    }
    confidence_problem(confidence)
}

/// Computes the expected level of a distribution whose sum has already been
/// checked to be close to one, normalizing away residual rounding.
fn weighted_level(probabilities: &BTreeMap<u32, f64>, sum: f64) -> f64 {
    probabilities
        .iter()
        .map(|(index, probability)| f64::from(*index) * probability)
        .sum::<f64>()
        / sum
}

fn probability_problem(value: f64) -> Option<JudgmentAnswerProblem> {
    value_out_of_range(value, 1.0).then_some(JudgmentAnswerProblem::ProbabilityOutOfRange { value })
}

fn confidence_problem(value: f64) -> Option<JudgmentAnswerProblem> {
    value_out_of_range(value, 1.0).then_some(JudgmentAnswerProblem::ConfidenceOutOfRange { value })
}

fn distribution_sum_problem(sum: f64) -> Option<JudgmentAnswerProblem> {
    let accepted =
        (1.0 - PROBABILITY_SUM_TOLERANCE..=1.0 + PROBABILITY_SUM_TOLERANCE).contains(&sum);
    (!accepted).then_some(JudgmentAnswerProblem::DistributionSum { sum })
}

fn value_out_of_range(value: f64, maximum: f64) -> bool {
    !value.is_finite() || !(0.0..=maximum).contains(&value)
}

#[cfg(test)]
#[path = "_tests_/response_tests.rs"]
mod response_tests;

#[cfg(test)]
#[path = "_tests_/response_validation_tests.rs"]
mod response_validation_tests;

#[cfg(test)]
#[path = "_tests_/response_membership_tests.rs"]
mod response_membership_tests;

#[cfg(test)]
#[path = "_tests_/response_range_tests.rs"]
mod response_range_tests;

#[cfg(test)]
#[path = "_tests_/response_consistency_tests.rs"]
mod response_consistency_tests;
