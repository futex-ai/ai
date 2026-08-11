//! OpenAI image generation response mapping.

use ai_interface::{
    GeneratedImage, ImageGenerationError, ImageGenerationResponse, ImageGenerationResult,
    ModelUsage,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

const PROVIDER: &str = "openai";

#[derive(Deserialize)]
struct OpenAiImagesResponse {
    data: Option<Vec<OpenAiImageData>>,
    output_format: Option<String>,
    usage: Option<OpenAiImageUsage>,
}

#[derive(Deserialize)]
struct OpenAiImageData {
    b64_json: Option<String>,
    revised_prompt: Option<String>,
}

#[derive(Default, Deserialize)]
struct OpenAiImageUsage {
    #[serde(default)]
    input_tokens: u64,
    #[serde(default)]
    output_tokens: u64,
    total_tokens: Option<u64>,
}

#[derive(Debug, Error)]
enum ResponseError {
    #[error(
        "[ai_models_openai/image_generation/response] generated image MIME type could not be inferred"
    )]
    UnknownImageFormat,
}

pub(super) fn parse_response(
    model_id: &str,
    body: Value,
) -> ImageGenerationResult<ImageGenerationResponse> {
    let response = match serde_json::from_value::<OpenAiImagesResponse>(body) {
        Ok(response) => response,
        Err(source) => return Err(ImageGenerationError::internal(source)),
    };
    let Some(item) = response.data.and_then(|items| items.into_iter().next()) else {
        return Err(ImageGenerationError::no_image(PROVIDER, model_id));
    };
    let Some(encoded) = item.b64_json else {
        return Err(ImageGenerationError::no_image(PROVIDER, model_id));
    };
    let data = match STANDARD.decode(encoded) {
        Ok(data) => data,
        Err(source) => return Err(ImageGenerationError::internal(source)),
    };
    let mime_type = match image_mime_type(response.output_format.as_deref(), &data) {
        Ok(mime_type) => mime_type,
        Err(source) => return Err(ImageGenerationError::internal(source)),
    };
    let usage = normalize_usage(response.usage.unwrap_or_default());
    Ok(ImageGenerationResponse {
        provider: PROVIDER.to_owned(),
        model_id: model_id.to_owned(),
        image: GeneratedImage { data, mime_type },
        revised_prompt: item.revised_prompt,
        usage,
    })
}

fn normalize_usage(usage: OpenAiImageUsage) -> ModelUsage {
    ModelUsage {
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        total_tokens: usage
            .total_tokens
            .unwrap_or(usage.input_tokens.saturating_add(usage.output_tokens)),
        ..ModelUsage::default()
    }
}

fn image_mime_type(output_format: Option<&str>, data: &[u8]) -> Result<String, ResponseError> {
    let reported = match output_format {
        Some("png") => Some("image/png"),
        Some("jpeg" | "jpg") => Some("image/jpeg"),
        Some("webp") => Some("image/webp"),
        _ => None,
    };
    if let Some(mime_type) = reported {
        return Ok(mime_type.to_owned());
    }
    if data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok("image/png".to_owned());
    }
    if data.starts_with(b"\xff\xd8\xff") {
        return Ok("image/jpeg".to_owned());
    }
    if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        return Ok("image/webp".to_owned());
    }
    Err(ResponseError::UnknownImageFormat)
}
