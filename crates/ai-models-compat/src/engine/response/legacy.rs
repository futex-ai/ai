//! Legacy function-call response decoding and deterministic identity.

use ai_interface::ProviderConversationItem;
use ai_models_core::synthetic_tool_call_id;
use serde::Deserialize;
use serde_json::Value;

use super::{
    super::dialect::CompletionContext,
    types::{
        ChatCompletionsAssistantMessage, ChatCompletionsChoice, ChatCompletionsResponse,
        ChatCompletionsToolCall, ChatCompletionsToolFunction,
    },
};

#[derive(Deserialize)]
struct LegacyResponse {
    choices: Vec<LegacyChoice>,
    #[serde(default)]
    usage: Option<Value>,
}

#[derive(Deserialize)]
struct LegacyChoice {
    message: LegacyMessage,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct LegacyMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatCompletionsToolCall>,
    #[serde(default)]
    function_call: Option<ChatCompletionsToolFunction>,
}

pub(super) fn decode(body: Value) -> Result<ChatCompletionsResponse, serde_json::Error> {
    let raw: LegacyResponse = serde_json::from_value(body)?;
    Ok(ChatCompletionsResponse {
        choices: raw
            .choices
            .into_iter()
            .map(|choice| ChatCompletionsChoice {
                finish_reason: choice.finish_reason,
                message: ChatCompletionsAssistantMessage {
                    content: choice.message.content,
                    reasoning_content: None,
                    reasoning_details: Vec::new(),
                    tool_calls: None,
                    parsed_tool_calls: Some(choice.message.tool_calls),
                    function_call: choice.message.function_call,
                },
            })
            .collect(),
        usage: raw.usage,
        base_resp: None,
    })
}

pub(super) fn normalize(
    context: CompletionContext<'_>,
    calls: &mut Option<Vec<ChatCompletionsToolCall>>,
    function: Option<ChatCompletionsToolFunction>,
) -> Vec<ProviderConversationItem> {
    let Some(calls) = calls.as_mut().filter(|calls| calls.is_empty()) else {
        return Vec::new();
    };
    let Some(function) = function else {
        return Vec::new();
    };
    let id = synthetic_tool_call_id(
        "xai_legacy_function_call:",
        context.synthetic_tool_call_scope,
        0,
        &function.name,
        &function.arguments,
    );
    let replay = ProviderConversationItem::XaiLegacyFunctionCall {
        tool_call_id: id.clone(),
        name: function.name.clone(),
        arguments: function.arguments.clone(),
    };
    calls.push(ChatCompletionsToolCall {
        id: id.clone(),
        function,
        operation_id: Some(id),
    });
    vec![replay]
}
