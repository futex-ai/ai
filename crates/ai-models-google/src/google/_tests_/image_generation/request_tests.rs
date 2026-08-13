//! Google image request mapping tests.

use ai_interface::{
    ImageGenerationAspect, ImageGenerationError, ImageGenerationInputImage, ImageGenerationQuality,
    ImageGenerationRequest,
};
use serde_json::{Value, json};

use super::super::request::build_request;

#[test]
fn prompt_and_ordered_inputs_map_to_image_only_request() {
    let body = build_request(&ImageGenerationRequest {
        prompt: "Put both products on a table".to_owned(),
        input_images: vec![
            ImageGenerationInputImage {
                data: vec![1, 2],
                mime_type: "image/png".to_owned(),
            },
            ImageGenerationInputImage {
                data: vec![3, 4],
                mime_type: "image/jpeg".to_owned(),
            },
        ],
        aspect: ImageGenerationAspect::Landscape,
        quality: ImageGenerationQuality::High,
    })
    .unwrap();

    assert_eq!(
        serde_json::to_value(body).unwrap(),
        json!({
            "contents": [{
                "role": "user",
                "parts": [
                    {"text": "Put both products on a table"},
                    {"inlineData": {"mimeType": "image/png", "data": "AQI="}},
                    {"inlineData": {"mimeType": "image/jpeg", "data": "AwQ="}}
                ]
            }],
            "generationConfig": {
                "responseModalities": ["IMAGE"],
                "responseFormat": {
                    "image": {"aspectRatio": "ASPECT_RATIO_THREE_BY_TWO"}
                }
            }
        })
    );
}

#[test]
fn aspects_map_exhaustively_and_auto_omits_response_format() {
    for (aspect, expected) in [
        (ImageGenerationAspect::Auto, None),
        (
            ImageGenerationAspect::Square,
            Some("ASPECT_RATIO_ONE_BY_ONE"),
        ),
        (
            ImageGenerationAspect::Landscape,
            Some("ASPECT_RATIO_THREE_BY_TWO"),
        ),
        (
            ImageGenerationAspect::Portrait,
            Some("ASPECT_RATIO_TWO_BY_THREE"),
        ),
    ] {
        let body = serde_json::to_value(
            build_request(&ImageGenerationRequest {
                prompt: "test".to_owned(),
                aspect,
                ..ImageGenerationRequest::default()
            })
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            body.pointer("/generationConfig/responseFormat/image/aspectRatio")
                .and_then(Value::as_str),
            expected
        );
    }
}

#[test]
fn quality_is_intentionally_ignored() {
    let map = |quality| {
        serde_json::to_value(
            build_request(&ImageGenerationRequest {
                prompt: "test".to_owned(),
                quality,
                ..ImageGenerationRequest::default()
            })
            .unwrap(),
        )
        .unwrap()
    };

    assert_eq!(
        map(ImageGenerationQuality::Low),
        map(ImageGenerationQuality::High)
    );
}

#[test]
fn invalid_local_inputs_are_rejected() {
    assert!(matches!(
        build_request(&ImageGenerationRequest::default()),
        Err(ImageGenerationError::EmptyPrompt)
    ));
    assert!(matches!(
        build_request(&ImageGenerationRequest {
            prompt: "edit".to_owned(),
            input_images: vec![ImageGenerationInputImage {
                data: vec![1],
                mime_type: "image/tiff".to_owned(),
            }],
            ..ImageGenerationRequest::default()
        }),
        Err(ImageGenerationError::UnsupportedMediaType { content_type })
            if content_type == "image/tiff"
    ));
}

#[test]
fn common_edit_media_types_are_supported() {
    for mime_type in ["image/png", "image/jpeg", "image/webp"] {
        assert!(
            build_request(&ImageGenerationRequest {
                prompt: "edit".to_owned(),
                input_images: vec![ImageGenerationInputImage {
                    data: vec![1],
                    mime_type: mime_type.to_owned(),
                }],
                ..ImageGenerationRequest::default()
            })
            .is_ok()
        );
    }
}
