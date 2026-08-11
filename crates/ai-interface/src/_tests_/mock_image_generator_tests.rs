//! Built-in mock image generator tests.

use crate::{
    GeneratedImage, ImageGenerationError, ImageGenerationRequest, ImageGenerator,
    MockImageGenerator,
};

#[tokio::test]
async fn default_mock_returns_a_valid_deterministic_png() {
    let generator = MockImageGenerator::default();

    let response = generator
        .generate(&ImageGenerationRequest {
            prompt: "Draw a test pixel".to_owned(),
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.provider, "mock");
    assert_eq!(response.model_id, "mock-image");
    assert_eq!(response.image.mime_type, "image/png");
    assert!(response.image.data.starts_with(b"\x89PNG\r\n\x1a\n"));
    assert_eq!(
        response,
        generator
            .generate(&ImageGenerationRequest {
                prompt: "A second prompt".to_owned(),
                ..ImageGenerationRequest::default()
            })
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn configurable_mock_returns_the_supplied_image() {
    let image = GeneratedImage {
        data: vec![8, 9],
        mime_type: "image/webp".to_owned(),
    };
    let generator = MockImageGenerator::new(image.clone());

    let response = generator
        .generate(&ImageGenerationRequest {
            prompt: "Use configured output".to_owned(),
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.image, image);
}

#[tokio::test]
async fn mock_rejects_a_blank_prompt() {
    let error = MockImageGenerator::default()
        .generate(&ImageGenerationRequest::default())
        .await
        .unwrap_err();

    assert!(matches!(error, ImageGenerationError::EmptyPrompt));
}
