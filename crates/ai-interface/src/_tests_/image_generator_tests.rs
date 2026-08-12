//! Image generation boundary tests.

use serde_json::json;
use thiserror::Error;

use crate::{
    GeneratedImage, ImageGenerationAspect, ImageGenerationError, ImageGenerationInputImage,
    ImageGenerationQuality, ImageGenerationRequest, ImageGenerationResponse, ModelUsage,
};

#[test]
fn request_defaults_optional_inputs_and_controls() {
    let request: ImageGenerationRequest =
        serde_json::from_value(json!({ "prompt": "Draw an otter" })).unwrap();

    assert_eq!(
        request,
        ImageGenerationRequest {
            prompt: "Draw an otter".to_owned(),
            input_images: Vec::new(),
            aspect: ImageGenerationAspect::Auto,
            quality: ImageGenerationQuality::Auto,
        }
    );
}

#[test]
fn request_and_response_dtos_round_trip_with_snake_case_enums() {
    let request = ImageGenerationRequest {
        prompt: "Turn this into a poster".to_owned(),
        input_images: vec![ImageGenerationInputImage {
            data: vec![1, 2, 3],
            mime_type: "image/png".to_owned(),
        }],
        aspect: ImageGenerationAspect::Landscape,
        quality: ImageGenerationQuality::High,
    };
    let response = ImageGenerationResponse {
        provider: "mock".to_owned(),
        model_id: "mock-image".to_owned(),
        image: GeneratedImage {
            data: vec![4, 5, 6],
            mime_type: "image/webp".to_owned(),
        },
        revised_prompt: Some("A refined poster".to_owned()),
        usage: ModelUsage {
            input_tokens: 2,
            output_tokens: 3,
            total_tokens: 5,
            ..ModelUsage::default()
        },
    };

    assert_eq!(
        serde_json::to_value(&request).unwrap(),
        json!({
            "prompt": "Turn this into a poster",
            "input_images": [{"data": [1, 2, 3], "mime_type": "image/png"}],
            "aspect": "landscape",
            "quality": "high"
        })
    );
    assert_eq!(
        serde_json::from_value::<ImageGenerationRequest>(
            serde_json::to_value(request.clone()).unwrap()
        )
        .unwrap(),
        request
    );
    assert_eq!(
        serde_json::from_value::<ImageGenerationResponse>(
            serde_json::to_value(response.clone()).unwrap()
        )
        .unwrap(),
        response
    );
}

#[test]
fn enum_defaults_are_auto() {
    assert_eq!(
        ImageGenerationAspect::default(),
        ImageGenerationAspect::Auto
    );
    assert_eq!(
        ImageGenerationQuality::default(),
        ImageGenerationQuality::Auto
    );
}

#[test]
fn error_helpers_preserve_typed_context_and_display_contract() {
    let unsupported = ImageGenerationError::unsupported_media_type("image/tiff");
    assert_eq!(
        unsupported.to_string(),
        "[ai_interface/image_generator] unsupported media type `image/tiff`"
    );
    assert!(matches!(
        unsupported,
        ImageGenerationError::UnsupportedMediaType { content_type }
            if content_type == "image/tiff"
    ));

    let policy = ImageGenerationError::content_policy("openai", "gpt-image-2", "blocked");
    assert_eq!(
        policy.to_string(),
        "[ai_interface/image_generator] content policy refusal for `openai` model `gpt-image-2`: blocked"
    );
    assert!(matches!(
        policy,
        ImageGenerationError::ContentPolicy { provider, model_id, message }
            if provider == "openai" && model_id == "gpt-image-2" && message == "blocked"
    ));

    assert!(matches!(
        ImageGenerationError::no_image("google", "gemini-image"),
        ImageGenerationError::NoImage { provider, model_id }
            if provider == "google" && model_id == "gemini-image"
    ));
    assert!(matches!(
        ImageGenerationError::rate_limited("openai", "image", "slow down"),
        ImageGenerationError::RateLimited { .. }
    ));
    assert!(matches!(
        ImageGenerationError::transient_provider("openai", "image", "retry"),
        ImageGenerationError::TransientProvider { .. }
    ));
    assert!(matches!(
        ImageGenerationError::provider("openai", "image", "bad request"),
        ImageGenerationError::Provider { .. }
    ));
}

#[test]
fn internal_error_preserves_contract_and_caller_locations() {
    let expected_line = line!() + 1;
    let error = ImageGenerationError::internal(TestSourceError);
    assert_eq!(
        error.to_string(),
        "[ai_interface/image_generator] internal error"
    );
    let ImageGenerationError::Internal(internal) = error else {
        panic!("expected internal error");
    };

    assert_eq!(
        internal.defined_at().module_path(),
        "ai_interface::image_generator"
    );
    assert_eq!(internal.caller_at().file(), file!());
    assert_eq!(internal.caller_at().line(), expected_line);
    assert_eq!(internal.source_ref().to_string(), "test source");
}

#[derive(Debug, Error)]
#[error("test source")]
struct TestSourceError;
