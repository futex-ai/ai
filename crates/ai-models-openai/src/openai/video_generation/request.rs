//! OpenAI video generation request mapping.

use ai_interface::{
    VideoGenerationAspect, VideoGenerationError, VideoGenerationRequest, VideoGenerationResult,
};
use json_http::JsonHttpMultipartField;
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct OpenAiVideoRequest {
    model: String,
    prompt: String,
    seconds: String,
    size: String,
}

pub(super) enum OpenAiVideoApiRequest {
    Json(OpenAiVideoRequest),
    Multipart(Vec<JsonHttpMultipartField>),
}

pub(super) fn build_request(
    model_id: &str,
    request: &VideoGenerationRequest,
) -> VideoGenerationResult<OpenAiVideoApiRequest> {
    if request.prompt.trim().is_empty() {
        return Err(VideoGenerationError::EmptyPrompt);
    }
    let body = OpenAiVideoRequest {
        model: model_id.to_owned(),
        prompt: request.prompt.clone(),
        seconds: request.duration.seconds().to_string(),
        size: size(request.aspect).to_owned(),
    };
    let Some(image) = &request.input_image else {
        return Ok(OpenAiVideoApiRequest::Json(body));
    };
    validate_media_type(&image.mime_type)?;
    Ok(OpenAiVideoApiRequest::Multipart(vec![
        text_field("model", body.model),
        text_field("prompt", body.prompt),
        text_field("seconds", body.seconds),
        text_field("size", body.size),
        JsonHttpMultipartField::bytes("input_reference", image.data.clone())
            .filename(format!("input.{}", extension(&image.mime_type)))
            .content_type(image.mime_type.clone()),
    ]))
}

fn size(aspect: VideoGenerationAspect) -> &'static str {
    match aspect {
        VideoGenerationAspect::Landscape => "1280x720",
        VideoGenerationAspect::Portrait => "720x1280",
    }
}

fn validate_media_type(mime_type: &str) -> VideoGenerationResult<()> {
    match mime_type {
        "image/jpeg" | "image/png" | "image/webp" => Ok(()),
        _ => Err(VideoGenerationError::unsupported_media_type(mime_type)),
    }
}

fn extension(mime_type: &str) -> &'static str {
    match mime_type {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        _ => "bin",
    }
}

fn text_field(name: &str, value: impl Into<String>) -> JsonHttpMultipartField {
    JsonHttpMultipartField::bytes(name, value.into().into_bytes())
}
