//! OpenAI Responses API response DTOs.

use ai_interface::OpenAiReasoningSummary;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(in crate::openai) struct ResponsesResponse {
    #[serde(default)]
    pub(in crate::openai) status: Option<String>,
    #[serde(default)]
    pub(in crate::openai) output: Vec<ResponsesOutputItem>,
    #[serde(default)]
    pub(in crate::openai) usage: Option<ResponsesUsage>,
    #[serde(default)]
    pub(in crate::openai) incomplete_details: Option<ResponsesIncompleteDetails>,
    #[serde(default)]
    pub(in crate::openai) error: Option<ResponsesError>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub(in crate::openai) enum ResponsesOutputItem {
    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        phase: Option<String>,
        content: Vec<ResponsesContentPart>,
    },
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
pub(in crate::openai) enum ResponsesContentPart {
    #[serde(rename = "output_text")]
    OutputText { text: String },
    #[serde(rename = "refusal")]
    Refusal { refusal: String },
    #[serde(other)]
    Other,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(in crate::openai) struct ResponsesUsage {
    #[serde(default)]
    pub(in crate::openai) input_tokens: u64,
    #[serde(default)]
    pub(in crate::openai) output_tokens: u64,
    #[serde(default)]
    pub(in crate::openai) total_tokens: Option<u64>,
    #[serde(default)]
    pub(in crate::openai) input_tokens_details: ResponsesInputTokenDetails,
    #[serde(default)]
    pub(in crate::openai) output_tokens_details: ResponsesOutputTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(in crate::openai) struct ResponsesInputTokenDetails {
    #[serde(default)]
    pub(in crate::openai) cached_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(in crate::openai) struct ResponsesOutputTokenDetails {
    #[serde(default)]
    pub(in crate::openai) reasoning_tokens: u64,
}

#[derive(Debug, Deserialize)]
pub(in crate::openai) struct ResponsesIncompleteDetails {
    pub(in crate::openai) reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(in crate::openai) struct ResponsesError {
    pub(in crate::openai) message: Option<String>,
}
