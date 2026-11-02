//! Google image error mapping tests.

use ai_interface::ImageGenerationError;
use serde_json::json;

use crate::google::google_tests::stream_support::error_context;

use super::super::{
    error::{classify_request_error, classify_status},
    response::parse_response,
};

#[test]
fn prompt_blocks_are_content_policy_refusals() {
    let error = parse_response(
        &error_context("gemini-3.1-flash-image"),
        &Default::default(),
        "gemini-3.1-flash-image",
        json!({
            "promptFeedback": {
                "blockReason": "SAFETY",
                "blockReasonMessage": "unsafe prompt"
            }
        }),
    )
    .unwrap_err();

    assert!(
        matches!(error, ref error if error.core().kind == ai_interface::ErrorKind::ContentPolicy && (error.core().message == "unsafe prompt"))
    );
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
            &error_context("gemini-3.1-flash-image"),
            &Default::default(),
            "gemini-3.1-flash-image",
            json!({"candidates": [{
                "finishReason": reason,
                "finishMessage": "provider refused image"
            }]}),
        )
        .unwrap_err();
        assert!(
            matches!(error, ref error if error.core().kind == ai_interface::ErrorKind::ContentPolicy && (error.core().message == "provider refused image"))
        );
    }
}

#[test]
fn no_image_and_other_finish_failures_stay_distinct() {
    assert!(matches!(
        parse_response(
            &error_context("gemini-3.1-flash-image"),
            &Default::default(),
            "gemini-3.1-flash-image",
            json!({"candidates": [{"finishReason": "NO_IMAGE"}]})
        ),
        Err(ImageGenerationError::NoImage { ref core }) if core.kind == ai_interface::ErrorKind::InvalidOutput
    ));
    assert!(
        matches!(parse_response(&error_context("gemini-3.1-flash-image"), &Default::default(), "gemini-3.1-flash-image", json!({"candidates": [{
                "finishReason": "IMAGE_OTHER",
                "finishMessage": "generation failed"
            }]})), Err(ref error) if error.core().kind == ai_interface::ErrorKind::InvalidOutput && (error.core().message == "generation failed"))
    );
}

#[test]
fn http_and_transport_failures_are_classified_for_retry() {
    assert!(
        matches!(classify_status(&error_context("gemini-3.1-flash-image"), 429, &Default::default(), &json!({})), ref error if error.core().kind == ai_interface::ErrorKind::RateLimited)
    );
    assert!(
        matches!(classify_status(&error_context("gemini-3.1-flash-image"), 503, &Default::default(), &json!({})), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
    assert!(
        matches!(classify_status(&error_context("gemini-3.1-flash-image"), 400, &Default::default(), &json!({"error": {"message": "bad request"}})), ref error if error.core().kind == ai_interface::ErrorKind::InvalidRequest && (error.core().message.contains("bad request")))
    );
    assert!(
        matches!(classify_request_error(&error_context("gemini-3.1-flash-image"), json_http::Error::transport("disconnected")), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
    assert!(
        matches!(classify_request_error(&error_context("gemini-3.1-flash-image"), json_http::Error::auth("unavailable")), ref error if error.core().kind == ai_interface::ErrorKind::Transient)
    );
}
