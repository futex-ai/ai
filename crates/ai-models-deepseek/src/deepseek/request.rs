//! Shared request-to-DeepSeek Chat Completions mapping.

use ai_interface::{
    ConversationMessage, ConversationRole, DeepSeekToolCallContext, ModelError, ModelRequest,
    ModelResult, ProviderConversationItem, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::request_types::{
    ChatCompletionsMessage, ChatCompletionsRequest, ChatCompletionsResponseFormat,
    ChatCompletionsThinking, ChatCompletionsTool, ChatCompletionsToolCall,
    ChatCompletionsToolDefinition, ChatCompletionsToolFunction,
};

const PROVIDER: &str = "deepseek";

pub(super) fn build_request(
    model_id: &str,
    thinking_level: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<ChatCompletionsRequest> {
    validate_content_parts(model_id, request)?;
    let (thinking, reasoning_effort) = thinking_fields(model_id, thinking_level)?;
    let mut messages = vec![ChatCompletionsMessage {
        role: "system".to_owned(),
        content: system_prompt(request),
        name: None,
        tool_call_id: None,
        reasoning_content: None,
        tool_calls: Vec::new(),
    }];
    messages.extend(request.messages.iter().map(message));
    Ok(ChatCompletionsRequest {
        model: model_id.to_owned(),
        messages,
        tools: request.tools.iter().map(tool).collect(),
        stream: false,
        thinking,
        reasoning_effort,
        response_format: request
            .response_schema
            .as_ref()
            .map(|_| ChatCompletionsResponseFormat {
                kind: "json_object".to_owned(),
            }),
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

fn validate_content_parts(model_id: &str, request: &ModelRequest) -> ModelResult<()> {
    if request
        .messages
        .iter()
        .any(|message| !message.content_parts.is_empty())
    {
        return Err(ModelError::provider(
            PROVIDER,
            model_id,
            "DeepSeek accepts plain text messages only",
        ));
    }
    Ok(())
}

fn message(message: &ConversationMessage) -> ChatCompletionsMessage {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = deepseek_replay(message)
    {
        return replay_message(replay);
    }
    ChatCompletionsMessage {
        role: role(message.role).to_owned(),
        content: message.content.clone(),
        name: match message.role {
            ConversationRole::User | ConversationRole::Assistant => message.name.clone(),
            ConversationRole::Tool => None,
        },
        tool_call_id: match message.role {
            ConversationRole::Tool => message.tool_call_id.clone(),
            ConversationRole::User | ConversationRole::Assistant => None,
        },
        reasoning_content: None,
        tool_calls: if message.role == ConversationRole::Assistant {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
    }
}

fn deepseek_replay(message: &ConversationMessage) -> Option<&ProviderConversationItem> {
    message.provider_context.iter().find(|item| {
        matches!(
            item,
            ProviderConversationItem::DeepSeekAssistantMessage { .. }
        )
    })
}

fn replay_message(item: &ProviderConversationItem) -> ChatCompletionsMessage {
    let ProviderConversationItem::DeepSeekAssistantMessage {
        content,
        reasoning_content,
        tool_calls,
    } = item
    else {
        return ChatCompletionsMessage {
            role: "assistant".to_owned(),
            content: String::new(),
            name: None,
            tool_call_id: None,
            reasoning_content: None,
            tool_calls: Vec::new(),
        };
    };
    ChatCompletionsMessage {
        role: "assistant".to_owned(),
        content: content.clone(),
        name: None,
        tool_call_id: None,
        reasoning_content: reasoning_content.clone(),
        tool_calls: tool_calls.iter().map(raw_tool_call).collect(),
    }
}

fn role(role: ConversationRole) -> &'static str {
    match role {
        ConversationRole::User => "user",
        ConversationRole::Assistant => "assistant",
        ConversationRole::Tool => "tool",
    }
}

fn thinking_fields(
    model_id: &str,
    thinking_level: ThinkingLevel,
) -> ModelResult<(ChatCompletionsThinking, Option<String>)> {
    let (kind, reasoning_effort) = match thinking_level {
        ThinkingLevel::Disabled => ("disabled", None),
        ThinkingLevel::High => ("enabled", Some("high".to_owned())),
        ThinkingLevel::Max => ("enabled", Some("max".to_owned())),
        ThinkingLevel::Low | ThinkingLevel::Medium | ThinkingLevel::ExtraHigh => {
            return Err(ModelError::provider(
                PROVIDER,
                model_id,
                "unsupported DeepSeek thinking level",
            ));
        }
    };
    Ok((
        ChatCompletionsThinking {
            kind: kind.to_owned(),
        },
        reasoning_effort,
    ))
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

fn raw_tool_call(call: &DeepSeekToolCallContext) -> ChatCompletionsToolCall {
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
