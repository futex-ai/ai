//! Judgment response numeric postcondition tests.

use std::collections::BTreeMap;

use crate::{
    JudgmentAnswer, JudgmentAnswerProblem, JudgmentError, JudgmentQuestion,
    PROBABILITY_SUM_TOLERANCE,
};

use super::response_validation_tests::{assert_problem, request, response};

#[test]
fn validation_rejects_an_out_of_range_probability() {
    let request = request(JudgmentQuestion::condition("condition", None));
    let response = response(JudgmentAnswer::Condition { probability: 1.1 });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::ProbabilityOutOfRange { value: 1.1 },
    );
}

#[test]
fn validation_rejects_an_out_of_range_confidence() {
    let request = request(JudgmentQuestion::choice("choice", [("yes", None)]));
    let response = response(JudgmentAnswer::Choice {
        selected: "yes".to_owned(),
        probabilities: BTreeMap::from([("yes".to_owned(), 1.0)]),
        confidence: f64::INFINITY,
    });

    let error = response.validate_against(&request).unwrap_err();
    let JudgmentError::InvalidAnswer { problem, .. } = error else {
        panic!("expected invalid answer, got {error}");
    };
    let JudgmentAnswerProblem::ConfidenceOutOfRange { value } = problem else {
        panic!("expected confidence problem, got {problem:?}");
    };
    assert!(value.is_infinite());
}

#[test]
fn validation_rejects_an_out_of_range_expected_score() {
    let request = request(JudgmentQuestion::score("score", [None, None]));
    let response = response(JudgmentAnswer::Score {
        expected: 2.0,
        probabilities: BTreeMap::from([(0, 0.5), (1, 0.5)]),
        confidence: 1.0,
    });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::ExpectedOutOfRange { value: 2.0 },
    );
}

#[test]
fn validation_rejects_a_distribution_outside_the_sum_tolerance() {
    let request = request(JudgmentQuestion::choice(
        "choice",
        [("no", None), ("yes", None)],
    ));
    let response = response(JudgmentAnswer::Choice {
        selected: "yes".to_owned(),
        probabilities: BTreeMap::from([("no".to_owned(), 0.4), ("yes".to_owned(), 0.4)]),
        confidence: 1.0,
    });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::DistributionSum { sum: 0.8 },
    );
}

#[test]
fn validation_accepts_distribution_sums_at_both_tolerance_boundaries() {
    assert_eq!(PROBABILITY_SUM_TOLERANCE, 0.02);
    let request = request(JudgmentQuestion::choice(
        "choice",
        [("no", None), ("yes", None)],
    ));

    for probability in [0.49, 0.51] {
        response(JudgmentAnswer::Choice {
            selected: "yes".to_owned(),
            probabilities: BTreeMap::from([
                ("no".to_owned(), probability),
                ("yes".to_owned(), probability),
            ]),
            confidence: 1.0,
        })
        .validate_against(&request)
        .unwrap();
    }
}

#[test]
fn numeric_checks_report_ranges_then_sum_then_expected_then_confidence() {
    let request = request(JudgmentQuestion::score("score", [None, None]));
    let response = response(JudgmentAnswer::Score {
        expected: 2.0,
        probabilities: BTreeMap::from([(0, 0.4), (1, 0.4)]),
        confidence: 2.0,
    });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::DistributionSum { sum: 0.8 },
    );
}
