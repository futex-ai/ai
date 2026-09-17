//! Judgment response option and level membership tests.

use std::collections::BTreeMap;

use crate::{JudgmentAnswer, JudgmentAnswerProblem, JudgmentQuestion};

use super::response_validation_tests::{assert_problem, request, response};

#[test]
fn validation_rejects_an_unknown_probability_option() {
    assert_choice_problem(
        "billing",
        BTreeMap::from([
            ("billing".to_owned(), 0.5),
            ("other".to_owned(), 0.0),
            ("technical".to_owned(), 0.5),
        ]),
        JudgmentAnswerProblem::UnknownOption {
            label: "other".to_owned(),
        },
    );
}

#[test]
fn validation_rejects_a_missing_probability_option() {
    assert_choice_problem(
        "billing",
        BTreeMap::from([("billing".to_owned(), 1.0)]),
        JudgmentAnswerProblem::MissingOption {
            label: "technical".to_owned(),
        },
    );
}

#[test]
fn validation_rejects_an_unknown_selected_option() {
    assert_choice_problem(
        "other",
        BTreeMap::from([("billing".to_owned(), 0.5), ("technical".to_owned(), 0.5)]),
        JudgmentAnswerProblem::UnknownOption {
            label: "other".to_owned(),
        },
    );
}

#[test]
fn validation_rejects_an_unknown_probability_level() {
    let request = request(JudgmentQuestion::score("score", [None, None]));
    let response = response(JudgmentAnswer::Score {
        expected: 0.5,
        probabilities: BTreeMap::from([(0, 0.5), (1, 0.5), (2, 0.0)]),
        confidence: 1.0,
    });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::UnknownLevel { index: 2 },
    );
}

#[test]
fn validation_rejects_a_missing_probability_level() {
    let request = request(JudgmentQuestion::score("score", [None, None]));
    let response = response(JudgmentAnswer::Score {
        expected: 0.0,
        probabilities: BTreeMap::from([(0, 1.0)]),
        confidence: 1.0,
    });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::MissingLevel { index: 1 },
    );
}

#[test]
fn choice_membership_checks_unknown_probabilities_before_missing_options() {
    assert_choice_problem(
        "billing",
        BTreeMap::from([("billing".to_owned(), 1.0), ("other".to_owned(), 0.0)]),
        JudgmentAnswerProblem::UnknownOption {
            label: "other".to_owned(),
        },
    );
}

fn assert_choice_problem(
    selected: &str,
    probabilities: BTreeMap<String, f64>,
    expected: JudgmentAnswerProblem,
) {
    let request = request(JudgmentQuestion::choice(
        "choice",
        [("billing", None), ("technical", None)],
    ));
    let response = response(JudgmentAnswer::Choice {
        selected: selected.to_owned(),
        probabilities,
        confidence: 1.0,
    });

    assert_problem(response, &request, "question", expected);
}
