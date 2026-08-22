//! Typed OpenAI Responses stream events and failures.

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum OpenAiStreamEvent {
    #[serde(rename = "response.completed")]
    Completed { response: Value },
    #[serde(rename = "response.incomplete")]
    Incomplete { response: Value },
    #[serde(rename = "response.failed")]
    Failed { response: FailedResponse },
    #[serde(rename = "error")]
    Error(OpenAiNativeError),
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta { delta: String },
    #[serde(
        rename = "response.reasoning_summary_text.delta",
        alias = "response.reasoning_text.delta"
    )]
    ReasoningTextDelta { delta: String },
    #[serde(other)]
    Progress,
}

#[derive(Debug, Deserialize)]
pub(super) struct FailedResponse {
    #[serde(default)]
    error: Option<OpenAiResponseError>,
}

impl FailedResponse {
    pub(super) fn into_failure(self) -> OpenAiStreamError {
        let error = self.error.unwrap_or_default();
        OpenAiStreamError::ResponseFailed {
            code: error.code,
            message: error
                .message
                .unwrap_or_else(|| "OpenAI response failed".to_owned()),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiResponseError {
    code: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpenAiNativeError {
    pub(super) code: Option<String>,
    pub(super) message: String,
    pub(super) param: Option<String>,
}

impl OpenAiNativeError {
    pub(super) fn into_failure(self) -> OpenAiStreamError {
        OpenAiStreamError::NativeError {
            code: self.code,
            message: self.message,
            param: self.param,
        }
    }
}

#[derive(Debug, Error)]
pub(super) enum OpenAiStreamError {
    #[error("[ai_models_openai/stream] invalid event JSON: {source}")]
    DeserializeEvent { source: serde_json::Error },
    #[error("[ai_models_openai/stream] response failed ({code:?}): {message}")]
    ResponseFailed {
        code: Option<String>,
        message: String,
    },
    #[error("[ai_models_openai/stream] native error ({code:?}, parameter {param:?}): {message}")]
    NativeError {
        code: Option<String>,
        message: String,
        param: Option<String>,
    },
    #[error("[ai_models_openai/stream] response body ended before a terminal event")]
    UnexpectedEof,
}

impl OpenAiStreamError {
    pub(super) fn code(&self) -> Option<&str> {
        match self {
            Self::ResponseFailed { code, .. } | Self::NativeError { code, .. } => code.as_deref(),
            Self::DeserializeEvent { .. } | Self::UnexpectedEof => None,
        }
    }

    pub(super) fn provider_message(self) -> Option<String> {
        match self {
            Self::ResponseFailed { message, .. } | Self::NativeError { message, .. } => {
                Some(message)
            }
            Self::DeserializeEvent { .. } | Self::UnexpectedEof => None,
        }
    }
}
