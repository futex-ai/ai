//! TypeSafe judgment HTTP dispatch and local validation tests.

use ai_interface::ErrorKind;

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;
use unimock::Unimock;

use ai_interface::{
    JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentQuestionProblem, JudgmentRequest,
};
use json_http::{JsonHttpBody, JsonHttpMethod, JsonHttpResponse};
use serde_json::json;

use super::{
    TypeSafeJudgmentModel,
    test_support::{
        recording_http_client, simple_request, successful_response, unused_http_client,
    },
};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn sends_the_exact_bearer_authenticated_request() {
    let (http_client, requests) = recording_http_client(successful_response());
    let model = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY);

    let response = model
        .judge(&simple_request())
        .await
        .expect("valid response should parse");

    assert_eq!(response.provider, "typesafe");
    assert_eq!(response.model_id, "jev-latest");
    {
        let requests = requests.lock().expect("request lock should be valid");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].method, JsonHttpMethod::Post);
        assert_eq!(requests[0].url, "https://api.typesafe.ai/v1/systemone");
        assert_eq!(requests[0].timeout, Duration::from_secs(60));
        assert_eq!(
            requests[0].headers.get("Authorization").map(String::as_str),
            Some("Bearer typesafe-secret-key")
        );
        assert_eq!(
            requests[0]
                .body
                .as_ref()
                .and_then(JsonHttpBody::as_json_bytes)
                .map(|bytes| serde_json::from_slice::<serde_json::Value>(bytes)
                    .expect("valid JSON request body"))
                .expect("request body should be JSON"),
            json!({
                "state": "A payout has been delayed for three days.",
                "model": "jev-latest",
                "questions": {
                    "is_urgent": {
                        "type": "noul",
                        "instructions": "Is this urgent?"
                    }
                }
            })
        );
    }
    let (http, compound_requests) = recording_http_client(successful_response());
    TypeSafeJudgmentModel::new(http, "jev-latest", "Scheme prefix local-secret")
        .judge(&simple_request())
        .await
        .unwrap();
    let compound_requests = compound_requests.lock().unwrap();
    assert_eq!(compound_requests.len(), 1);
    assert_eq!(
        compound_requests[0]
            .headers
            .get("Authorization")
            .map(String::as_str),
        Some("Bearer Scheme prefix local-secret")
    );
}

#[tokio::test]
async fn duplicate_score_keys_are_rejected_through_judge() {
    let request = JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([(
            "score".to_owned(),
            JudgmentQuestion::score("Score?", ["low", "high"]),
        )]),
    };
    let response = JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: br#"{
            "model":"jev-1.13.0",
            "answers":{
                "score":{
                    "type":"score",
                    "score":0.5,
                    "probabilities":{"0":0.2,"0":0.5,"1":0.5},
                    "confidence":1.0
                }
            }
        }"#
        .to_vec(),
    };
    let (http_client, _) = recording_http_client(response);

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&request)
        .await
        .expect_err("duplicate score keys should fail");

    assert!(matches!(error.core().kind, ErrorKind::InvalidOutput));
}

#[tokio::test]
async fn malformed_success_body_retains_the_decoder_diagnostic() {
    let response = JsonHttpResponse {
        headers: Default::default(),
        status: 200,
        body: br#"{"model":"jev-1.13.0"}"#.to_vec(),
    };
    let (http_client, _) = recording_http_client(response);

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("malformed response should return an error");

    let message = &error.core().message;
    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert!(message.starts_with("malformed provider payload: "));
    assert!(message.contains("answers"));
}

#[tokio::test]
async fn local_validation_returns_the_exact_error_before_transport() {
    let model = TypeSafeJudgmentModel::new(unused_http_client(), "jev-latest", API_KEY);

    let empty_state = model
        .judge(&JudgmentRequest {
            state: "  ".into(),
            questions: simple_request().questions,
        })
        .await
        .expect_err("empty state should fail locally");
    assert_eq!(empty_state.core().provider, "typesafe");
    assert_eq!(empty_state.core().model_id, "jev-latest");
    assert!(
        matches!(empty_state, JudgmentError::EmptyState { ref core } if core.kind == ErrorKind::InvalidRequest)
    );

    let no_questions = model
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::new(),
        })
        .await
        .expect_err("missing questions should fail locally");
    assert_eq!(no_questions.core().provider, "typesafe");
    assert_eq!(no_questions.core().model_id, "jev-latest");
    assert!(
        matches!(no_questions, JudgmentError::NoQuestions { ref core } if core.kind == ErrorKind::InvalidRequest)
    );

    let no_options = model
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::from([(
                "choice".to_owned(),
                JudgmentQuestion::Choice {
                    instructions: "Choose".into(),
                    options: BTreeMap::new(),
                },
            )]),
        })
        .await
        .expect_err("choice without options should fail locally");
    assert_eq!(no_options.core().provider, "typesafe");
    assert_eq!(no_options.core().model_id, "jev-latest");
    assert!(matches!(
        no_options,
        JudgmentError::InvalidQuestion {
            id,
            problem: JudgmentQuestionProblem::NoOptions,
            core,
        } if id == "choice" && core.kind == ErrorKind::InvalidRequest
    ));

    let secret_id = model
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::from([(
                API_KEY.to_owned(),
                JudgmentQuestion::Choice {
                    instructions: "Choose".into(),
                    options: BTreeMap::new(),
                },
            )]),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(&secret_id, JudgmentError::InvalidQuestion { id, problem: JudgmentQuestionProblem::NoOptions, .. } if id == "[redacted]")
    );
    assert!(!secret_id.to_string().contains(API_KEY));
    assert!(!format!("{secret_id:?}").contains(API_KEY));
    let dynamic = TypeSafeJudgmentModel::with_auth(
        unused_http_client(),
        "jev-custom",
        Arc::new(Unimock::new(())),
    );
    let invalid = dynamic
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::new(),
        })
        .await
        .unwrap_err();
    assert_eq!(invalid.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(invalid.core().provider, "typesafe");
    assert_eq!(invalid.core().model_id, "jev-custom");
    let mut compound = TypeSafeJudgmentModel::new(
        unused_http_client(),
        "jev-custom",
        "Scheme prefix local-secret   ",
    );
    compound.auth = Arc::new(Unimock::new(()));
    let compound_error = compound
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::from([(
                "q-local-secret".to_owned(),
                JudgmentQuestion::Choice {
                    instructions: "Choose".into(),
                    options: BTreeMap::new(),
                },
            )]),
        })
        .await
        .unwrap_err();
    assert!(
        matches!(&compound_error, JudgmentError::InvalidQuestion { id, problem: JudgmentQuestionProblem::NoOptions, .. } if id == "q-[redacted]")
    );
    assert_eq!(compound_error.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(compound_error.core().provider, "typesafe");
    assert_eq!(compound_error.core().model_id, "jev-custom");
    assert!(!compound_error.to_string().contains("local-secret"));
    assert!(!format!("{compound_error:?}").contains("local-secret"));
}
