use ai_interface::{FinishReason, Model};
use json_http::JsonHttpResponse;
use serde_json::json;

use super::{GoogleModel, google_response_body, recording_http_client, simple_request};

#[tokio::test]
async fn maps_google_finish_reasons() {
    let cases = [
        ("STOP", FinishReason::Stop, false),
        ("MAX_TOKENS", FinishReason::Truncated, false),
        ("SAFETY", FinishReason::Filtered, false),
        ("BLOCKLIST", FinishReason::Filtered, false),
        ("PROHIBITED_CONTENT", FinishReason::Filtered, false),
        ("SPII", FinishReason::Filtered, false),
        ("RECITATION", FinishReason::Filtered, false),
        ("LANGUAGE", FinishReason::Filtered, false),
        ("IMAGE_SAFETY", FinishReason::Filtered, false),
        ("IMAGE_PROHIBITED_CONTENT", FinishReason::Filtered, false),
        ("IMAGE_RECITATION", FinishReason::Filtered, false),
        (
            "MALFORMED_FUNCTION_CALL",
            FinishReason::Other("MALFORMED_FUNCTION_CALL".to_owned()),
            false,
        ),
        (
            "UNEXPECTED_TOOL_CALL",
            FinishReason::Other("UNEXPECTED_TOOL_CALL".to_owned()),
            false,
        ),
        (
            "TOO_MANY_TOOL_CALLS",
            FinishReason::Other("TOO_MANY_TOOL_CALLS".to_owned()),
            false,
        ),
        (
            "MISSING_THOUGHT_SIGNATURE",
            FinishReason::Other("MISSING_THOUGHT_SIGNATURE".to_owned()),
            false,
        ),
        (
            "MALFORMED_RESPONSE",
            FinishReason::Other("MALFORMED_RESPONSE".to_owned()),
            false,
        ),
        (
            "FINISH_REASON_UNSPECIFIED",
            FinishReason::Other("FINISH_REASON_UNSPECIFIED".to_owned()),
            false,
        ),
        (
            "CUSTOM_REASON",
            FinishReason::Other("CUSTOM_REASON".to_owned()),
            false,
        ),
        ("STOP", FinishReason::ToolCalls, true),
        ("FINISH_REASON_UNSPECIFIED", FinishReason::ToolCalls, true),
    ];

    for (raw_reason, expected, include_tool) in cases {
        let (http_client, _) = recording_http_client(JsonHttpResponse {
            status: 200,
            body: google_response_body(Some(raw_reason), include_tool),
        });
        let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

        let response = model
            .complete(&simple_request())
            .await
            .expect("Google response should parse");

        assert_eq!(response.finish_reason, expected);
    }
}

#[tokio::test]
async fn maps_missing_google_finish_reason_with_tool_calls() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: google_response_body(None, true),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("Google response should parse");

    assert_eq!(response.finish_reason, FinishReason::ToolCalls);
}

#[tokio::test]
async fn maps_missing_google_finish_reason_without_tool_calls_to_other() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: google_response_body(None, false),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("Google response should parse");

    assert_eq!(
        response.finish_reason,
        FinishReason::Other("missing".to_owned())
    );
}

#[tokio::test]
async fn google_filtered_candidates_without_content_still_surface_finish_reason() {
    let (http_client, _) = recording_http_client(JsonHttpResponse {
        status: 200,
        body: json!({
            "candidates": [{
                "finishReason": "SAFETY"
            }]
        }),
    });
    let model = GoogleModel::new(http_client, "gemini-2.5-pro", "google-key");

    let response = model
        .complete(&simple_request())
        .await
        .expect("Google response should parse");

    assert_eq!(response.finish_reason, FinishReason::Filtered);
    assert!(response.assistant_message.is_empty());
}
