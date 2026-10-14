//! Shared postcondition propagation tests for OpenAI Decisions responses.

use ai_interface::{
    JudgmentAnswerProblem, JudgmentError, JudgmentQuestionKind, JudgmentResponse, JudgmentResult,
};
use serde_json::{Value, json};

use super::super::test_support::{full_body, full_request};
use super::parse_response;

#[test]
fn shared_validator_surfaces_every_answer_problem() {
    let mut body = full_body();
    body["answers"]
        .as_array_mut()
        .expect("answers should be an array")
        .remove(0);
    assert_problem(body, JudgmentAnswerProblem::Missing, "condition");

    let mut body = full_body();
    body["answers"][0] = json!({
        "type": "score",
        "name": "condition",
        "score": 0.0,
        "probabilities": [{"value": 0, "probability": 1.0}],
        "confidence": 1.0
    });
    assert_problem(
        body,
        JudgmentAnswerProblem::KindMismatch {
            expected: JudgmentQuestionKind::Condition,
            actual: JudgmentQuestionKind::Score,
        },
        "condition",
    );

    let mut body = full_body();
    body["answers"][0]["probability"] = json!(1.1);
    assert_problem(
        body,
        JudgmentAnswerProblem::ProbabilityOutOfRange { value: 1.1 },
        "condition",
    );

    let mut body = full_body();
    body["answers"][1]["confidence"] = json!(1.1);
    assert_problem(
        body,
        JudgmentAnswerProblem::ConfidenceOutOfRange { value: 1.1 },
        "choice",
    );

    let mut body = full_body();
    body["answers"][1]["choice"] = json!("operations");
    assert_problem(
        body,
        JudgmentAnswerProblem::UnknownOption {
            label: "operations".to_owned(),
        },
        "choice",
    );

    let mut body = full_body();
    body["answers"][1]["probabilities"] = json!([
        {"value": "billing", "probability": 0.84},
        {"value": "technical", "probability": 0.16},
        {"value": "operations", "probability": 0.0}
    ]);
    assert_problem(
        body,
        JudgmentAnswerProblem::UnknownOption {
            label: "operations".to_owned(),
        },
        "choice",
    );

    let mut body = full_body();
    body["answers"][1]["probabilities"] = json!([{"value": "billing", "probability": 1.0}]);
    assert_problem(
        body,
        JudgmentAnswerProblem::MissingOption {
            label: "technical".to_owned(),
        },
        "choice",
    );

    let mut body = full_body();
    body["answers"][2]["probabilities"]
        .as_array_mut()
        .expect("probabilities should be an array")
        .push(json!({"value": 3, "probability": 0.0}));
    assert_problem(
        body,
        JudgmentAnswerProblem::UnknownLevel { index: 3 },
        "score",
    );

    let mut body = full_body();
    body["answers"][2]["probabilities"]
        .as_array_mut()
        .expect("probabilities should be an array")
        .pop();
    assert_problem(
        body,
        JudgmentAnswerProblem::MissingLevel { index: 2 },
        "score",
    );

    let mut body = full_body();
    body["answers"][2]["score"] = json!(3.0);
    assert_problem(
        body,
        JudgmentAnswerProblem::ExpectedOutOfRange { value: 3.0 },
        "score",
    );

    let mut body = full_body();
    body["answers"][1]["probabilities"] = json!([
        {"value": "billing", "probability": 0.4},
        {"value": "technical", "probability": 0.4}
    ]);
    assert_problem(
        body,
        JudgmentAnswerProblem::DistributionSum { sum: 0.8 },
        "choice",
    );

    let mut body = full_body();
    body["answers"][1]["choice"] = json!("technical");
    assert_problem(
        body,
        JudgmentAnswerProblem::SelectedNotMaximal {
            selected: "technical".to_owned(),
            maximal: "billing".to_owned(),
        },
        "choice",
    );

    let mut body = full_body();
    body["answers"][2]["score"] = json!(0.2);
    let error = parse_value(body).expect_err("inconsistent expectation should fail");
    assert!(matches!(
        error,
        JudgmentError::InvalidAnswer {
            problem: JudgmentAnswerProblem::ExpectedInconsistent { .. },
            ..
        }
    ));
}

fn assert_problem(body: Value, expected: JudgmentAnswerProblem, id: &str) {
    let error = parse_value(body).expect_err("shared response validation should fail");
    let JudgmentError::InvalidAnswer {
        provider,
        model_id,
        id: actual_id,
        problem,
    } = error
    else {
        panic!("expected invalid answer, got {error}");
    };

    assert_eq!(provider, "openai");
    assert_eq!(model_id, "gpt-6-luna");
    assert_eq!(actual_id, id);
    assert_eq!(problem, expected);
}

fn parse_value(body: Value) -> JudgmentResult<JudgmentResponse> {
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");
    parse_response("gpt-6-luna", &full_request(), &body)
}
