//! Google image generation request mapping.

use ai_interface::{
    ImageGenerationAspect, ImageGenerationError, ImageGenerationRequest, ImageGenerationResult,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(super) struct GoogleImageRequest {
    contents: Vec<GoogleImageContent>,
    #[serde(rename = "generationConfig")]
    generation_config: GoogleImageGenerationConfig,
}

#[derive(Debug, Serialize)]
struct GoogleImageContent {
    role: &'static str,
    parts: Vec<GoogleImagePart>,
}

#[derive(Debug, Serialize)]
struct GoogleImagePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    inline_data: Option<GoogleInlineData>,
}

#[derive(Debug, Serialize)]
struct GoogleInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct GoogleImageGenerationConfig {
    #[serde(rename = "responseModalities")]
    response_modalities: [&'static str; 1],
    #[serde(rename = "responseFormat", skip_serializing_if = "Option::is_none")]
    response_format: Option<GoogleResponseFormat>,
}

#[derive(Debug, Serialize)]
struct GoogleResponseFormat {
    image: GoogleImageFormat,
}

#[derive(Debug, Serialize)]
struct GoogleImageFormat {
    #[serde(rename = "aspectRatio")]
    aspect_ratio: &'static str,
}

pub(super) fn build_request(
    request: &ImageGenerationRequest,
) -> ImageGenerationResult<GoogleImageRequest> {
    if request.prompt.trim().is_empty() {
        return Err(ImageGenerationError::EmptyPrompt);
    }
    let mut parts = vec![GoogleImagePart {
        text: Some(request.prompt.clone()),
        inline_data: None,
    }];
    for image in &request.input_images {
        validate_media_type(&image.mime_type)?;
        parts.push(GoogleImagePart {
            text: None,
            inline_data: Some(GoogleInlineData {
                mime_type: image.mime_type.clone(),
                data: STANDARD.encode(&image.data),
            }),
        });
    }
    Ok(GoogleImageRequest {
        contents: vec![GoogleImageContent {
            role: "user",
            parts,
        }],
        generation_config: GoogleImageGenerationConfig {
            response_modalities: ["IMAGE"],
            response_format: map_aspect(request.aspect).map(|aspect_ratio| GoogleResponseFormat {
                image: GoogleImageFormat { aspect_ratio },
            }),
        },
    })
}

fn validate_media_type(content_type: &str) -> ImageGenerationResult<()> {
    match content_type {
        "image/png" | "image/jpeg" | "image/webp" => Ok(()),
        _ => Err(ImageGenerationError::unsupported_media_type(content_type)),
    }
}

fn map_aspect(aspect: ImageGenerationAspect) -> Option<&'static str> {
    match aspect {
        ImageGenerationAspect::Auto => None,
        ImageGenerationAspect::Square => Some("1:1"),
        ImageGenerationAspect::Landscape => Some("3:2"),
        ImageGenerationAspect::Portrait => Some("2:3"),
    }
}
