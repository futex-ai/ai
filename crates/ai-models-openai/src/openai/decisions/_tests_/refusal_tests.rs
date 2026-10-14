//! OpenAI Decisions refusal mapping tests.

use ai_interface::{JudgmentError, JudgmentModel};
use serde_json::json;

use super::super::OpenAiJudgmentModel;
use super::super::test_support::{full_body, full_request, json_response, recording_http_client};
use super::parse_response;

#[test]
fn a_refused_requested_question_fails_the_call_with_the_lowest_refused_id() {
    let mut body = full_body();
    body["answers"][0] = json!({"type": "refusal", "name": "condition"});
    body["answers"][2] = json!({"type": "refusal", "name": "score"});
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");

    let error = parse_response("gpt-6-luna", &full_request(), &body)
        .expect_err("a refusal should fail the call");

    assert!(matches!(
        error,
        JudgmentError::Refused { provider, model_id, id }
            if provider == "openai" && model_id == "gpt-6-luna" && id == "condition"
    ));
}

#[test]
fn malformed_fragments_are_reported_before_refusals() {
    let mut body = full_body();
    body["answers"][0] = json!({"type": "refusal", "name": "condition"});
    body["answers"][1]["choice"] = json!(true);
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");

    let error = parse_response("gpt-6-luna", &full_request(), &body)
        .expect_err("a malformed fragment should fail the call");

    assert!(matches!(error, JudgmentError::Provider { .. }), "{error}");
}

#[test]
fn refusals_take_precedence_over_missing_answers() {
    let mut body = full_body();
    body["answers"] = json!([{"type": "refusal", "name": "score"}]);
    let body = serde_json::to_vec(&body).expect("response fixture should serialize");

    let error = parse_response("gpt-6-luna", &full_request(), &body)
        .expect_err("a refusal should fail the call");

    assert!(matches!(error, JudgmentError::Refused { id, .. } if id == "score"));
}

#[tokio::test]
async fn refusals_surface_through_the_public_judge_path() {
    let mut body = full_body();
    body["answers"][1] = json!({"type": "refusal", "name": "choice"});
    let (http_client, _) = recording_http_client(json_response(200, body));

    let error = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", "openai-key")
        .judge(&full_request())
        .await
        .expect_err("a refusal should fail the call");

    assert!(matches!(error, JudgmentError::Refused { id, .. } if id == "choice"));
}
