//! Google long-running video operation response parsing.

use ai_interface::{ErrorKind, VideoGenerationError, VideoGenerationResult};
use ai_models_core::ErrorContext;
use json_http::JsonHttpHeaders;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::google::error_context::operation_status;

use super::error::classify_status;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum OperationState {
    Pending,
    Completed { download_uri: String },
}

#[derive(Debug, Deserialize)]
struct VideoOperation {
    name: String,
    #[serde(default)]
    done: bool,
    error: Option<OperationError>,
    response: Option<OperationResponse>,
}

#[derive(Debug, Deserialize)]
struct OperationError {
    code: Option<i64>,
    status: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OperationResponse {
    #[serde(rename = "generateVideoResponse")]
    generate_video_response: Option<GenerateVideoResponse>,
}

#[derive(Debug, Default, Deserialize)]
struct GenerateVideoResponse {
    #[serde(default, rename = "generatedSamples")]
    generated_samples: Vec<GeneratedSample>,
    #[serde(default, rename = "raiMediaFilteredCount")]
    rai_media_filtered_count: u32,
    #[serde(default, rename = "raiMediaFilteredReasons")]
    rai_media_filtered_reasons: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GeneratedSample {
    video: Option<GeneratedVideo>,
}

#[derive(Debug, Deserialize)]
struct GeneratedVideo {
    uri: Option<String>,
}

pub(super) fn parse_operation(
    context: &ErrorContext,
    headers: &JsonHttpHeaders,
    body: Value,
) -> VideoGenerationResult<(String, OperationState)> {
    let operation: VideoOperation = match serde_json::from_value(body) {
        Ok(operation) => operation,
        Err(source) => {
            return Err(VideoGenerationError::with_source(
                context.core(ErrorKind::InvalidOutput, "invalid provider video operation"),
                source,
            ));
        }
    };
    if !valid_operation_name(&operation.name) {
        return Err(VideoGenerationError::new(context.core(
            ErrorKind::InvalidOutput,
            "provider returned an invalid operation name",
        )));
    }
    if !operation.done {
        return Ok((operation.name, OperationState::Pending));
    }
    if let Some(error) = operation.error {
        return Err(operation_error(context, headers, error));
    }
    let response = operation
        .response
        .and_then(|response| response.generate_video_response)
        .unwrap_or_default();
    if response.rai_media_filtered_count > 0 || !response.rai_media_filtered_reasons.is_empty() {
        let message = if response.rai_media_filtered_reasons.is_empty() {
            "video was filtered by provider policy".to_owned()
        } else {
            response.rai_media_filtered_reasons.join("; ")
        };
        return Err(VideoGenerationError::new(
            context.core(ErrorKind::ContentPolicy, message),
        ));
    }
    let uri = response
        .generated_samples
        .into_iter()
        .find_map(|sample| sample.video.and_then(|video| video.uri))
        .filter(|uri| !uri.is_empty())
        .ok_or_else(|| VideoGenerationError::NoVideo {
            core: Box::new(context.core(ErrorKind::InvalidOutput, "no video returned")),
        })?;
    Ok((
        operation.name,
        OperationState::Completed { download_uri: uri },
    ))
}

fn operation_error(
    context: &ErrorContext,
    headers: &JsonHttpHeaders,
    error: OperationError,
) -> VideoGenerationError {
    let status = operation_status(error.code, error.status.as_deref());
    let message = error
        .message
        .unwrap_or_else(|| "video operation failed".to_owned());
    match status {
        Some(status) => classify_status(
            context,
            status,
            headers,
            &json!({"error":{"code":error.code,"status":error.status,"message":message}}),
        ),
        None => VideoGenerationError::new(context.core(ErrorKind::InvalidOutput, message)),
    }
}

fn valid_operation_name(name: &str) -> bool {
    name.starts_with("models/")
        && name.contains("/operations/")
        && name.len() <= 1024
        && !name.contains("..")
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'_' | b'.'))
}
