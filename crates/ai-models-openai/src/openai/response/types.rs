//! OpenAI Responses API response DTOs.

use ai_interface::OpenAiReasoningSummary;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(super) struct ResponsesResponse {
    #[serde(default)]
    pub(super) status: Option<String>,
    #[serde(default)]
    pub(super) output: Vec<ResponsesOutputItem>,
    #[serde(default)]
    pub(super) usage: Option<ResponsesUsage>,
    #[serde(default)]
    pub(super) incomplete_details: Option<ResponsesIncompleteDetails>,
    #[serde(default)]
    pub(super) error: Option<ResponsesError>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum ResponsesOutputItem {
    #[serde(rename = "message")]
    Message { content: Vec<ResponsesContentPart> },
    #[serde(rename = "reasoning")]
    Reasoning {
        id: String,
        #[serde(default)]
        summary: Vec<OpenAiReasoningSummary>,
        #[serde(default)]
        encrypted_content: Option<String>,
    },
    #[serde(rename = "function_call")]
    FunctionCall {
        #[serde(default)]
        id: Option<String>,
        call_id: String,
        name: String,
        arguments: String,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(super) enum ResponsesContentPart {
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct ResponsesUsage {
    #[serde(default)]
    pub(super) input_tokens: u64,
    #[serde(default)]
    pub(super) output_tokens: u64,
    #[serde(default)]
    pub(super) total_tokens: Option<u64>,
    #[serde(default)]
    pub(super) input_tokens_details: ResponsesInputTokenDetails,
    #[serde(default)]
    pub(super) output_tokens_details: ResponsesOutputTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct ResponsesInputTokenDetails {
    #[serde(default)]
    pub(super) cached_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct ResponsesOutputTokenDetails {
    #[serde(default)]
    pub(super) reasoning_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub(super) struct ResponsesIncompleteDetails {
    pub(super) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ResponsesError {
    pub(super) message: Option<String>,
}
