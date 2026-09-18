//! TypeSafe judgment response mapping and validation tests.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswer, JudgmentAnswerProblem, JudgmentContent, JudgmentError, JudgmentQuestion,
    JudgmentQuestionKind, JudgmentRequest, JudgmentResponse, JudgmentResult,
};
use serde_json::{Value, json};

use super::parse_response;

#[test]
fn response_maps_answers_and_ignores_provider_only_data() {
    let response = parse_value(valid_body()).expect("valid TypeSafe response should parse");

    assert_eq!(response.provider, "typesafe");
    assert_eq!(response.model_id, "jev-latest");
    assert_eq!(response.resolved_model_id, "jev-1.13.0");
    assert_eq!(response.answers.len(), 3);
    assert_eq!(
        response.answers["condition"],
        JudgmentAnswer::Condition { probability: 0.92 }
    );
    assert_eq!(
        response.answers["choice"],
        JudgmentAnswer::Choice {
            selected: "billing".to_owned(),
            probabilities: BTreeMap::from([
                ("billing".to_owned(), 0.84),
                ("technical".to_owned(), 0.16),
            ]),
            confidence: 0.6,
        }
    );
    assert_eq!(
        response.answers["score"],
        JudgmentAnswer::Score {
            expected: 1.6,
            probabilities: BTreeMap::from([(0, 0.05), (1, 0.3), (2, 0.65)]),
            confidence: 0.78,
        }
    );
    assert!(!response.answers.contains_key("provider_extra"));
}

#[test]
fn malformed_payloads_are_terminal_provider_errors() {
    for body in [
        json!({"model": 7, "answers": []}),
        body_with_score_probabilities(json!({"zero": 1.0})),
        body_with_score_probabilities(json!({"00": 1.0})),
    ] {
        let error = parse_value(body).expect_err("malformed response should fail");
        assert!(matches!(
            error,
            JudgmentError::Provider { provider, model_id, message }
                if provider == "typesafe"
                    && model_id == "jev-latest"
                    && message.starts_with("malformed provider payload: ")
        ));
    }
}

#[test]
fn undecodable_unrequested_answer_fragments_are_ignored() {
    let raw = r#"{
        "model":"jev-1.13.0",
        "answers":{
            "condition":{"type":"noul","noul":0.92},
            "choice":{
                "type":"choice",
                "choice":"billing",
                "probabilities":{"billing":0.84,"technical":0.16},
                "confidence":0.6
            },
            "score":{
                "type":"score",
                "score":1.6,
                "probabilities":{"0":0.05,"1":0.3,"2":0.65},
                "confidence":0.78
            },
            "extra":{"type":"future"},
            "extra2":null
        }
    }"#;

    let response = parse_response("jev-latest", &request(), raw.as_bytes())
        .expect("unrequested answer fragments should not be decoded");

    assert_eq!(response.answers.len(), 3);
    assert!(!response.answers.contains_key("extra"));
    assert!(!response.answers.contains_key("extra2"));
}

#[test]
fn shared_validator_surfaces_every_answer_problem() {
    let mut body = valid_body();
    answer_map(&mut body).remove("condition");
    assert_problem(body, JudgmentAnswerProblem::Missing, "condition");

    let mut body = valid_body();
    body["answers"]["condition"] = json!({
        "type": "score",
        "score": 0.0,
        "probabilities": {"0": 1.0},
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

    let mut body = valid_body();
    body["answers"]["condition"]["noul"] = json!(1.1);
    assert_problem(
        body,
        JudgmentAnswerProblem::ProbabilityOutOfRange { value: 1.1 },
        "condition",
    );

    let mut body = valid_body();
    body["answers"]["choice"]["confidence"] = json!(1.1);
    assert_problem(
        body,
        JudgmentAnswerProblem::ConfidenceOutOfRange { value: 1.1 },
        "choice",
    );

    let mut body = valid_body();
    body["answers"]["choice"]["choice"] = json!("operations");
    assert_problem(
        body,
        JudgmentAnswerProblem::UnknownOption {
            label: "operations".to_owned(),
        },
        "choice",
    );

    let mut body = valid_body();
    probability_map(&mut body, "choice").insert("operations".to_owned(), json!(0.0));
    assert_problem(
        body,
        JudgmentAnswerProblem::UnknownOption {
            label: "operations".to_owned(),
        },
        "choice",
    );

    let mut body = valid_body();
    probability_map(&mut body, "choice").remove("technical");
    assert_problem(
        body,
        JudgmentAnswerProblem::MissingOption {
            label: "technical".to_owned(),
        },
        "choice",
    );

    let mut body = valid_body();
    probability_map(&mut body, "score").insert("3".to_owned(), json!(0.0));
    assert_problem(
        body,
        JudgmentAnswerProblem::UnknownLevel { index: 3 },
        "score",
    );

    let mut body = valid_body();
    probability_map(&mut body, "score").remove("2");
    assert_problem(
        body,
        JudgmentAnswerProblem::MissingLevel { index: 2 },
        "score",
    );

    let mut body = valid_body();
    body["answers"]["score"]["score"] = json!(3.0);
    assert_problem(
        body,
        JudgmentAnswerProblem::ExpectedOutOfRange { value: 3.0 },
        "score",
    );

    let mut body = valid_body();
    body["answers"]["choice"]["probabilities"] = json!({"billing": 0.4, "technical": 0.4});
    assert_problem(
        body,
        JudgmentAnswerProblem::DistributionSum { sum: 0.8 },
        "choice",
    );
}

fn request() -> JudgmentRequest {
    JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([
            (
                "choice".to_owned(),
                JudgmentQuestion::choice(
                    "Team?",
                    [
                        ("billing", Some(JudgmentContent::from("Payments"))),
                        ("technical", None),
                    ],
                ),
            ),
            (
                "condition".to_owned(),
                JudgmentQuestion::condition("Urgent?", None),
            ),
            (
                "score".to_owned(),
                JudgmentQuestion::score("Frustration?", [None, None, None]),
            ),
        ]),
    }
}

fn valid_body() -> Value {
    json!({
        "model": "jev-1.13.0",
        "answers": {
            "condition": {"type": "noul", "noul": 0.92, "ignored": true},
            "choice": {
                "type": "choice",
                "choice": "billing",
                "probabilities": {"billing": 0.84, "technical": 0.16},
                "confidence": 0.6,
                "ignored": "field"
            },
            "score": {
                "type": "score",
                "score": 1.6,
                "legend": {"0": "Calm", "1": "Frustrated", "2": "Angry"},
                "probabilities": {"0": 0.05, "1": 0.3, "2": 0.65},
                "confidence": 0.78
            },
            "provider_extra": {"type": "noul", "noul": 4.0}
        },
        "provider_metadata": {"ignored": true},
        "usage": {"input_tokens": 312, "output_tokens": 48}
    })
}

fn body_with_score_probabilities(probabilities: Value) -> Value {
    let mut body = valid_body();
    body["answers"]["score"]["probabilities"] = probabilities;
    body
}

fn answer_map(body: &mut Value) -> &mut serde_json::Map<String, Value> {
    body["answers"]
        .as_object_mut()
        .expect("answers should be an object")
}

fn probability_map<'a>(
    body: &'a mut Value,
    answer: &str,
) -> &'a mut serde_json::Map<String, Value> {
    body["answers"][answer]["probabilities"]
        .as_object_mut()
        .expect("probabilities should be an object")
}

fn assert_problem(body: Value, expected: JudgmentAnswerProblem, id: &str) {
    let result = parse_value(body);
    let error = result.expect_err("shared response validation should fail");
    let JudgmentError::InvalidAnswer {
        provider,
        model_id,
        id: actual_id,
        problem,
    } = error
    else {
        panic!("expected invalid answer, got {error}");
    };

    assert_eq!(provider, "typesafe");
    assert_eq!(model_id, "jev-latest");
    assert_eq!(actual_id, id);
    assert_eq!(problem, expected);
}

fn parse_value(body: Value) -> JudgmentResult<JudgmentResponse> {
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");
    parse_response("jev-latest", &request(), &body)
}
