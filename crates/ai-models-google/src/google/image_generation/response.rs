//! Google image generation response mapping.

use ai_interface::{
    ErrorKind, GeneratedImage, ImageGenerationError, ImageGenerationResponse,
    ImageGenerationResult, ModelUsage,
};
use ai_models_core::ErrorContext;
use base64::{Engine, engine::general_purpose::STANDARD};
use json_http::JsonHttpHeaders;
use serde::Deserialize;
use serde_json::Value;

use crate::google::error_context::embedded_status;

use super::error::classify_status;

const PROVIDER: &str = "google";

#[derive(Deserialize)]
struct GoogleImageResponse {
    #[serde(default)]
    candidates: Vec<GoogleImageCandidate>,
    #[serde(default, rename = "promptFeedback")]
    prompt_feedback: Option<GooglePromptFeedback>,
    #[serde(default, rename = "usageMetadata")]
    usage_metadata: Option<GoogleImageUsage>,
}

#[derive(Deserialize)]
struct GooglePromptFeedback {
    #[serde(default, rename = "blockReason")]
    block_reason: Option<String>,
    #[serde(default, rename = "blockReasonMessage")]
    block_reason_message: Option<String>,
}

#[derive(Deserialize)]
struct GoogleImageCandidate {
    #[serde(default)]
    content: Option<GoogleImageContent>,
    #[serde(default, rename = "finishReason")]
    finish_reason: Option<String>,
    #[serde(default, rename = "finishMessage")]
    finish_message: Option<String>,
}

#[derive(Default, Deserialize)]
struct GoogleImageContent {
    #[serde(default)]
    parts: Vec<GoogleImagePart>,
}

#[derive(Deserialize)]
struct GoogleImagePart {
    #[serde(default)]
    thought: bool,
    #[serde(default, rename = "inlineData")]
    inline_data: Option<GoogleInlineData>,
}

#[derive(Deserialize)]
struct GoogleInlineData {
    #[serde(default, rename = "mimeType")]
    mime_type: Option<String>,
    #[serde(default)]
    data: Option<String>,
}

#[derive(Clone, Default, Deserialize)]
struct GoogleImageUsage {
    #[serde(default, rename = "promptTokenCount")]
    prompt_token_count: u64,
    #[serde(default, rename = "candidatesTokenCount")]
    candidates_token_count: u64,
    #[serde(default, rename = "totalTokenCount")]
    total_token_count: Option<u64>,
    #[serde(default, rename = "cachedContentTokenCount")]
    cached_content_token_count: u64,
    #[serde(default, rename = "thoughtsTokenCount")]
    thoughts_token_count: u64,
}

pub(super) fn parse_response(
    context: &ErrorContext,
    headers: &JsonHttpHeaders,
    model_id: &str,
    body: Value,
) -> ImageGenerationResult<ImageGenerationResponse> {
    let Some(status) = embedded_status(&body) else {
        return parse_image(context, model_id, body);
    };
    match parse_image(context, model_id, body.clone()) {
        Ok(response) => Ok(response),
        Err(error) => {
            let classified = classify_status(context, status, headers, &body);
            match error {
                ImageGenerationError::Internal { source, .. } => {
                    Err(ImageGenerationError::Internal {
                        core: Box::new(classified.core().clone()),
                        source,
                    })
                }
                _ => Err(classified),
            }
        }
    }
}

fn parse_image(
    context: &ErrorContext,
    model_id: &str,
    body: Value,
) -> ImageGenerationResult<ImageGenerationResponse> {
    let parsed = match serde_json::from_value::<GoogleImageResponse>(body) {
        Ok(parsed) => parsed,
        Err(source) => {
            return Err(ImageGenerationError::with_source(
                context.core(ErrorKind::InvalidOutput, "invalid provider image response"),
                source,
            ));
        }
    };
    if let Some(error) = prompt_feedback_error(context, parsed.prompt_feedback.as_ref()) {
        return Err(error);
    }
    let usage = parsed.usage_metadata.unwrap_or_default();
    let mut provider_failure = None;
    for candidate in parsed.candidates {
        if let Some(error) = candidate_finish_error(context, &candidate) {
            match error {
                ImageGenerationError::NoImage { .. } => continue,
                error if error.core().kind == ErrorKind::ContentPolicy => return Err(error),
                _ => {
                    if provider_failure.is_none() {
                        provider_failure = Some(error);
                    }
                    continue;
                }
            }
        }
        for part in candidate.content.unwrap_or_default().parts {
            if part.thought {
                continue;
            }
            let Some(inline_data) = part.inline_data else {
                continue;
            };
            let Some(encoded) = inline_data.data else {
                return Err(ImageGenerationError::new(
                    context.core(ErrorKind::InvalidOutput, "Google image part omitted data"),
                ));
            };
            let Some(mime_type) = inline_data.mime_type else {
                return Err(ImageGenerationError::new(context.core(
                    ErrorKind::InvalidOutput,
                    "Google image part omitted MIME type",
                )));
            };
            let data = match STANDARD.decode(encoded) {
                Ok(data) => data,
                Err(source) => {
                    return Err(ImageGenerationError::with_source(
                        context.core(ErrorKind::InvalidOutput, "invalid provider image response"),
                        source,
                    ));
                }
            };
            if data.is_empty() {
                return Err(ImageGenerationError::NoImage {
                    core: Box::new(context.core(ErrorKind::InvalidOutput, "no image returned")),
                });
            }
            return Ok(ImageGenerationResponse {
                provider: PROVIDER.to_owned(),
                model_id: model_id.to_owned(),
                image: GeneratedImage { data, mime_type },
                revised_prompt: None,
                usage: normalize_usage(usage),
            });
        }
    }
    if let Some(error) = provider_failure {
        return Err(error);
    }
    Err(ImageGenerationError::NoImage {
        core: Box::new(context.core(ErrorKind::InvalidOutput, "no image returned")),
    })
}

fn prompt_feedback_error(
    context: &ErrorContext,
    feedback: Option<&GooglePromptFeedback>,
) -> Option<ImageGenerationError> {
    let feedback = feedback?;
    let reason = feedback.block_reason.as_deref()?;
    let message = feedback
        .block_reason_message
        .as_deref()
        .unwrap_or(reason)
        .to_owned();
    if matches!(
        reason,
        "SAFETY" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "IMAGE_SAFETY"
    ) {
        return Some(ImageGenerationError::new(
            context.core(ErrorKind::ContentPolicy, message),
        ));
    }
    Some(ImageGenerationError::new(
        context.core(ErrorKind::InvalidOutput, message),
    ))
}

fn candidate_finish_error(
    context: &ErrorContext,
    candidate: &GoogleImageCandidate,
) -> Option<ImageGenerationError> {
    let reason = candidate.finish_reason.as_deref()?;
    if matches!(reason, "STOP" | "FINISH_REASON_UNSPECIFIED") {
        return None;
    }
    if reason == "NO_IMAGE" {
        return Some(ImageGenerationError::NoImage {
            core: Box::new(context.core(ErrorKind::InvalidOutput, "no image returned")),
        });
    }
    let message = candidate
        .finish_message
        .as_deref()
        .unwrap_or(reason)
        .to_owned();
    if is_policy_finish_reason(reason) {
        return Some(ImageGenerationError::new(
            context.core(ErrorKind::ContentPolicy, message),
        ));
    }
    Some(ImageGenerationError::new(
        context.core(ErrorKind::InvalidOutput, message),
    ))
}

fn is_policy_finish_reason(reason: &str) -> bool {
    matches!(
        reason,
        "SAFETY"
            | "RECITATION"
            | "BLOCKLIST"
            | "PROHIBITED_CONTENT"
            | "SPII"
            | "IMAGE_SAFETY"
            | "IMAGE_PROHIBITED_CONTENT"
            | "IMAGE_RECITATION"
            | "ESCALATION"
    )
}

fn normalize_usage(usage: GoogleImageUsage) -> ModelUsage {
    let input_tokens = usage
        .prompt_token_count
        .saturating_sub(usage.cached_content_token_count);
    let total_tokens = usage.total_token_count.unwrap_or_else(|| {
        input_tokens
            .saturating_add(usage.cached_content_token_count)
            .saturating_add(usage.candidates_token_count)
            .saturating_add(usage.thoughts_token_count)
    });
    ModelUsage {
        input_tokens,
        output_tokens: usage.candidates_token_count,
        cached_input_tokens: usage.cached_content_token_count,
        reasoning_tokens: usage.thoughts_token_count,
        total_tokens,
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    }
}
