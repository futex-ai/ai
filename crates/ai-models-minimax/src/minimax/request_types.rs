//! MiniMax chat-completions request DTOs.

use ai_interface::MiniMaxReasoningDetail;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsRequest {
    pub(super) model: String,
    pub(super) messages: Vec<ChatCompletionsMessage>,
    pub(super) stream: bool,
    pub(super) stream_options: ChatCompletionsStreamOptions,
    pub(super) reasoning_split: bool,
    pub(super) thinking: ThinkingControl,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tools: Vec<ChatCompletionsTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) max_completion_tokens: Option<u32>,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsStreamOptions {
    pub(super) include_usage: bool,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsMessage {
    pub(super) role: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) content: Option<ChatCompletionsContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tool_calls: Vec<ChatCompletionsToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) reasoning_details: Vec<MiniMaxReasoningDetail>,
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
    #[serde(rename = "video_url")]
    VideoUrl { video_url: ChatCompletionsVideoUrl },
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsImageUrl {
    pub(super) url: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsVideoUrl {
    pub(super) url: String,
}

#[derive(Debug, Serialize)]
pub(super) struct ThinkingControl {
    #[serde(rename = "type")]
    pub(super) kind: &'static str,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsToolCall {
    pub(super) id: String,
    #[serde(rename = "type")]
    pub(super) kind: &'static str,
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
    pub(super) kind: &'static str,
    pub(super) function: ChatCompletionsToolDefinition,
}

#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionsToolDefinition {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) parameters: Value,
}
