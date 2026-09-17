//! Judgment request validation tests.

use std::collections::BTreeMap;

use serde_json::Map;

use crate::{
    JudgmentContent, JudgmentError, JudgmentQuestion, JudgmentQuestionProblem, JudgmentRequest,
};

#[test]
fn validation_rejects_every_empty_state_shape() {
    for state in [
        JudgmentContent::from(" \n\t"),
        JudgmentContent::Object(Map::new()),
        JudgmentContent::Array(Vec::new()),
    ] {
        let error = request(state, valid_questions()).validate().unwrap_err();
        assert!(matches!(error, JudgmentError::EmptyState));
    }
}

#[test]
fn validation_rejects_an_empty_question_map() {
    let error = request(JudgmentContent::from("state"), BTreeMap::new())
        .validate()
        .unwrap_err();

    assert!(matches!(error, JudgmentError::NoQuestions));
}

#[test]
fn validation_rejects_a_blank_question_map_key() {
    let questions = BTreeMap::from([(
        " \n".to_owned(),
        JudgmentQuestion::condition("Valid instructions", None),
    )]);

    assert_invalid_question(questions, " \n", JudgmentQuestionProblem::BlankId);
}

#[test]
fn validation_rejects_a_choice_without_options() {
    let questions = BTreeMap::from([(
        "team".to_owned(),
        JudgmentQuestion::choice(
            "Choose a team",
            std::iter::empty::<(&str, Option<JudgmentContent>)>(),
        ),
    )]);

    assert_invalid_question(questions, "team", JudgmentQuestionProblem::NoOptions);
}

#[test]
fn validation_rejects_a_blank_choice_option_label() {
    let questions = BTreeMap::from([(
        "team".to_owned(),
        JudgmentQuestion::choice("Choose a team", [(" \t", None)]),
    )]);

    assert_invalid_question(questions, "team", JudgmentQuestionProblem::BlankOptionLabel);
}

#[test]
fn validation_rejects_scores_with_fewer_than_two_levels() {
    for levels in [Vec::new(), vec![None]] {
        let questions = BTreeMap::from([(
            "frustration".to_owned(),
            JudgmentQuestion::score("Rate frustration", levels.clone()),
        )]);

        assert_invalid_question(
            questions,
            "frustration",
            JudgmentQuestionProblem::TooFewLevels {
                levels: levels.len(),
            },
        );
    }
}

#[test]
fn validation_accepts_every_valid_question_kind() {
    request(JudgmentContent::from("state"), valid_questions())
        .validate()
        .unwrap();
}

fn request(
    state: JudgmentContent,
    questions: BTreeMap<String, JudgmentQuestion>,
) -> JudgmentRequest {
    JudgmentRequest { state, questions }
}

fn valid_questions() -> BTreeMap<String, JudgmentQuestion> {
    BTreeMap::from([
        (
            "condition".to_owned(),
            JudgmentQuestion::condition("Is it valid?", None),
        ),
        (
            "choice".to_owned(),
            JudgmentQuestion::choice("Choose", [("yes", None)]),
        ),
        (
            "score".to_owned(),
            JudgmentQuestion::score("Score", [None, None]),
        ),
    ])
}

fn assert_invalid_question(
    questions: BTreeMap<String, JudgmentQuestion>,
    expected_id: &str,
    expected_problem: JudgmentQuestionProblem,
) {
    let error = request(JudgmentContent::from("state"), questions)
        .validate()
        .unwrap_err();
    assert!(matches!(
        error,
        JudgmentError::InvalidQuestion { id, problem }
            if id == expected_id && problem == expected_problem
    ));
}
