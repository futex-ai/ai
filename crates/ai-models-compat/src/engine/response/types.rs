//! Typed shared response and tool-call wire fields.

use ai_interface::MiniMaxReasoningDetail;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsResponse {
    #[serde(default)]
    pub(super) choices: Vec<ChatCompletionsChoice>,
    #[serde(default)]
    pub(super) usage: Option<Value>,
    #[serde(skip)]
    pub(super) base_resp: Option<BaseResponse>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsChoice {
    pub(super) message: ChatCompletionsAssistantMessage,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsAssistantMessage {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
    #[serde(default)]
    pub(super) tool_calls: Option<Value>,
    #[serde(skip)]
    pub(super) parsed_tool_calls: Option<Vec<ChatCompletionsToolCall>>,
    #[serde(skip)]
    pub(super) reasoning_details: Vec<MiniMaxReasoningDetail>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsToolCall {
    pub(super) id: String,
    pub(super) function: ChatCompletionsToolFunction,
}

#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionsToolFunction {
    pub(super) name: String,
    pub(super) arguments: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BaseResponse {
    #[serde(default)]
    pub(crate) status_code: Option<i64>,
    #[serde(default)]
    pub(crate) status_msg: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct BaseResponseEnvelope {
    #[serde(default)]
    pub(super) base_resp: Option<BaseResponse>,
}
