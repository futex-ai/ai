//! Google video generation request mapping.

use ai_interface::{
    VideoGenerationAspect, VideoGenerationError, VideoGenerationRequest, VideoGenerationResult,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct GoogleVideoRequest {
    instances: Vec<GoogleVideoInstance>,
    parameters: GoogleVideoParameters,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GoogleVideoInstance {
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image: Option<GoogleVideoImage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GoogleVideoImage {
    #[serde(rename = "inlineData")]
    inline_data: GoogleInlineData,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GoogleInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
struct GoogleVideoParameters {
    #[serde(rename = "aspectRatio")]
    aspect_ratio: &'static str,
    #[serde(rename = "durationSeconds")]
    duration_seconds: u8,
    resolution: &'static str,
    #[serde(rename = "sampleCount")]
    sample_count: u8,
}

pub(super) fn build_request(
    request: &VideoGenerationRequest,
) -> VideoGenerationResult<GoogleVideoRequest> {
    if request.prompt.trim().is_empty() {
        return Err(VideoGenerationError::EmptyPrompt);
    }
    let image = match &request.input_image {
        Some(image) => {
            validate_media_type(&image.mime_type)?;
            Some(GoogleVideoImage {
                inline_data: GoogleInlineData {
                    mime_type: image.mime_type.clone(),
                    data: STANDARD.encode(&image.data),
                },
            })
        }
        None => None,
    };
    Ok(GoogleVideoRequest {
        instances: vec![GoogleVideoInstance {
            prompt: request.prompt.clone(),
            image,
        }],
        parameters: GoogleVideoParameters {
            aspect_ratio: match request.aspect {
                VideoGenerationAspect::Landscape => "16:9",
                VideoGenerationAspect::Portrait => "9:16",
            },
            duration_seconds: request.duration.seconds(),
            resolution: "720p",
            sample_count: 1,
        },
    })
}

fn validate_media_type(mime_type: &str) -> VideoGenerationResult<()> {
    match mime_type {
        "image/jpeg" | "image/png" | "image/webp" => Ok(()),
        _ => Err(VideoGenerationError::unsupported_media_type(mime_type)),
    }
}
