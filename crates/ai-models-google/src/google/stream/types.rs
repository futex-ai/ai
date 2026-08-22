//! Typed Google generate-content stream chunks and failures.

use serde::Deserialize;
use serde_json::{Value, json};
use thiserror::Error;

#[derive(Debug, Deserialize)]
pub(super) struct GoogleStreamChunk {
    #[serde(default)]
    pub(super) candidates: Vec<GoogleStreamCandidate>,
    #[serde(default, rename = "promptFeedback")]
    pub(super) prompt_feedback: Option<GooglePromptFeedback>,
    #[serde(default, rename = "usageMetadata")]
    pub(super) usage_metadata: Option<GoogleUsageMetadata>,
    #[serde(default)]
    pub(super) error: Option<GoogleProviderError>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GoogleStreamCandidate {
    #[serde(default)]
    pub(super) content: Option<GoogleStreamContent>,
    #[serde(default, rename = "finishReason")]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GoogleStreamContent {
    #[serde(default)]
    pub(super) parts: Vec<GoogleStreamPart>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GoogleStreamPart {
    #[serde(default)]
    pub(super) text: Option<String>,
    #[serde(default)]
    pub(super) thought: Option<bool>,
    #[serde(default, rename = "functionCall")]
    pub(super) function_call: Option<GoogleFunctionCall>,
}

#[derive(Debug, Deserialize)]
pub(super) struct GoogleFunctionCall {
    #[serde(default)]
    pub(super) id: Option<String>,
    pub(super) name: String,
    #[serde(default)]
    pub(super) args: Option<Value>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct GooglePromptFeedback {
    #[serde(default, rename = "blockReason")]
    pub(super) block_reason: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub(super) struct GoogleUsageMetadata {
    #[serde(default, rename = "promptTokenCount")]
    pub(super) prompt_token_count: u32,
    #[serde(default, rename = "candidatesTokenCount")]
    pub(super) candidates_token_count: u32,
    #[serde(default, rename = "totalTokenCount")]
    pub(super) total_token_count: Option<u32>,
    #[serde(default, rename = "cachedContentTokenCount")]
    pub(super) cached_content_token_count: u32,
    #[serde(default, rename = "thoughtsTokenCount")]
    pub(super) thoughts_token_count: u32,
}

#[derive(Debug, Deserialize)]
pub(super) struct GoogleProviderError {
    pub(super) code: u16,
    pub(super) message: String,
    #[serde(default)]
    pub(super) status: Option<String>,
}

impl GoogleProviderError {
    pub(super) fn body(&self) -> Value {
        json!({
            "error": {
                "code": self.code,
                "message": self.message,
                "status": self.status
            }
        })
    }
}

#[derive(Debug)]
pub(super) enum GoogleStreamUpdate {
    Continue { deltas: Vec<GoogleStreamDelta> },
    ProviderError(GoogleProviderError),
}

#[derive(Debug)]
pub(super) enum GoogleStreamDelta {
    AssistantText { delta: String, starts_part: bool },
    ReasoningText { delta: String },
}

#[derive(Debug, Error)]
pub(super) enum GoogleStreamError {
    #[error("[ai_models_google/stream] invalid chunk JSON: {source}")]
    DeserializeChunk { source: serde_json::Error },
    #[error("[ai_models_google/stream] provider error HTTP {code} ({status:?}): {message}")]
    ProviderEvent {
        code: u16,
        status: Option<String>,
        message: String,
    },
    #[error("[ai_models_google/stream] response body ended before a terminal candidate")]
    MissingTerminal,
}
