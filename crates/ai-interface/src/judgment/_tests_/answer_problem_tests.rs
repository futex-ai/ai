//! Judgment answer problem display tests.

use crate::{JudgmentAnswerProblem, JudgmentQuestionKind};

#[test]
fn missing_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::Missing.to_string(),
        "answer is missing"
    );
}

#[test]
fn kind_mismatch_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::KindMismatch {
            expected: JudgmentQuestionKind::Condition,
            actual: JudgmentQuestionKind::Choice,
        }
        .to_string(),
        "answer kind mismatch: expected Condition, got Choice"
    );
}

#[test]
fn probability_out_of_range_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::ProbabilityOutOfRange { value: 1.5 }.to_string(),
        "probability must be finite and between zero and one, got 1.5"
    );
}

#[test]
fn confidence_out_of_range_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::ConfidenceOutOfRange { value: -0.1 }.to_string(),
        "confidence must be finite and between zero and one, got -0.1"
    );
}

#[test]
fn unknown_option_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::UnknownOption {
            label: "other".to_owned(),
        }
        .to_string(),
        "option `other` was not requested"
    );
}

#[test]
fn missing_option_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::MissingOption {
            label: "billing".to_owned(),
        }
        .to_string(),
        "probability for option `billing` is missing"
    );
}

#[test]
fn unknown_level_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::UnknownLevel { index: 3 }.to_string(),
        "level index `3` was not requested"
    );
}

#[test]
fn missing_level_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::MissingLevel { index: 2 }.to_string(),
        "probability for level index `2` is missing"
    );
}

#[test]
fn expected_out_of_range_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::ExpectedOutOfRange { value: 2.5 }.to_string(),
        "expected score must be finite and within the requested levels, got 2.5"
    );
}

#[test]
fn distribution_sum_problem_has_a_specific_message() {
    assert_eq!(
        JudgmentAnswerProblem::DistributionSum { sum: 0.8 }.to_string(),
        "probability distribution must sum to one, got 0.8"
    );
}
