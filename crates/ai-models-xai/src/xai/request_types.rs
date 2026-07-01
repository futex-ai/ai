//! xAI chat-completions request DTOs.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsRequest {
    pub(super) model: String,
    pub(super) messages: Vec<ChatCompletionsMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tools: Vec<ChatCompletionsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) response_format: Option<ChatCompletionsResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reasoning_effort: Option<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsMessage {
    pub(super) role: String,
    pub(super) content: Option<ChatCompletionsContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tool_calls: Vec<ChatCompletionsToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) function_call: Option<ChatCompletionsToolFunction>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum ChatCompletionsContent {
    Text(String),
    Parts(Vec<ChatCompletionsContentPart>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub(super) enum ChatCompletionsContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ChatCompletionsImageUrl },
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsImageUrl {
    pub(super) url: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsToolCall {
    pub(super) id: String,
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) function: ChatCompletionsToolFunction,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsTool {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) function: ChatCompletionsToolDefinition,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsToolDefinition {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) parameters: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsToolFunction {
    pub(super) name: String,
    pub(super) arguments: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsResponseFormat {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) json_schema: ChatCompletionsJsonSchema,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsJsonSchema {
    pub(super) name: String,
    pub(super) schema: Value,
    pub(super) strict: bool,
}
