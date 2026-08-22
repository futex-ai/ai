//! QwenCloud Chat Completions request DTOs.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsRequest {
    pub(super) model: String,
    pub(super) messages: Vec<ChatCompletionsMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tools: Vec<ChatCompletionsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<ChatCompletionsToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parallel_tool_calls: Option<bool>,
    pub(super) stream: bool,
    pub(super) stream_options: ChatCompletionsStreamOptions,
    pub(super) enable_thinking: bool,
    pub(super) preserve_thinking: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) response_format: Option<ChatCompletionsResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) stop: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsStreamOptions {
    pub(super) include_usage: bool,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum ChatCompletionsToolChoice {
    Mode(String),
    Function(ChatCompletionsNamedToolChoice),
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsNamedToolChoice {
    #[serde(rename = "type")]
    pub(super) kind: String,
    pub(super) function: ChatCompletionsNamedFunction,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsNamedFunction {
    pub(super) name: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsMessage {
    pub(super) role: String,
    pub(super) content: Option<ChatCompletionsContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tool_calls: Vec<ChatCompletionsToolCall>,
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
pub(super) struct ChatCompletionsToolFunction {
    pub(super) name: String,
    pub(super) arguments: String,
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
pub(super) struct ChatCompletionsResponseFormat {
    #[serde(rename = "type")]
    pub(super) kind: String,
}
