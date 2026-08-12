//! Built-in mock image generator for development and tests.

use async_trait::async_trait;

use crate::{
    GeneratedImage, ImageGenerationError, ImageGenerationRequest, ImageGenerationResponse,
    ImageGenerationResult, ImageGenerator, ModelUsage,
};

const DEFAULT_PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 4, 0,
    0, 0, 181, 28, 12, 2, 0, 0, 0, 11, 73, 68, 65, 84, 120, 218, 99, 100, 248, 15, 0, 1, 5, 1, 1,
    39, 24, 227, 102, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

/// Deterministic mock image generator used by development and tests.
#[derive(Clone, Debug)]
pub struct MockImageGenerator {
    image: GeneratedImage,
}

impl MockImageGenerator {
    /// Builds a mock generator that returns the supplied image.
    pub fn new(image: GeneratedImage) -> Self {
        Self { image }
    }
}

impl Default for MockImageGenerator {
    fn default() -> Self {
        Self::new(GeneratedImage {
            data: DEFAULT_PNG.to_vec(),
            mime_type: "image/png".to_owned(),
        })
    }
}

#[async_trait]
impl ImageGenerator for MockImageGenerator {
    async fn generate(
        &self,
        request: &ImageGenerationRequest,
    ) -> ImageGenerationResult<ImageGenerationResponse> {
        if request.prompt.trim().is_empty() {
            return Err(ImageGenerationError::EmptyPrompt);
        }
        Ok(ImageGenerationResponse {
            provider: "mock".to_owned(),
            model_id: "mock-image".to_owned(),
            image: self.image.clone(),
            revised_prompt: None,
            usage: ModelUsage::default(),
        })
    }
}
