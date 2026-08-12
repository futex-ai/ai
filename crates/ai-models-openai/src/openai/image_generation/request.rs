//! OpenAI image generation request mapping.

use ai_interface::{
    ImageGenerationAspect, ImageGenerationError, ImageGenerationInputImage, ImageGenerationQuality,
    ImageGenerationRequest, ImageGenerationResult,
};
use serde::Serialize;

#[derive(Debug)]
pub(super) enum OpenAiImageApiRequest {
    Generation(OpenAiGenerationRequest),
    Edit(OpenAiEditRequest),
}

#[derive(Debug, Serialize)]
pub(super) struct OpenAiGenerationRequest {
    pub(super) model: String,
    pub(super) prompt: String,
    pub(super) size: &'static str,
    pub(super) quality: &'static str,
    pub(super) n: u8,
}

#[derive(Debug)]
pub(super) struct OpenAiEditRequest {
    pub(super) model: String,
    pub(super) prompt: String,
    pub(super) size: &'static str,
    pub(super) quality: &'static str,
    pub(super) images: Vec<ImageGenerationInputImage>,
}

pub(super) fn build_request(
    model_id: &str,
    request: &ImageGenerationRequest,
) -> ImageGenerationResult<OpenAiImageApiRequest> {
    if request.prompt.trim().is_empty() {
        return Err(ImageGenerationError::EmptyPrompt);
    }
    for image in &request.input_images {
        validate_media_type(&image.mime_type)?;
    }
    let model = model_id.to_owned();
    let prompt = request.prompt.clone();
    let size = map_aspect(request.aspect);
    let quality = map_quality(request.quality);
    if request.input_images.is_empty() {
        return Ok(OpenAiImageApiRequest::Generation(OpenAiGenerationRequest {
            model,
            prompt,
            size,
            quality,
            n: 1,
        }));
    }
    Ok(OpenAiImageApiRequest::Edit(OpenAiEditRequest {
        model,
        prompt,
        size,
        quality,
        images: request.input_images.clone(),
    }))
}

pub(super) fn media_type_extension(content_type: &str) -> &'static str {
    match content_type {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/webp" => "webp",
        _ => "image",
    }
}

fn validate_media_type(content_type: &str) -> ImageGenerationResult<()> {
    match content_type {
        "image/png" | "image/jpeg" | "image/webp" => Ok(()),
        _ => Err(ImageGenerationError::unsupported_media_type(content_type)),
    }
}

fn map_aspect(aspect: ImageGenerationAspect) -> &'static str {
    match aspect {
        ImageGenerationAspect::Auto => "auto",
        ImageGenerationAspect::Square => "1024x1024",
        ImageGenerationAspect::Landscape => "1536x1024",
        ImageGenerationAspect::Portrait => "1024x1536",
    }
}

fn map_quality(quality: ImageGenerationQuality) -> &'static str {
    match quality {
        ImageGenerationQuality::Auto => "auto",
        ImageGenerationQuality::Low => "low",
        ImageGenerationQuality::Medium => "medium",
        ImageGenerationQuality::High => "high",
    }
}
