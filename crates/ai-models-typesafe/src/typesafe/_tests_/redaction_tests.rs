//! TypeSafe judgment credential-redaction tests.

use std::collections::BTreeMap;

use ai_interface::{
    JudgmentAnswerProblem, JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentRequest,
};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    TypeSafeJudgmentModel,
    test_support::{
        json_response, recording_http_client, simple_request, transport_failure_http_client,
    },
};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn bearer_key_is_redacted_from_transport_diagnostics() {
    let error = TypeSafeJudgmentModel::new(
        transport_failure_http_client("connection failed for Bearer typesafe-secret-key"),
        "jev-latest",
        API_KEY,
    )
    .judge(&simple_request())
    .await
    .expect_err("transport failure should return an error");

    assert!(matches!(error, JudgmentError::TransientProvider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn bearer_key_is_redacted_from_provider_bodies() {
    let (http_client, _) = recording_http_client(json_response(
        400,
        json!({"detail": {"message": "rejected Bearer typesafe-secret-key"}}),
    ));

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("provider rejection should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn malformed_payload_diagnostics_are_redacted() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: br#"{
            "model":"jev-1.13.0",
            "answers":{"is_urgent":{"type":"typesafe-secret-key"}}
        }"#
        .to_vec(),
    });

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("malformed response should return an error");

    assert!(matches!(error, JudgmentError::Provider { .. }));
    assert_redacted(&error, API_KEY);
}

#[tokio::test]
async fn unknown_selected_option_labels_are_redacted() {
    let response = json_response(
        200,
        json!({
            "model": "jev-1.13.0",
            "answers": {
                "team": {
                    "type": "choice",
                    "choice": "typesafe-secret-key",
                    "probabilities": {"billing": 0.75, "technical": 0.25},
                    "confidence": 0.8
                }
            }
        }),
    );
    let (http_client, _) = recording_http_client(response);

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&choice_request())
        .await
        .expect_err("unknown selected option should return an error");

    assert_unknown_option_redacted(&error);
}

#[tokio::test]
async fn unknown_probability_option_labels_are_redacted() {
    let response = json_response(
        200,
        json!({
            "model": "jev-1.13.0",
            "answers": {
                "team": {
                    "type": "choice",
                    "choice": "billing",
                    "probabilities": {
                        "billing": 0.75,
                        "technical": 0.25,
                        "typesafe-secret-key": 0.0
                    },
                    "confidence": 0.8
                }
            }
        }),
    );
    let (http_client, _) = recording_http_client(response);

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&choice_request())
        .await
        .expect_err("unknown probability option should return an error");

    assert_unknown_option_redacted(&error);
}

fn choice_request() -> JudgmentRequest {
    JudgmentRequest {
        state: "A support ticket".into(),
        questions: BTreeMap::from([(
            "team".to_owned(),
            JudgmentQuestion::choice("Choose a team", [("billing", None), ("technical", None)]),
        )]),
    }
}

fn assert_unknown_option_redacted(error: &JudgmentError) {
    let JudgmentError::InvalidAnswer {
        provider,
        model_id,
        id,
        problem: JudgmentAnswerProblem::UnknownOption { label },
    } = error
    else {
        panic!("expected an unknown-option answer error, got {error}");
    };

    assert_eq!(provider, "typesafe");
    assert_eq!(model_id, "jev-latest");
    assert_eq!(id, "team");
    assert_eq!(label, "[redacted]");
    assert_redacted(error, API_KEY);
}

fn assert_redacted(error: &JudgmentError, secret: &str) {
    let display = error.to_string();
    let debug = format!("{error:?}");

    assert!(
        !display.contains(secret),
        "Display leaked {secret}: {display}"
    );
    assert!(!debug.contains(secret), "Debug leaked {secret}: {debug}");
    assert!(display.contains("[redacted]"));
    assert!(debug.contains("[redacted]"));
}
