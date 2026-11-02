//! TypeSafe judgment HTTP status classification tests.

use ai_interface::ErrorKind;

use ai_interface::{JudgmentError, JudgmentModel};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{
    TypeSafeJudgmentModel,
    test_support::{json_response, recording_http_client, simple_request, successful_response},
};

const API_KEY: &str = "typesafe-secret-key";

#[tokio::test]
async fn classifies_rate_limited_and_transient_statuses() {
    for (status, expected) in [
        (429, ErrorKind::RateLimited),
        (408, ErrorKind::Transient),
        (409, ErrorKind::Transient),
        (425, ErrorKind::Transient),
        (500, ErrorKind::Transient),
        (599, ErrorKind::Transient),
        (529, ErrorKind::Transient),
        (503, ErrorKind::Transient),
    ] {
        let (http_client, _) = recording_http_client(json_response(
            status,
            json!({"detail": {"message": "provider overloaded"}}),
        ));
        let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert!(expected == error.core().kind, "status {status}: {error}");
        assert_provider_fields(&error, "provider overloaded");
    }
}

#[tokio::test]
async fn terminal_statuses_use_nested_detail_messages() {
    for (status, kind) in [
        (401, ErrorKind::RejectedKey),
        (402, ErrorKind::RejectedKey),
        (403, ErrorKind::RejectedKey),
        (404, ErrorKind::UnknownModel),
        (400, ErrorKind::InvalidRequest),
        (413, ErrorKind::InvalidRequest),
        (422, ErrorKind::InvalidRequest),
        (451, ErrorKind::InvalidRequest),
    ] {
        let (http_client, _) = recording_http_client(json_response(
            status,
            json!({
                "detail": {
                    "error_type": "authentication_error",
                    "message": "request rejected"
                }
            }),
        ));
        let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert!(matches!(error.core().kind, actual if actual == kind));
        assert_provider_fields(&error, "request rejected");
    }
}

#[tokio::test]
async fn message_extraction_retains_null_and_falls_back_only_for_empty_bytes() {
    let responses = [
        (
            json_response(400, json!({"detail": "plain rejection"})),
            "plain rejection",
        ),
        (
            json_response(400, json!("text rejection")),
            "text rejection",
        ),
        (
            json_response(400, json!({"code": "bad_request"})),
            "{\"code\":\"bad_request\"}",
        ),
        (json_response(400, json!(null)), "null"),
        (
            JsonHttpResponse {
                headers: Default::default(),
                status: 400,
                body: Vec::new(),
            },
            "HTTP 400",
        ),
    ];

    for (response, expected) in responses {
        let (http_client, _) = recording_http_client(response);
        let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
            .judge(&simple_request())
            .await
            .expect_err("failing status should return an error");

        assert_provider_fields(&error, expected);
    }
}

#[tokio::test]
async fn redirects_with_valid_judgments_are_terminal_provider_errors() {
    let mut response = successful_response();
    response.status = 302;
    let (http_client, _) = recording_http_client(response);

    let error = TypeSafeJudgmentModel::new(http_client, "jev-latest", API_KEY)
        .judge(&simple_request())
        .await
        .expect_err("redirects must not be parsed as judgments");

    assert!(matches!(error.core().kind, ErrorKind::InvalidOutput));
}

fn assert_provider_fields(error: &JudgmentError, expected_message: &str) {
    assert_eq!(error.core().provider, "typesafe");
    assert_eq!(error.core().model_id, "jev-latest");
    assert_eq!(error.core().message, expected_message);
}
