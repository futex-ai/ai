//! OpenAI image request mapping tests.

use ai_interface::{
    ImageGenerationAspect, ImageGenerationError, ImageGenerationInputImage, ImageGenerationQuality,
    ImageGenerationRequest,
};
use serde_json::json;

use super::super::request::{OpenAiImageApiRequest, build_request};

#[test]
fn text_to_image_maps_to_one_json_generation() {
    let request = ImageGenerationRequest {
        prompt: "A lighthouse in a storm".to_owned(),
        aspect: ImageGenerationAspect::Landscape,
        quality: ImageGenerationQuality::High,
        ..ImageGenerationRequest::default()
    };

    let mapped = build_request("gpt-image-2", &request).unwrap();

    let OpenAiImageApiRequest::Generation(body) = mapped else {
        panic!("text-to-image should select generation JSON");
    };
    assert_eq!(
        serde_json::to_value(body).unwrap(),
        json!({
            "model": "gpt-image-2",
            "prompt": "A lighthouse in a storm",
            "size": "1536x1024",
            "quality": "high",
            "n": 1
        })
    );
}

#[test]
fn input_images_select_multipart_edit_mapping() {
    let request = ImageGenerationRequest {
        prompt: "Add a red scarf".to_owned(),
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
        aspect: ImageGenerationAspect::Portrait,
        quality: ImageGenerationQuality::Medium,
    };

    let mapped = build_request("gpt-image-2", &request).unwrap();

    let OpenAiImageApiRequest::Edit(body) = mapped else {
        panic!("input images should select multipart editing");
    };
    assert_eq!(body.model, "gpt-image-2");
    assert_eq!(body.prompt, "Add a red scarf");
    assert_eq!(body.size, "1024x1536");
    assert_eq!(body.quality, "medium");
    assert_eq!(body.images, request.input_images);
}

#[test]
fn aspect_and_quality_values_map_exhaustively() {
    let cases = [
        (
            ImageGenerationAspect::Auto,
            ImageGenerationQuality::Auto,
            "auto",
            "auto",
        ),
        (
            ImageGenerationAspect::Square,
            ImageGenerationQuality::Low,
            "1024x1024",
            "low",
        ),
        (
            ImageGenerationAspect::Landscape,
            ImageGenerationQuality::Medium,
            "1536x1024",
            "medium",
        ),
        (
            ImageGenerationAspect::Portrait,
            ImageGenerationQuality::High,
            "1024x1536",
            "high",
        ),
    ];

    for (aspect, quality, size, expected_quality) in cases {
        let mapped = build_request(
            "gpt-image-2",
            &ImageGenerationRequest {
                prompt: "test".to_owned(),
                aspect,
                quality,
                ..ImageGenerationRequest::default()
            },
        )
        .unwrap();
        let OpenAiImageApiRequest::Generation(body) = mapped else {
            panic!("case should map to generation");
        };
        assert_eq!(body.size, size);
        assert_eq!(body.quality, expected_quality);
    }
}

#[test]
fn invalid_local_inputs_are_rejected_before_transport() {
    assert!(matches!(
        build_request("gpt-image-2", &ImageGenerationRequest::default()),
        Err(ImageGenerationError::EmptyPrompt)
    ));

    let error = build_request(
        "gpt-image-2",
        &ImageGenerationRequest {
            prompt: "edit".to_owned(),
            input_images: vec![ImageGenerationInputImage {
                data: vec![1],
                mime_type: "image/tiff".to_owned(),
            }],
            ..ImageGenerationRequest::default()
        },
    )
    .unwrap_err();
    assert!(matches!(
        error,
        ImageGenerationError::UnsupportedMediaType { content_type }
            if content_type == "image/tiff"
    ));
}

#[test]
fn common_edit_media_types_are_supported() {
    for mime_type in ["image/png", "image/jpeg", "image/webp"] {
        let mapped = build_request(
            "gpt-image-2",
            &ImageGenerationRequest {
                prompt: "edit".to_owned(),
                input_images: vec![ImageGenerationInputImage {
                    data: vec![1],
                    mime_type: mime_type.to_owned(),
                }],
                ..ImageGenerationRequest::default()
            },
        );
        assert!(mapped.is_ok(), "{mime_type} should be supported");
    }
}
