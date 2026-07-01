//! OpenAI Responses request DTOs.

use ai_interface::OpenAiReasoningSummary;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct ResponsesRequest {
    pub(super) model: String,
    pub(super) instructions: String,
    pub(super) input: Vec<ResponsesInputItem>,
    pub(super) store: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) include: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tools: Vec<ResponsesTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) text: Option<ResponsesText>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reasoning: Option<ResponsesReasoning>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum ResponsesInputItem {
    Message(ResponsesMessage),
    Reasoning(ResponsesReasoningInput),
    FunctionCall(ResponsesFunctionCallInput),
    FunctionCallOutput(ResponsesFunctionCallOutput),
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesMessage {
    pub(super) role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) phase: Option<String>,
    pub(super) content: ResponsesMessageContent,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum ResponsesMessageContent {
    Text(String),
    Parts(Vec<ResponsesContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(super) enum ResponsesContentPart {
    #[serde(rename = "input_text")]
    InputText { text: String },
    #[serde(rename = "input_image")]
    InputImage { image_url: String },
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesFunctionCallInput {
    #[serde(rename = "type")]
    pub(super) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    pub(super) call_id: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesReasoningInput {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) id: String,
    pub(super) summary: Vec<OpenAiReasoningSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) encrypted_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesFunctionCallOutput {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) call_id: String,
    pub(super) output: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesTool {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) parameters: Value,
    pub(super) strict: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesText {
    pub(super) format: ResponsesTextFormat,
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesTextFormat {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) name: String,
    pub(super) strict: bool,
    pub(super) schema: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct ResponsesReasoning {
    pub(super) effort: String,
}
