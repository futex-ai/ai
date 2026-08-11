//! OpenAI image client configuration and live smoke tests.

use std::time::Duration;

use ai_interface::{
    ImageGenerationAspect, ImageGenerationQuality, ImageGenerationRequest, ImageGenerator,
};

use super::super::client::OpenAiImageGenerator;

#[test]
fn client_has_provider_defaults_and_overrides() {
    let generator = OpenAiImageGenerator::new("gpt-image-2", "sk-test");

    assert_eq!(generator.timeout, Duration::from_secs(120));
    assert_eq!(
        generator.generation_endpoint,
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(
        generator.edit_endpoint,
        "https://api.openai.com/v1/images/edits"
    );

    let generator = generator
        .with_endpoints("http://generation.test", "http://edit.test")
        .with_timeout(Duration::from_secs(3));
    assert_eq!(generator.generation_endpoint, "http://generation.test");
    assert_eq!(generator.edit_endpoint, "http://edit.test");
    assert_eq!(generator.timeout, Duration::from_secs(3));
}

#[tokio::test]
#[ignore = "requires OPENAI_API_KEY and live provider access"]
async fn live_openai_image_generation_smoke() {
    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        return;
    };
    if api_key.trim().is_empty() {
        return;
    }
    let response = OpenAiImageGenerator::new("gpt-image-2", api_key)
        .generate(&ImageGenerationRequest {
            prompt: "A simple solid blue circle on white".to_owned(),
            aspect: ImageGenerationAspect::Square,
            quality: ImageGenerationQuality::Low,
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap();

    assert!(!response.image.data.is_empty());
    assert!(response.image.mime_type.starts_with("image/"));
}
