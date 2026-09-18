//! TypeSafe judgment HTTP dispatch and local validation tests.

use std::collections::BTreeMap;
use std::time::Duration;

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
            .and_then(JsonHttpBody::as_json)
            .expect("request body should be JSON"),
        &json!({
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

#[tokio::test]
async fn duplicate_score_keys_are_rejected_through_judge() {
    let request = JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([(
            "score".to_owned(),
            JudgmentQuestion::score("Score?", [None, None]),
        )]),
    };
    let response = JsonHttpResponse {
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

    assert!(matches!(error, JudgmentError::Provider { .. }));
}

#[tokio::test]
async fn malformed_success_body_retains_the_decoder_diagnostic() {
    let response = JsonHttpResponse {
        status: 200,
        body: br#"{"model":"jev-1.13.0"}"#.to_vec(),
    };
    let (http_client, _) = recording_http_client(response);

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("malformed response should return an error");

    let JudgmentError::Provider { message, .. } = error else {
        panic!("expected a terminal provider error");
    };
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
    assert!(matches!(empty_state, JudgmentError::EmptyState));

    let no_questions = model
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::new(),
        })
        .await
        .expect_err("missing questions should fail locally");
    assert!(matches!(no_questions, JudgmentError::NoQuestions));

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
    assert!(matches!(
        no_options,
        JudgmentError::InvalidQuestion {
            id,
            problem: JudgmentQuestionProblem::NoOptions,
        } if id == "choice"
    ));
}
