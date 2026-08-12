//! OpenAI image response mapping tests.

use ai_interface::{ImageGenerationError, ModelUsage};
use serde_json::json;

use super::super::response::parse_response;

#[test]
fn response_decodes_image_prompt_and_usage() {
    let response = parse_response(
        "gpt-image-2",
        json!({
            "data": [{
                "b64_json": "iVBORw0KGgo=",
                "revised_prompt": "A refined lighthouse"
            }],
            "output_format": "png",
            "usage": {
                "input_tokens": 11,
                "output_tokens": 13,
                "total_tokens": 24
            }
        }),
    )
    .unwrap();

    assert_eq!(response.provider, "openai");
    assert_eq!(response.model_id, "gpt-image-2");
    assert_eq!(response.image.data, b"\x89PNG\r\n\x1a\n");
    assert_eq!(response.image.mime_type, "image/png");
    assert_eq!(
        response.revised_prompt.as_deref(),
        Some("A refined lighthouse")
    );
    assert_eq!(response.usage.input_tokens, 11);
    assert_eq!(response.usage.output_tokens, 13);
    assert_eq!(response.usage.total_tokens, 24);
}

#[test]
fn response_infers_mime_type_when_output_format_is_absent() {
    let response = parse_response("gpt-image-2", json!({"data": [{"b64_json": "/9j/"}]})).unwrap();

    assert_eq!(response.image.mime_type, "image/jpeg");
}

#[test]
fn missing_usage_defaults_to_zero() {
    let response = parse_response(
        "gpt-image-2",
        json!({"data": [{"b64_json": "iVBORw0KGgo="}], "output_format": "png"}),
    )
    .unwrap();

    assert_eq!(response.usage, ModelUsage::default());
}

#[test]
fn absent_image_is_a_typed_no_image_error() {
    for body in [
        json!({}),
        json!({"data": []}),
        json!({"data": [{}]}),
        json!({"data": [{"b64_json": ""}], "output_format": "png"}),
    ] {
        assert!(matches!(
            parse_response("gpt-image-2", body),
            Err(ImageGenerationError::NoImage { .. })
        ));
    }
}

#[test]
fn malformed_base64_is_an_internal_error() {
    assert!(matches!(
        parse_response("gpt-image-2", json!({"data": [{"b64_json": "not base64"}]})),
        Err(ImageGenerationError::Internal(_))
    ));
}
