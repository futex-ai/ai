//! OpenAI Decisions response mapping tests.

use std::collections::BTreeMap;

use ai_interface::{JudgmentAnswer, JudgmentError, JudgmentResponse, JudgmentResult};
use serde_json::{Value, json};

use super::super::test_support::{full_body, full_request};
use super::parse_response;

#[test]
fn response_maps_answers_and_ignores_provider_only_data() {
    let response = parse_value(full_body()).expect("valid Decisions response should parse");

    assert_eq!(response.provider, "openai");
    assert_eq!(response.model_id, "gpt-6-luna");
    assert_eq!(response.resolved_model_id, "gpt-6-luna");
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
    assert_eq!(response.usage.input_tokens, 312);
}

#[test]
fn answer_order_does_not_matter_because_answers_match_by_name() {
    let mut body = full_body();
    body["answers"]
        .as_array_mut()
        .expect("answers should be an array")
        .reverse();

    let response = parse_value(body).expect("reordered answers should parse");

    assert_eq!(
        response.answers["condition"],
        JudgmentAnswer::Condition { probability: 0.92 }
    );
    assert_eq!(response.answers.len(), 3);
}

#[test]
fn unnamed_unrequested_and_undecodable_fragments_are_ignored() {
    let raw = r#"{
        "model":"gpt-6-luna",
        "answers":[
            {"type":"predicate","name":"condition","probability":0.92},
            {"type":"choice","name":"choice","choice":"billing",
             "probabilities":[{"value":"billing","probability":0.84},
                              {"value":"technical","probability":0.16}],
             "confidence":0.6},
            {"type":"score","name":"score","score":1.6,
             "probabilities":[{"value":0,"probability":0.05},
                              {"value":1,"probability":0.3},
                              {"value":2,"probability":0.65}],
             "confidence":0.78},
            {"type":"predicate","name":null,"probability":4.0},
            {"type":"predicate","probability":4.0},
            {"type":"future","name":"extra"},
            {"type":"refusal","name":"unrequested"},
            {"name":7},
            null,
            "text"
        ]
    }"#;

    let response = parse_response("gpt-6-luna", &full_request(), raw.as_bytes())
        .expect("unrequested answer fragments should not be decoded");

    assert_eq!(response.answers.len(), 3);
    assert!(!response.answers.contains_key("extra"));
    assert!(!response.answers.contains_key("unrequested"));
}

#[test]
fn malformed_payloads_are_terminal_provider_errors() {
    let mut boolean_choice = full_body();
    boolean_choice["answers"][1]["choice"] = json!(true);
    let mut boolean_value = full_body();
    boolean_value["answers"][1]["probabilities"][0]["value"] = json!(true);
    let mut negative_index = full_body();
    negative_index["answers"][2]["probabilities"][0]["value"] = json!(-1);
    let mut fractional_index = full_body();
    fractional_index["answers"][2]["probabilities"][0]["value"] = json!(0.5);
    let mut unknown_type = full_body();
    unknown_type["answers"][0]["type"] = json!("boolean");
    let mut missing_probability = full_body();
    missing_probability["answers"][0]
        .as_object_mut()
        .expect("answer should be an object")
        .remove("probability");

    for (label, body) in [
        ("non-object model", json!({"model": 7, "answers": []})),
        (
            "answers object",
            json!({"model": "gpt-6-luna", "answers": {}}),
        ),
        ("boolean choice", boolean_choice),
        ("boolean value", boolean_value),
        ("negative index", negative_index),
        ("fractional index", fractional_index),
        ("unknown type", unknown_type),
        ("missing probability", missing_probability),
    ] {
        let error = parse_value(body).expect_err(label);
        assert!(
            matches!(
                &error,
                JudgmentError::Provider { provider, model_id, message }
                    if provider == "openai"
                        && model_id == "gpt-6-luna"
                        && message.starts_with("malformed provider payload: ")
            ),
            "{label}: {error}"
        );
    }
}

#[test]
fn duplicate_names_values_and_indexes_are_rejected_instead_of_last_wins() {
    let mut duplicate_name = full_body();
    let condition = duplicate_name["answers"][0].clone();
    duplicate_name["answers"]
        .as_array_mut()
        .expect("answers should be an array")
        .push(condition);
    let mut duplicate_value = full_body();
    duplicate_value["answers"][1]["probabilities"] = json!([
        {"value": "billing", "probability": 0.1},
        {"value": "billing", "probability": 0.74},
        {"value": "technical", "probability": 0.16}
    ]);
    let mut duplicate_index = full_body();
    duplicate_index["answers"][2]["probabilities"] = json!([
        {"value": 0, "probability": 0.0},
        {"value": 0, "probability": 0.05},
        {"value": 1, "probability": 0.3},
        {"value": 2, "probability": 0.65}
    ]);

    for (body, detail) in [
        (duplicate_name, "duplicate answer name `condition`"),
        (duplicate_value, "duplicate choice value `billing`"),
        (duplicate_index, "duplicate score level index `0`"),
    ] {
        let error = parse_value(body).expect_err("duplicates should fail");
        let JudgmentError::Provider { message, .. } = &error else {
            panic!("expected a provider error, got {error}");
        };
        assert!(
            message.starts_with("malformed provider payload: "),
            "{message}"
        );
        assert!(message.contains(detail), "{message}");
    }
}

fn parse_value(body: Value) -> JudgmentResult<JudgmentResponse> {
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");
    parse_response("gpt-6-luna", &full_request(), &body)
}
