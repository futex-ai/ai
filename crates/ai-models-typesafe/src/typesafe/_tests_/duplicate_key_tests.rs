//! Duplicate provider map key rejection tests through the public judge path.

use std::collections::BTreeMap;

use ai_interface::{JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentRequest};
use json_http::JsonHttpResponse;

use super::{TypeSafeJudgmentModel, test_support::recording_http_client};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn duplicate_answer_ids_are_rejected_instead_of_last_wins() {
    let body = br#"{
        "model":"jev-1.13.0",
        "answers":{
            "is_urgent":{"type":"noul","noul":0.2},
            "is_urgent":{"type":"noul","noul":0.9}
        }
    }"#;

    let error = judge_raw(condition_request(), body).await;

    assert_malformed(&error, "a unique map key");
}

#[tokio::test]
async fn duplicate_choice_labels_are_rejected_instead_of_last_wins() {
    let body = br#"{
        "model":"jev-1.13.0",
        "answers":{
            "team":{
                "type":"choice",
                "choice":"billing",
                "probabilities":{"billing":0.1,"billing":0.9,"technical":0.1},
                "confidence":0.8
            }
        }
    }"#;

    let error = judge_raw(choice_request(), body).await;

    assert_malformed(&error, "a unique map key");
}

async fn judge_raw(request: JudgmentRequest, body: &[u8]) -> JudgmentError {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: body.to_vec(),
    });

    TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&request)
        .await
        .expect_err("duplicate provider keys should fail")
}

fn assert_malformed(error: &JudgmentError, expected_detail: &str) {
    let JudgmentError::Provider { message, .. } = error else {
        panic!("expected a provider error, got {error}");
    };
    assert!(
        message.starts_with("malformed provider payload: "),
        "{message}"
    );
    assert!(message.contains(expected_detail), "{message}");
}

fn condition_request() -> JudgmentRequest {
    JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([(
            "is_urgent".to_owned(),
            JudgmentQuestion::condition("Urgent?", None),
        )]),
    }
}

fn choice_request() -> JudgmentRequest {
    JudgmentRequest {
        state: "ticket".into(),
        questions: BTreeMap::from([(
            "team".to_owned(),
            JudgmentQuestion::choice("Team?", [("billing", None), ("technical", None)]),
        )]),
    }
}
