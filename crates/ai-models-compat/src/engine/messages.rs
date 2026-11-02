//! Conversation content, participant names, and role mapping.

use ai_interface::{
    ConversationMessage, ConversationPart, ConversationRole, ModelError, ModelRequest, ModelResult,
};

use super::{
    dialect::{Dialect, InputParts, ReplayRule},
    message_content::message_content,
    request_legacy::{LegacyFunctionCall, legacy_tool_name},
    request_replay::decode_message,
    request_types::{ChatCompletionsContent, ChatCompletionsMessage, ChatCompletionsToolFunction},
    tools::{raw_tool_call, tool_call},
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
    request.validate_messages(dialect.provider, model_id)?;
    for message in &request.messages {
        for part in &message.parts {
            if !matches!(
                part,
                ConversationPart::Image { .. } | ConversationPart::Video { .. }
            ) {
                continue;
            }
            let error = match dialect.input {
                InputParts::PlainText => Some(format!(
                    "{} accepts plain text messages only",
                    dialect.label
                )),
                InputParts::UserTextImage { plain_text_model } if model_id == plain_text_model => {
                    Some(format!("{model_id} accepts plain text messages only"))
                }
                _ => None,
            };
            if let Some(error) = error {
                return Err(ModelError::provider(dialect.provider, model_id, error));
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
) -> ModelResult<(ChatCompletionsMessage, Option<LegacyFunctionCall>)> {
    let replay = decode_message(dialect, model_id, message)?;
    if message.role == ConversationRole::Assistant
        && let Some(snapshot) = replay.snapshot
    {
        return Ok((snapshot, None));
    }
    let legacy = if dialect.replay == ReplayRule::Xai {
        replay.legacy
    } else {
        None
    };
    let legacy_name = (dialect.replay == ReplayRule::Xai)
        .then(|| legacy_tool_name(message, legacy_calls))
        .flatten();
    let mut content = message_content(dialect, model_id, message)?;
    if message.role == ConversationRole::Assistant
        && let Some(raw) = replay.text
    {
        content = raw.map(ChatCompletionsContent::Text);
        if content.is_none()
            && dialect.replay == ReplayRule::Qwen
            && message.tool_calls().is_empty()
        {
            content = Some(ChatCompletionsContent::Text(String::new()));
        }
    }
    let content =
        if matches!(dialect.replay, ReplayRule::MiniMax | ReplayRule::Xai) && content.is_none() {
            None
        } else {
            Some(content)
        };
    let calls = if legacy.is_none() {
        message
            .tool_calls()
            .iter()
            .map(|call| {
                if matches!(
                    dialect.replay,
                    ReplayRule::DeepSeek | ReplayRule::Kimi | ReplayRule::Qwen
                ) && let Some((_, raw)) = replay.calls.iter().find(|(id, _)| *id == call.id)
                {
                    raw_tool_call(&raw.id, &raw.name, &raw.arguments)
                } else {
                    tool_call(call)
                }
            })
            .collect()
    } else {
        Vec::new()
    };
    Ok((
        ChatCompletionsMessage {
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
            tool_call_id: if legacy_name.is_none() {
                message.parts.iter().find_map(|part| match part {
                    ConversationPart::ToolResult { call_id, .. } => Some(call_id.clone()),
                    _ => None,
                })
            } else {
                None
            },
            reasoning_content: replay.reasoning_content,
            reasoning_details: replay.reasoning_details,
            tool_calls: calls,
            function_call: legacy.as_ref().map(|call| ChatCompletionsToolFunction {
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            }),
        },
        legacy,
    ))
}

pub(super) fn conversation_messages(
    dialect: &Dialect,
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<ChatCompletionsMessage>> {
    let mut legacy_calls = Vec::new();
    let mut output = Vec::new();
    for input in messages {
        let (mapped, legacy) = message(dialect, model_id, input, &legacy_calls)?;
        output.push(mapped);
        if let Some(call) = legacy {
            legacy_calls.push(call);
        }
    }
    Ok(output)
}
