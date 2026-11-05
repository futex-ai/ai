//! Conversation content, participant names, and role mapping.

use ai_interface::{ConversationMessage, ConversationRole, ModelError, ModelRequest, ModelResult};

use super::{
    dialect::{Dialect, InputParts, ReplayRule},
    message_content::message_content,
    request_legacy::{LegacyFunctionCall, legacy_function_call, legacy_tool_name},
    request_replay::{reasoning_context, replay_message},
    request_types::{ChatCompletionsContent, ChatCompletionsMessage, ChatCompletionsToolFunction},
    tools::tool_call,
};

pub(super) fn system_message(content: String) -> ChatCompletionsMessage {
    ChatCompletionsMessage {
        role: "system",
        content: Some(Some(ChatCompletionsContent::Text(content))),
        name: None,
        tool_call_id: None,
        reasoning_content: None,
        reasoning_details: Vec::new(),
        tool_calls: Vec::new(),
        function_call: None,
    }
}

pub(super) fn validate_content_parts(
    dialect: &Dialect,
    model_id: &str,
    request: &ModelRequest,
) -> ModelResult<()> {
    if matches!(dialect.input, InputParts::PlainText)
        && request
            .messages
            .iter()
            .any(|message| !message.content_parts.is_empty())
    {
        return Err(ModelError::provider(
            dialect.provider,
            model_id,
            format!("{} accepts plain text messages only", dialect.label),
        ));
    }
    if let InputParts::UserTextImage { plain_text_model } = dialect.input {
        for message in &request.messages {
            if message.content_parts.is_empty() {
                continue;
            }
            if model_id == plain_text_model {
                return Err(ModelError::provider(
                    dialect.provider,
                    model_id,
                    format!("{model_id} accepts plain text messages only"),
                ));
            }
            if message.role != ConversationRole::User {
                return Err(ModelError::provider(
                    dialect.provider,
                    model_id,
                    format!(
                        "{} accepts typed content parts only on user messages",
                        dialect.label
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn message(
    dialect: &Dialect,
    model_id: &str,
    message: &ConversationMessage,
    legacy_calls: &[LegacyFunctionCall],
) -> ModelResult<ChatCompletionsMessage> {
    if message.role == ConversationRole::Assistant
        && let Some(replay) = replay_message(dialect.replay, message)
    {
        return Ok(replay);
    }
    let legacy_call = (dialect.replay == ReplayRule::Xai)
        .then(|| legacy_function_call(message))
        .flatten();
    let legacy_name = if dialect.replay == ReplayRule::Xai {
        legacy_tool_name(message, legacy_calls)
    } else {
        None
    };
    let (reasoning_content, reasoning_details) = reasoning_context(dialect.replay, message);
    let content = message_content(dialect, model_id, message)?;
    let content = if dialect.replay == ReplayRule::MiniMax {
        content.map(Some)
    } else {
        Some(content)
    };
    Ok(ChatCompletionsMessage {
        role: if legacy_name.is_some() {
            "function"
        } else {
            match message.role {
                ConversationRole::User => "user",
                ConversationRole::Assistant => "assistant",
                ConversationRole::Tool => "tool",
            }
        },
        content,
        name: if let Some(name) = legacy_name {
            Some(name.to_owned())
        } else if dialect.names && message.role != ConversationRole::Tool {
            message.name.clone()
        } else {
            None
        },
        tool_call_id: if legacy_name.is_some() {
            None
        } else if message.role == ConversationRole::Tool
            || matches!(dialect.replay, ReplayRule::Kimi | ReplayRule::Xai)
        {
            message.tool_call_id.clone()
        } else {
            None
        },
        reasoning_content,
        reasoning_details,
        tool_calls: if legacy_call.is_none()
            && (message.role == ConversationRole::Assistant
                || matches!(dialect.replay, ReplayRule::MiniMax | ReplayRule::Xai))
        {
            message.tool_calls.iter().map(tool_call).collect()
        } else {
            Vec::new()
        },
        function_call: legacy_call.map(|call| ChatCompletionsToolFunction {
            name: call.name,
            arguments: call.arguments,
        }),
    })
}

pub(super) fn conversation_messages(
    dialect: &Dialect,
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<ChatCompletionsMessage>> {
    let mut legacy_calls = Vec::new();
    let mut output = Vec::new();
    for input in messages {
        output.push(message(dialect, model_id, input, &legacy_calls)?);
        if dialect.replay == ReplayRule::Xai
            && let Some(call) = legacy_function_call(input)
        {
            legacy_calls.push(call);
        }
    }
    Ok(output)
}
