//! Judgment response internal-consistency postcondition tests.

use std::collections::BTreeMap;

use crate::{
    EXPECTED_SCORE_TOLERANCE, JudgmentAnswer, JudgmentAnswerProblem, JudgmentQuestion,
    SELECTED_PROBABILITY_TOLERANCE,
};

use super::response_validation_tests::{assert_problem, request, response};

#[test]
fn validation_rejects_a_selected_option_that_is_not_the_most_probable() {
    let request = choice_request();
    let response = response(choice("billing", 0.1, 0.9, 1.0));

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::SelectedNotMaximal {
            selected: "billing".to_owned(),
            maximal: "technical".to_owned(),
        },
    );
}

#[test]
fn validation_accepts_ties_and_near_ties_for_the_selected_option() {
    assert_eq!(SELECTED_PROBABILITY_TOLERANCE, 0.02);
    let request = choice_request();

    for (billing, technical) in [(0.5, 0.5), (0.495, 0.505), (0.505, 0.495)] {
        response(choice("billing", billing, technical, 1.0))
            .validate_against(&request)
            .unwrap();
    }
}

#[test]
fn selected_consistency_is_checked_before_confidence() {
    let request = choice_request();
    let response = response(choice("billing", 0.1, 0.9, 2.0));

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::SelectedNotMaximal {
            selected: "billing".to_owned(),
            maximal: "technical".to_owned(),
        },
    );
}

#[test]
fn validation_rejects_an_expected_score_that_contradicts_its_distribution() {
    let request = score_request();
    let response = response(score(0.0, [0.0, 1.0], 1.0));

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::ExpectedInconsistent {
            expected: 0.0,
            weighted: 1.0,
        },
    );
}

#[test]
fn validation_accepts_expected_scores_within_rounding_of_the_weighted_level() {
    assert_eq!(EXPECTED_SCORE_TOLERANCE, 0.1);
    let request = score_request();

    for expected in [0.5, 0.45, 0.55] {
        response(score(expected, [0.5, 0.5], 1.0))
            .validate_against(&request)
            .unwrap();
    }
}

#[test]
fn expected_consistency_normalizes_residual_distribution_rounding() {
    let request = score_request();

    response(score(1.0, [0.0, 0.99], 1.0))
        .validate_against(&request)
        .unwrap();
}

#[test]
fn expected_consistency_is_checked_after_range_and_before_confidence() {
    let request = score_request();

    assert_problem(
        response(score(2.0, [0.0, 1.0], 2.0)),
        &request,
        "question",
        JudgmentAnswerProblem::ExpectedOutOfRange { value: 2.0 },
    );
    assert_problem(
        response(score(0.0, [0.0, 1.0], 2.0)),
        &request,
        "question",
        JudgmentAnswerProblem::ExpectedInconsistent {
            expected: 0.0,
            weighted: 1.0,
        },
    );
}

fn choice_request() -> crate::JudgmentRequest {
    request(JudgmentQuestion::choice(
        "choice",
        [("billing", None), ("technical", None)],
    ))
}

fn score_request() -> crate::JudgmentRequest {
    request(JudgmentQuestion::score("score", [None, None]))
}

fn choice(selected: &str, billing: f64, technical: f64, confidence: f64) -> JudgmentAnswer {
    JudgmentAnswer::Choice {
        selected: selected.to_owned(),
        probabilities: BTreeMap::from([
            ("billing".to_owned(), billing),
            ("technical".to_owned(), technical),
        ]),
        confidence,
    }
}

fn score(expected: f64, levels: [f64; 2], confidence: f64) -> JudgmentAnswer {
    JudgmentAnswer::Score {
        expected,
        probabilities: BTreeMap::from([(0, levels[0]), (1, levels[1])]),
        confidence,
    }
}
