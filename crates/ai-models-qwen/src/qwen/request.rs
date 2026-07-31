//! Shared request-to-QwenCloud Chat Completions mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelError, ModelRequest,
    ModelResult, ProviderConversationItem, QwenToolCallContext, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use crate::{QWEN_3_7_FLASH, QWEN_3_7_MAX, QWEN_3_7_PLUS};

use super::request_types::{
    ChatCompletionsContent, ChatCompletionsContentPart, ChatCompletionsImageUrl,
    ChatCompletionsMessage, ChatCompletionsRequest, ChatCompletionsResponseFormat,
    ChatCompletionsTool, ChatCompletionsToolCall, ChatCompletionsToolDefinition,
    ChatCompletionsToolFunction,
};

const PROVIDER: &str = "qwen";

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    validate_content_parts(model_id, request)?;
    let thinking_enabled = thinking_level.is_enabled();
    let mut messages = vec![ChatCompletionsMessage {
        role: "system".to_owned(),
        content: Some(ChatCompletionsContent::Text(system_prompt(request))),
        tool_call_id: None,
        reasoning_content: None,
        tool_calls: Vec::new(),
    }];
    messages.extend(request.messages.iter().map(message));
    let has_tools = !request.tools.is_empty();
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        tool_choice: has_tools.then(|| "auto".to_owned()),
        parallel_tool_calls: has_tools.then_some(true),
        stream: false,
        enable_thinking: thinking_enabled,
        preserve_thinking: thinking_enabled,
        response_format: native_json_format(model_id, thinking_level, request),
    })
}

fn system_prompt(request: &ModelRequest) -> String {
    let Some(schema) = request.response_schema.as_ref() else {
        return request.system_prompt.clone();
    };
    format!(
        "{}\n\nReturn raw JSON only. Do not use Markdown fences or additional prose. \
The response must be a JSON object matching schema `{}`.\nJSON Schema:\n{}",
        request.system_prompt, schema.name, schema.schema
    )
}

fn native_json_format(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> Option<ChatCompletionsResponseFormat> {
    let supports_native_json = matches!(model_id, QWEN_3_7_PLUS | QWEN_3_7_FLASH)
        && thinking_level == ThinkingLevel::Disabled;
    (request.response_schema.is_some() && supports_native_json).then(|| {
        ChatCompletionsResponseFormat {
            kind: "json_object".to_owned(),
        }
    })
}

fn validate_content_parts(model_id: &str, request: &ModelRequest) -> ModelResult<()> {
    for message in &request.messages {
        if message.content_parts.is_empty() {
            continue;
        }
        if model_id == QWEN_3_7_MAX {
            return Err(ModelError::provider(
                PROVIDER,
                model_id,
                "qwen3.7-max accepts plain text messages only",
            ));
        }
        if message.role != ConversationRole::User {
            return Err(ModelError::provider(
                PROVIDER,
                model_id,
                "Qwen accepts typed content parts only on user messages",
            ));
        }
    }
    Ok(())
}

fn message(message: &ConversationMessage) -> ChatCompletionsMessage {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = qwen_replay(message)
    {
        return replay_message(replay);
    }
    ChatCompletionsMessage {
        role: role(message.role).to_owned(),
        content: message_content(message),
        tool_call_id: (message.role == ConversationRole::Tool)
            .then(|| message.tool_call_id.clone())
            .flatten(),
        reasoning_content: None,
        tool_calls: if message.role == ConversationRole::Assistant {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
    }
}

fn qwen_replay(message: &ConversationMessage) -> Option<&ProviderConversationItem> {
    message
        .provider_context
        .iter()
        .find(|item| matches!(item, ProviderConversationItem::QwenAssistantMessage { .. }))
}

fn replay_message(item: &ProviderConversationItem) -> ChatCompletionsMessage {
    let ProviderConversationItem::QwenAssistantMessage {
        content,
        reasoning_content,
        tool_calls,
    } = item
    else {
        return empty_assistant_message();
    };
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: content.clone().map(ChatCompletionsContent::Text),
        tool_call_id: None,
        reasoning_content: reasoning_content.clone(),
        tool_calls: tool_calls.iter().map(raw_tool_call).collect(),
    }
}

fn empty_assistant_message() -> ChatCompletionsMessage {
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: None,
        tool_call_id: None,
        reasoning_content: None,
        tool_calls: Vec::new(),
    }
}

fn role(role: ConversationRole) -> &'static str {
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}

fn message_content(message: &ConversationMessage) -> Option<ChatCompletionsContent> {
    if !message.content_parts.is_empty() {
        return Some(ChatCompletionsContent::Parts(
            message.content_parts.iter().map(content_part).collect(),
        ));
    }
    if message.role == ConversationRole::Assistant && message.content.is_empty() {
        None
    } else {
        Some(ChatCompletionsContent::Text(message.content.clone()))
    }
}

fn content_part(part: &ConversationContentPart) -> ChatCompletionsContentPart {
    match part {
        ConversationContentPart::Text { text } => {
            ChatCompletionsContentPart::Text { text: text.clone() }
        }
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => ChatCompletionsContentPart::ImageUrl {
            image_url: ChatCompletionsImageUrl {
                url: format!("data:{mime_type};base64,{data_base64}"),
            },
        },
    }
}

fn tool_call(call: &ToolCall) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.input.to_string(),
        },
    }
}

fn raw_tool_call(call: &QwenToolCallContext) -> ChatCompletionsToolCall {
    ChatCompletionsToolCall {
        id: call.id.clone(),
        kind: "function".to_owned(),
        function: ChatCompletionsToolFunction {
            name: call.name.clone(),
            arguments: call.arguments.clone(),
        },
    }
}

fn tool(tool: &ToolDefinition) -> ChatCompletionsTool {
    ChatCompletionsTool {
        kind: "function".to_owned(),
        function: ChatCompletionsToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        },
    }
}
