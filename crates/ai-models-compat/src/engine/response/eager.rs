//! Typed eager tool-call decoding for response dialects that require it.

use ai_interface::MiniMaxReasoningDetail;
use serde::Deserialize;
use serde_json::Value;

use super::types::{
    BaseResponse, ChatCompletionsAssistantMessage, ChatCompletionsChoice, ChatCompletionsResponse,
    ChatCompletionsToolCall,
};

#[derive(Deserialize)]
struct EagerResponse {
    #[serde(default)]
    choices: Vec<EagerChoice>,
    #[serde(default)]
    usage: Option<Value>,
    #[serde(default)]
    base_resp: Option<BaseResponse>,
}

#[derive(Deserialize)]
struct EagerChoice {
    message: EagerMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct EagerMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatCompletionsToolCall>,
    #[serde(default)]
    reasoning_content: Option<String>,
    #[serde(default)]
    reasoning_details: Vec<MiniMaxReasoningDetail>,
}

pub(super) fn decode(body: Value) -> Result<ChatCompletionsResponse, serde_json::Error> {
    let raw: EagerResponse = serde_json::from_value(body)?;
    Ok(ChatCompletionsResponse {
        choices: raw
            .choices
            .into_iter()
            .map(|choice| ChatCompletionsChoice {
                finish_reason: choice.finish_reason,
                message: ChatCompletionsAssistantMessage {
                    content: choice.message.content,
                    reasoning_content: choice.message.reasoning_content,
                    reasoning_details: choice.message.reasoning_details,
                    tool_calls: None,
                    parsed_tool_calls: Some(choice.message.tool_calls),
                    function_call: None,
                },
            })
            .collect(),
        usage: raw.usage,
        base_resp: raw.base_resp,
    })
}
