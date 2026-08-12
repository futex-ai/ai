//! Google image error mapping tests.

use ai_interface::ImageGenerationError;
use serde_json::json;

use super::super::{
    error::{classify_request_error, classify_status},
    response::parse_response,
};

#[test]
fn prompt_blocks_are_content_policy_refusals() {
    let error = parse_response(
        "gemini-3.1-flash-image",
        json!({
            "promptFeedback": {
                "blockReason": "SAFETY",
                "blockReasonMessage": "unsafe prompt"
            }
        }),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        ImageGenerationError::ContentPolicy { message, .. }
            if message == "unsafe prompt"
    ));
}

#[test]
fn documented_finish_reasons_are_content_policy_refusals() {
    for reason in [
        "SAFETY",
        "RECITATION",
        "BLOCKLIST",
        "PROHIBITED_CONTENT",
        "SPII",
        "IMAGE_SAFETY",
        "IMAGE_PROHIBITED_CONTENT",
        "IMAGE_RECITATION",
        "ESCALATION",
    ] {
        let error = parse_response(
            "gemini-3.1-flash-image",
            json!({"candidates": [{
                "finishReason": reason,
                "finishMessage": "provider refused image"
            }]}),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ImageGenerationError::ContentPolicy { message, .. }
                if message == "provider refused image"
        ));
    }
}

#[test]
fn no_image_and_other_finish_failures_stay_distinct() {
    assert!(matches!(
        parse_response(
            "gemini-3.1-flash-image",
            json!({"candidates": [{"finishReason": "NO_IMAGE"}]})
        ),
        Err(ImageGenerationError::NoImage { .. })
    ));
    assert!(matches!(
        parse_response(
            "gemini-3.1-flash-image",
            json!({"candidates": [{
                "finishReason": "IMAGE_OTHER",
                "finishMessage": "generation failed"
            }]})
        ),
        Err(ImageGenerationError::Provider { message, .. })
            if message == "generation failed"
    ));
}

#[test]
fn http_and_transport_failures_are_classified_for_retry() {
    assert!(matches!(
        classify_status(429, "gemini-3.1-flash-image", &json!({})),
        ImageGenerationError::RateLimited { .. }
    ));
    assert!(matches!(
        classify_status(503, "gemini-3.1-flash-image", &json!({})),
        ImageGenerationError::TransientProvider { .. }
    ));
    assert!(matches!(
        classify_status(
            400,
            "gemini-3.1-flash-image",
            &json!({"error": {"message": "bad request"}})
        ),
        ImageGenerationError::Provider { message, .. } if message.contains("bad request")
    ));
    assert!(matches!(
        classify_request_error(
            json_http::Error::transport("disconnected"),
            "gemini-3.1-flash-image"
        ),
        ImageGenerationError::TransientProvider { .. }
    ));
    assert!(matches!(
        classify_request_error(
            json_http::Error::auth("unavailable"),
            "gemini-3.1-flash-image"
        ),
        ImageGenerationError::TransientProvider { .. }
    ));
}
