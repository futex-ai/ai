//! Shared Chat Completions request DTOs.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatCompletionsMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tools: Vec<ChatCompletionsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<ChatCompletionsToolChoice>,
    pub(crate) stream: bool,
    pub(crate) stream_options: ChatCompletionsStreamOptions,
    pub(crate) thinking: ChatCompletionsThinking,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_effort: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) response_format: Option<ChatCompletionsResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) stop: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsStreamOptions {
    pub(crate) include_usage: bool,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ChatCompletionsToolChoice {
    Mode(String),
    Function(ChatCompletionsNamedToolChoice),
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsNamedToolChoice {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: ChatCompletionsNamedFunction,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsNamedFunction {
    pub(crate) name: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsMessage {
    pub(crate) role: String,
    pub(crate) content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tool_calls: Vec<ChatCompletionsToolCall>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsThinking {
    #[serde(rename = "type")]
    pub(crate) kind: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsToolCall {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: ChatCompletionsToolFunction,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsToolFunction {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsTool {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: ChatCompletionsToolDefinition,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsToolDefinition {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) parameters: Value,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionsResponseFormat {
    #[serde(rename = "type")]
    pub(crate) kind: String,
}
