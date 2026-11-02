//! Image generation boundary tests.

use serde_json::json;
use thiserror::Error;

use crate::{
    ErrorCore, ErrorKind, GeneratedImage, ImageGenerationAspect, ImageGenerationError,
    ImageGenerationInputImage, ImageGenerationQuality, ImageGenerationRequest,
    ImageGenerationResponse, ModelUsage,
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
    let unsupported = ImageGenerationError::unsupported_media_type("openai", "image", "image/tiff");
    assert_eq!(
        unsupported.to_string(),
        "[ai_interface/image_generator] unsupported media type `image/tiff`"
    );
    assert!(matches!(
        unsupported,
        ImageGenerationError::UnsupportedMediaType { content_type, core }
            if content_type == "image/tiff" && core.kind == ErrorKind::UnsupportedInput
                && core.provider == "openai" && core.model_id == "image"
    ));

    let policy = ImageGenerationError::content_policy("openai", "gpt-image-2", "blocked");
    assert_eq!(
        policy.to_string(),
        "[ai_interface/image_generator] ContentPolicy for `openai` model `gpt-image-2`: blocked"
    );
    assert_eq!(policy.core().kind, ErrorKind::ContentPolicy);
    assert_eq!(policy.core().provider, "openai");
    assert_eq!(policy.core().model_id, "gpt-image-2");
    assert_eq!(policy.core().message, "blocked");

    assert!(matches!(
        ImageGenerationError::no_image("google", "gemini-image"),
        ImageGenerationError::NoImage { core }
            if core.provider == "google" && core.model_id == "gemini-image"
                && core.kind == ErrorKind::InvalidOutput
    ));
    for (kind, message) in [
        (ErrorKind::RateLimited, "slow down"),
        (ErrorKind::Transient, "retry"),
        (ErrorKind::InvalidRequest, "bad request"),
    ] {
        let error = ImageGenerationError::new(ErrorCore::new(kind, "openai", "image", message));
        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().provider, "openai");
        assert_eq!(error.core().model_id, "image");
        assert_eq!(error.core().message, message);
    }
}

#[test]
fn internal_error_preserves_contract_and_caller_locations() {
    let expected_line = line!() + 1;
    let error = ImageGenerationError::internal("provider", "model", TestSourceError);
    assert_eq!(
        error.to_string(),
        "[ai_interface/image_generator] Internal for `provider` model `model`: internal error"
    );
    let ImageGenerationError::Internal {
        source: internal, ..
    } = error
    else {
        panic!("expected internal error");
    };

    assert_eq!(
        internal.defined_at().module_path(),
        "ai_interface::image_generator"
    );
    assert_eq!(internal.caller_at().file(), file!());
    assert_eq!(internal.caller_at().line(), expected_line);
    assert_eq!(internal.source_ref().to_string(), "test source");

    let core = ErrorCore::new(
        ErrorKind::InvalidOutput,
        "provider",
        "model",
        "malformed output",
    );
    let expected_line = line!() + 1;
    let error = ImageGenerationError::with_source(core, TestSourceError);
    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_eq!(error.core().provider, "provider");
    assert_eq!(error.core().model_id, "model");
    let ImageGenerationError::Internal {
        source: internal, ..
    } = error
    else {
        panic!("expected the tracked source");
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
