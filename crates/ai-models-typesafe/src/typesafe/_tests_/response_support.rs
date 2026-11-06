//! Shared fixtures for TypeSafe response mapping tests.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{
    ErrorKind, JudgmentAnswerProblem, JudgmentContent, JudgmentError, JudgmentQuestion,
    JudgmentRequest, JudgmentResponse, JudgmentResult,
};
use ai_models_core::{ErrorContext, SystemClock};
use serde_json::{Value, json};

use super::parse_response;

pub(super) fn request() -> JudgmentRequest {
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
                JudgmentQuestion::score("Frustration?", ["calm", "frustrated", "angry"]),
            ),
        ]),
    }
}

pub(super) fn valid_body() -> Value {
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

pub(super) fn body_with_score_probabilities(probabilities: Value) -> Value {
    let mut body = valid_body();
    body["answers"]["score"]["probabilities"] = probabilities;
    body
}

pub(super) fn answer_map(body: &mut Value) -> &mut serde_json::Map<String, Value> {
    body["answers"]
        .as_object_mut()
        .expect("answers should be an object")
}

pub(super) fn probability_map<'a>(
    body: &'a mut Value,
    answer: &str,
) -> &'a mut serde_json::Map<String, Value> {
    body["answers"][answer]["probabilities"]
        .as_object_mut()
        .expect("probabilities should be an object")
}

pub(super) fn assert_problem(body: Value, expected: JudgmentAnswerProblem, id: &str) {
    let result = parse_value(body);
    let error = result.expect_err("shared response validation should fail");
    let JudgmentError::InvalidAnswer {
        core,
        id: actual_id,
        problem,
    } = error
    else {
        panic!("expected invalid answer, got {error}");
    };
    let (provider, model_id) = (&core.provider, &core.model_id);
    assert_eq!(core.kind, ErrorKind::InvalidOutput);

    assert_eq!(provider, "typesafe");
    assert_eq!(model_id, "jev-latest");
    assert_eq!(actual_id, id);
    assert_eq!(problem, expected);
}

pub(super) fn parse_value(body: Value) -> JudgmentResult<JudgmentResponse> {
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");
    parse_response(
        &ErrorContext::new("typesafe", "jev-latest", &[], Arc::new(SystemClock)),
        "jev-latest",
        &request(),
        &body,
    )
}
