//! Ordered conversation-part mapping to the Responses input protocol.

use ai_interface::{ConversationMessage, ConversationPart, ConversationRole, ModelResult};
use ai_models_core::ErrorContext;

use crate::PROVIDER_ID;

use super::{
    replay_state::{FunctionCallState, ReplayState},
    request_content::message_content,
    request_types::{
        ResponsesFunctionCallInput, ResponsesFunctionCallOutput, ResponsesInputItem,
        ResponsesMessage, ResponsesMessageContent, ResponsesReasoningInput,
    },
};

pub(super) fn input_items(
    context: &ErrorContext,
    model_id: &str,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<ResponsesInputItem>> {
    let mut items = Vec::new();
    for message in messages {
        if let Err(error) = message.validate("openai", model_id) {
            return Err(context.apply_model_error(error));
        }
        let states = message
            .parts
            .iter()
            .map(|part| match part.provider_state() {
                Some(state) => match state.decode::<ReplayState>(&PROVIDER_ID, model_id, 1) {
                    Ok(state) => Ok(state),
                    Err(error) => Err(context.apply_model_error(error)),
                },
                None => Ok(None),
            })
            .collect::<ModelResult<Vec<_>>>()?;
        match message.role {
            ConversationRole::User => items.push(ResponsesInputItem::Message(ResponsesMessage {
                role: "user".to_owned(),
                phase: None,
                content: message_content(context, &message.parts)?,
            })),
            ConversationRole::Assistant => {
                let mut assistant = Vec::new();
                assistant_items(&mut assistant, &message.parts, states);
                items.extend(assistant);
            }
            ConversationRole::Tool => {
                for part in &message.parts {
                    if let ConversationPart::ToolResult {
                        call_id, content, ..
                    } = part
                    {
                        items.push(ResponsesInputItem::FunctionCallOutput(
                            ResponsesFunctionCallOutput {
                                kind: "function_call_output".to_owned(),
                                call_id: call_id.clone(),
                                output: content.clone(),
                            },
                        ));
                    }
                }
            }
        }
    }
    Ok(items)
}

fn assistant_items(
    items: &mut Vec<ResponsesInputItem>,
    parts: &[ConversationPart],
    states: Vec<Option<ReplayState>>,
) {
    let mut carried_phase = None;
    for (part, state) in parts.iter().zip(states) {
        match state {
            Some(ReplayState::Skip(_)) => continue,
            Some(ReplayState::Reasoning(state)) => {
                items.push(ResponsesInputItem::Reasoning(ResponsesReasoningInput {
                    kind: "reasoning".to_owned(),
                    id: state.id,
                    summary: state.summary,
                    encrypted_content: state.encrypted_content,
                }));
                continue;
            }
            Some(ReplayState::FunctionCall(state)) => {
                items.push(ResponsesInputItem::FunctionCall(function_call(state)));
                continue;
            }
            Some(ReplayState::Message(state)) => {
                if let ConversationPart::Text { text, .. } = part {
                    append_text(items, text, state.phase);
                    continue;
                }
                if let Some(ResponsesInputItem::Message(message)) = items
                    .iter_mut()
                    .find(|item| matches!(item, ResponsesInputItem::Message(_)))
                {
                    message.phase = state.phase;
                } else {
                    carried_phase = Some(state.phase);
                }
            }
            None => {}
        }
        match part {
            ConversationPart::Text { text, .. } => {
                append_text(items, text, carried_phase.take().flatten())
            }
            ConversationPart::ToolCall {
                call_id,
                tool_name,
                arguments,
                ..
            } => items.push(ResponsesInputItem::FunctionCall(function_call(
                FunctionCallState {
                    id: None,
                    call_id: call_id.clone(),
                    name: tool_name.clone(),
                    arguments: arguments.to_string(),
                },
            ))),
            _ => {}
        }
    }
}

fn append_text(items: &mut Vec<ResponsesInputItem>, text: &str, phase: Option<String>) {
    if let Some(ResponsesInputItem::Message(ResponsesMessage {
        role,
        phase: previous_phase,
        content: ResponsesMessageContent::Text(previous),
    })) = items.last_mut()
        && role == "assistant"
        && (*previous_phase == phase || phase.is_none())
    {
        previous.push_str(text);
        return;
    }
    items.push(ResponsesInputItem::Message(ResponsesMessage {
        role: "assistant".to_owned(),
        phase,
        content: ResponsesMessageContent::Text(text.to_owned()),
    }));
}

fn function_call(state: FunctionCallState) -> ResponsesFunctionCallInput {
    ResponsesFunctionCallInput {
        kind: "function_call".to_owned(),
        id: state.id,
        call_id: state.call_id,
        name: state.name,
        arguments: state.arguments,
    }
}
