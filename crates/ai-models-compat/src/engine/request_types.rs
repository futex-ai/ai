//! Shared Chat Completions request DTOs.

use serde::Serialize;
use serde_json::Value;

use crate::minimax::state::ReasoningDetail;

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsRequest {
    pub(crate) model: String,
    pub(crate) messages: Vec<ChatCompletionsMessage>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tools: Vec<ChatCompletionsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_choice: Option<ChatCompletionsToolChoice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) parallel_tool_calls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) enable_thinking: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) preserve_thinking: Option<bool>,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) stream: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub(crate) deferred: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stream_options: Option<ChatCompletionsStreamOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) thinking: Option<ChatCompletionsThinking>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_split: Option<bool>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) stop: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsStreamOptions {
    pub(crate) include_usage: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ChatCompletionsToolChoice {
    Mode(String),
    Function(ChatCompletionsNamedToolChoice),
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsNamedToolChoice {
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    pub(crate) function: ChatCompletionsNamedFunction,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsNamedFunction {
    pub(crate) name: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsMessage {
    pub(crate) role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// The outer absence omits content; the inner absence sends JSON null.
    pub(crate) content: Option<Option<ChatCompletionsContent>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) reasoning_details: Vec<ReasoningDetail>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) tool_calls: Vec<ChatCompletionsToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) function_call: Option<ChatCompletionsToolFunction>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub(crate) enum ChatCompletionsContent {
    Text(String),
    Parts(Vec<ChatCompletionsContentPart>),
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum ChatCompletionsContentPart {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image_url")]
    ImageUrl { image_url: ChatCompletionsImageUrl },
    #[serde(rename = "video_url")]
    VideoUrl { video_url: ChatCompletionsImageUrl },
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsImageUrl {
    pub(crate) url: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsThinking {
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsToolCall {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    pub(crate) function: ChatCompletionsToolFunction,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsToolFunction {
    pub(crate) name: String,
    pub(crate) arguments: String,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsTool {
    #[serde(rename = "type")]
    pub(crate) kind: &'static str,
    pub(crate) function: ChatCompletionsToolDefinition,
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsToolDefinition {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) parameters: Value,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub(crate) enum ChatCompletionsResponseFormat {
    #[serde(rename = "json_object")]
    JsonObject,
    #[serde(rename = "json_schema")]
    JsonSchema {
        json_schema: ChatCompletionsJsonSchema,
    },
}

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ChatCompletionsJsonSchema {
    pub(crate) name: String,
    pub(crate) schema: Value,
    pub(crate) strict: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}
