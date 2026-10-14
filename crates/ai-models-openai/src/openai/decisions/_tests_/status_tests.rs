//! OpenAI Decisions HTTP status classification tests.

use ai_interface::{JudgmentError, JudgmentModel};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::super::test_support::{json_response, recording_http_client, simple_request};
use super::OpenAiJudgmentModel;

const API_KEY: &str = "openai-secret-key";

#[tokio::test]
async fn classifies_rate_limited_and_transient_statuses() {
    for (status, expected) in [
        (429, ErrorKind::RateLimited),
        (503, ErrorKind::Transient),
        (529, ErrorKind::Transient),
        (408, ErrorKind::Transient),
    ] {
        let (http_client, _) = recording_http_client(json_response(
            status,
            json!({"error": {"message": "provider overloaded", "type": "server_error"}}),
        ));
        let error = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert!(expected.matches(&error), "status {status}: {error}");
        assert_provider_fields(&error, "provider overloaded");
    }
}

#[tokio::test]
async fn terminal_statuses_use_the_error_envelope_message() {
    for status in [400, 401, 403, 404, 422] {
        let (http_client, _) = recording_http_client(json_response(
            status,
            json!({
                "error": {
                    "message": "Decision API is not enabled for this user",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": null
                }
            }),
        ));
        let error = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert!(matches!(error, JudgmentError::Provider { .. }));
        assert_provider_fields(&error, "Decision API is not enabled for this user");
    }
}

#[tokio::test]
async fn message_extraction_retains_other_bodies_and_falls_back_for_empty_bytes() {
    let responses = [
        (
            json_response(400, json!("text rejection")),
            "text rejection",
        ),
        (
            json_response(400, json!({"error": "flat string error"})),
            "{\"error\":\"flat string error\"}",
        ),
        (
            json_response(400, json!({"code": "bad_request"})),
            "{\"code\":\"bad_request\"}",
        ),
        (json_response(400, json!(null)), "null"),
        (
            JsonHttpResponse {
                status: 400,
                body: b"not json".to_vec(),
            },
            "not json",
        ),
        (
            JsonHttpResponse {
                status: 400,
                body: Vec::new(),
            },
            "HTTP 400",
        ),
    ];

    for (response, expected) in responses {
        let (http_client, _) = recording_http_client(response);
        let error = OpenAiJudgmentModel::new(http_client, "gpt-6-luna", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert_provider_fields(&error, expected);
    }
}

fn assert_provider_fields(error: &JudgmentError, expected_message: &str) {
    match error {
        JudgmentError::RateLimited {
            provider,
            model_id,
            message,
        }
        | JudgmentError::TransientProvider {
            provider,
            model_id,
            message,
        }
        | JudgmentError::Provider {
            provider,
            model_id,
            message,
        } => {
            assert_eq!(provider, "openai");
            assert_eq!(model_id, "gpt-6-luna");
            assert_eq!(message, expected_message);
        }
        error => panic!("expected provider error, got {error}"),
    }
}

#[derive(Clone, Copy)]
enum ErrorKind {
    RateLimited,
    Transient,
}

impl ErrorKind {
    fn matches(self, error: &JudgmentError) -> bool {
        matches!(
            (self, error),
            (Self::RateLimited, JudgmentError::RateLimited { .. })
                | (Self::Transient, JudgmentError::TransientProvider { .. })
        )
    }
}
