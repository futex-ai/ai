//! Google image response mapping tests.

use ai_interface::ImageGenerationError;
use serde_json::json;

use crate::google::google_tests::stream_support::error_context;

use super::super::response::parse_response;

#[test]
fn valid_final_image_ignores_an_extra_top_level_error() {
    let result = parse_response(
        &error_context("gemini-3.1-flash-image"),
        &Default::default(),
        "gemini-3.1-flash-image",
        json!({
            "candidates":[{"finishReason":"STOP","content":{"parts":[{"inlineData":{"mimeType":"image/png","data":"AQID"}}]}}],
            "error":{"code":429,"message":"extra diagnostic"}
        }),
    );
    assert_eq!(
        result
            .map(|response| (response.image.data, response.image.mime_type))
            .map_err(|error| error.core().kind),
        Ok((vec![1, 2, 3], "image/png".to_owned()))
    );
}

#[test]
fn final_inline_image_and_usage_are_normalized() {
    let response = parse_response(
        &error_context("gemini-3.1-flash-image"),
        &Default::default(),
        "gemini-3.1-flash-image",
        json!({
            "candidates": [{
                "finishReason": "STOP",
                "content": {"parts": [{
                    "inlineData": {"mimeType": "image/png", "data": "AQID"}
                }]}
            }],
            "usageMetadata": {
                "promptTokenCount": 12,
                "cachedContentTokenCount": 4,
                "candidatesTokenCount": 6,
                "thoughtsTokenCount": 5,
                "totalTokenCount": 23
            }
        }),
    )
    .unwrap();

    assert_eq!(response.provider, "google");
    assert_eq!(response.model_id, "gemini-3.1-flash-image");
    assert_eq!(response.image.data, vec![1, 2, 3]);
    assert_eq!(response.image.mime_type, "image/png");
    assert_eq!(response.revised_prompt, None);
    assert_eq!(response.usage.input_tokens, 8);
    assert_eq!(response.usage.cached_input_tokens, 4);
    assert_eq!(response.usage.output_tokens, 6);
    assert_eq!(response.usage.reasoning_tokens, 5);
    assert_eq!(response.usage.total_tokens, 23);
}

#[test]
fn interim_thought_images_are_skipped() {
    let response = parse_response(
        &error_context("gemini-3.1-flash-image"),
        &Default::default(),
        "gemini-3.1-flash-image",
        json!({"candidates": [{"content": {"parts": [
            {"thought": true, "inlineData": {"mimeType": "image/png", "data": "AQ=="}},
            {"inlineData": {"mimeType": "image/webp", "data": "Ag=="}}
        ]}}]}),
    )
    .unwrap();

    assert_eq!(response.image.data, vec![2]);
    assert_eq!(response.image.mime_type, "image/webp");
}

#[test]
fn missing_image_and_missing_usage_are_typed() {
    for body in [
        json!({"candidates": [{"finishReason": "STOP", "content": {"parts": []}}]}),
        json!({"candidates": [{"content": {"parts": [{
            "inlineData": {"mimeType": "image/png", "data": ""}
        }]}}]}),
    ] {
        assert!(matches!(
            parse_response(
                &error_context("gemini-3.1-flash-image"),
                &Default::default(),
                "gemini-3.1-flash-image",
                body
            ),
            Err(ImageGenerationError::NoImage { ref core }) if core.kind == ai_interface::ErrorKind::InvalidOutput
        ));
    }
}

#[test]
fn malformed_base64_is_an_internal_error() {
    assert!(matches!(
        parse_response(&error_context("gemini-3.1-flash-image"), &Default::default(), "gemini-3.1-flash-image", json!({"candidates": [{"content": {"parts": [{
                "inlineData": {"mimeType": "image/png", "data": "not base64"}
            }]}}]})),
        Err(ImageGenerationError::Internal { ref core, .. }) if core.kind == ai_interface::ErrorKind::InvalidOutput
    ));
}

#[test]
fn missing_total_reconstructs_non_overlapping_usage() {
    let response = parse_response(
        &error_context("gemini-3.1-flash-image"),
        &Default::default(),
        "gemini-3.1-flash-image",
        json!({
            "candidates": [{"content": {"parts": [{
                "inlineData": {"mimeType": "image/png", "data": "AQ=="}
            }]}}],
            "usageMetadata": {
                "promptTokenCount": 12,
                "cachedContentTokenCount": 4,
                "candidatesTokenCount": 6,
                "thoughtsTokenCount": 5
            }
        }),
    )
    .unwrap();

    assert_eq!(response.usage.total_tokens, 23);
}
