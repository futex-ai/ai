//! OpenAI Decisions HTTP dispatch and local validation tests.

use std::collections::BTreeMap;
use std::time::Duration;

use ai_interface::{
    JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentQuestionProblem, JudgmentRequest,
};
use json_http::{JsonHttpBody, JsonHttpMethod, JsonHttpResponse};
use serde_json::json;

use super::super::test_support::{
    recording_http_client, simple_request, successful_response, unused_http_client,
};
use super::OpenAiJudgmentModel;

const API_KEY: &str = "openai-secret-key";

#[tokio::test]
async fn sends_the_exact_bearer_authenticated_request() {
    let (http_client, requests) = recording_http_client(successful_response());
    let model = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", API_KEY);

    let response = model
        .judge(&simple_request())
        .await
        .expect("valid response should parse");

    assert_eq!(response.provider, "openai");
    assert_eq!(response.model_id, "gpt-6-luna");
    assert_eq!(response.resolved_model_id, "gpt-6-luna");
    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
    assert_eq!(requests[0].url, "https://api.openai.com/v1/decisions");
    assert_eq!(requests[0].timeout, Duration::from_secs(60));
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer openai-secret-key")
    );
    assert_eq!(
        requests[0]
            .body
            .as_ref()
            .and_then(JsonHttpBody::as_json)
            .expect("request body should be JSON"),
        &json!({
            "model": "gpt-6-luna",
            "input": "A payout has been delayed for three days.",
            "questions": [
                {
                    "type": "predicate",
                    "name": "is_urgent",
                    "instructions": "Is this urgent?"
                }
            ]
        })
    );
}

#[tokio::test]
async fn malformed_success_body_retains_the_decoder_diagnostic() {
    let response = JsonHttpResponse {
        status: 200,
        body: br#"{"model":"gpt-6-luna"}"#.to_vec(),
    };
    let (http_client, _) = recording_http_client(response);

    let error = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", API_KEY)
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
async fn redirects_with_valid_judgments_are_terminal_provider_errors() {
    let mut response = successful_response();
    response.status = 302;
    let (http_client, _) = recording_http_client(response);

    let error = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("redirects must not be parsed as judgments");

    assert!(matches!(error, JudgmentError::Provider { .. }));
}

#[tokio::test]
async fn local_validation_returns_the_exact_error_before_transport() {
    let model = OpenAiJudgmentModel::new(unused_http_client(), "gpt-6-luna", API_KEY);

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

    let too_few_levels = model
        .judge(&JudgmentRequest {
            state: "ticket".into(),
            questions: BTreeMap::from([(
                "score".to_owned(),
                JudgmentQuestion::score("Score?", ["only"]),
            )]),
        })
        .await
        .expect_err("single-level score should fail locally");
    assert!(matches!(
        too_few_levels,
        JudgmentError::InvalidQuestion {
            id,
            problem: JudgmentQuestionProblem::TooFewLevels { levels: 1 },
        } if id == "score"
    ));
}
