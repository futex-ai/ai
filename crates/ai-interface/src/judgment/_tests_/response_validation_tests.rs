//! Judgment response postcondition tests and shared fixtures.

use std::collections::BTreeMap;

use crate::{
    JudgmentAnswer, JudgmentAnswerProblem, JudgmentContent, JudgmentError, JudgmentQuestion,
    JudgmentQuestionKind, JudgmentRequest, JudgmentResponse, ModelUsage,
};

#[test]
fn validation_reports_the_first_missing_answer_in_request_order() {
    let request = JudgmentRequest {
        state: JudgmentContent::from("state"),
        questions: BTreeMap::from([
            ("zeta".to_owned(), JudgmentQuestion::condition("zeta", None)),
            (
                "alpha".to_owned(),
                JudgmentQuestion::condition("alpha", None),
            ),
        ]),
    };

    assert_problem(
        response_with(BTreeMap::new()),
        &request,
        "alpha",
        JudgmentAnswerProblem::Missing,
    );
}

#[test]
fn validation_rejects_an_answer_kind_mismatch() {
    let request = request(JudgmentQuestion::condition("condition", None));
    let response = response(JudgmentAnswer::Score {
        expected: 0.0,
        probabilities: BTreeMap::from([(0, 1.0)]),
        confidence: 1.0,
    });

    assert_problem(
        response,
        &request,
        "question",
        JudgmentAnswerProblem::KindMismatch {
            expected: JudgmentQuestionKind::Condition,
            actual: JudgmentQuestionKind::Score,
        },
    );
}

#[test]
fn validation_accepts_complete_answers_for_all_question_kinds() {
    let request = JudgmentRequest {
        state: JudgmentContent::from("state"),
        questions: valid_questions(),
    };
    let response = response_with(BTreeMap::from([
        (
            "choice".to_owned(),
            JudgmentAnswer::Choice {
                selected: "billing".to_owned(),
                probabilities: BTreeMap::from([
                    ("billing".to_owned(), 0.75),
                    ("technical".to_owned(), 0.25),
                ]),
                confidence: 0.5,
            },
        ),
        (
            "condition".to_owned(),
            JudgmentAnswer::Condition { probability: 0.8 },
        ),
        (
            "score".to_owned(),
            JudgmentAnswer::Score {
                expected: 1.25,
                probabilities: BTreeMap::from([(0, 0.1), (1, 0.55), (2, 0.35)]),
                confidence: 0.6,
            },
        ),
    ]));

    response.validate_against(&request).unwrap();
}

#[test]
fn validation_ignores_answer_ids_not_present_in_the_request() {
    let request = request(JudgmentQuestion::condition("condition", None));
    let response = response_with(BTreeMap::from([
        (
            "question".to_owned(),
            JudgmentAnswer::Condition { probability: 1.0 },
        ),
        (
            "unexpected".to_owned(),
            JudgmentAnswer::Condition {
                probability: f64::NAN,
            },
        ),
    ]));

    response.validate_against(&request).unwrap();
}

pub(super) fn request(question: JudgmentQuestion) -> JudgmentRequest {
    JudgmentRequest {
        state: JudgmentContent::from("state"),
        questions: BTreeMap::from([("question".to_owned(), question)]),
    }
}

pub(super) fn response(answer: JudgmentAnswer) -> JudgmentResponse {
    response_with(BTreeMap::from([("question".to_owned(), answer)]))
}

pub(super) fn response_with(answers: BTreeMap<String, JudgmentAnswer>) -> JudgmentResponse {
    JudgmentResponse {
        provider: "fixture-provider".to_owned(),
        model_id: "fixture-model".to_owned(),
        resolved_model_id: "resolved-model".to_owned(),
        answers,
        usage: ModelUsage::default(),
    }
}

pub(super) fn assert_problem(
    response: JudgmentResponse,
    request: &JudgmentRequest,
    expected_id: &str,
    expected_problem: JudgmentAnswerProblem,
) {
    let error = response.validate_against(request).unwrap_err();
    let JudgmentError::InvalidAnswer {
        provider,
        model_id,
        id,
        problem,
    } = error
    else {
        panic!("expected invalid answer, got {error}");
    };

    assert_eq!(provider, "fixture-provider");
    assert_eq!(model_id, "fixture-model");
    assert_eq!(id, expected_id);
    assert_eq!(problem, expected_problem);
}

fn valid_questions() -> BTreeMap<String, JudgmentQuestion> {
    BTreeMap::from([
        (
            "choice".to_owned(),
            JudgmentQuestion::choice("choice", [("billing", None), ("technical", None)]),
        ),
        (
            "condition".to_owned(),
            JudgmentQuestion::condition("condition", None),
        ),
        (
            "score".to_owned(),
            JudgmentQuestion::score("score", [None, None, None]),
        ),
    ])
}
