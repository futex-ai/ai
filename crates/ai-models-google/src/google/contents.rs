//! Ordered portable parts and private Gemini continuation mapping.

use std::collections::BTreeMap;

use ai_interface::{ConversationMessage, ConversationPart, ConversationRole, ModelResult};
use ai_models_core::ErrorContext;

use super::{
    request_types::{
        GoogleContent, GoogleFunctionResponse, GoogleFunctionResult, GoogleInlineData, GooglePart,
    },
    state::{GoogleAssistantPart, GoogleFunctionCallContext, GooglePartState, GoogleStateScope},
};

pub(super) fn google_contents(
    context: &ErrorContext,
    messages: &[ConversationMessage],
) -> ModelResult<Vec<GoogleContent>> {
    let mut output = Vec::new();
    let mut sent_ids = BTreeMap::new();
    for message in messages {
        let states = message
            .parts
            .iter()
            .map(|part| match part.provider_state() {
                Some(state) => GooglePartState::decode(state, context),
                None => Ok(None),
            })
            .collect::<ModelResult<Vec<_>>>()?;
        if message.role == ConversationRole::Assistant
            && let Some(state) = states
                .iter()
                .flatten()
                .find(|state| state.scope == GoogleStateScope::Message)
        {
            replay_legacy_message(&mut output, &mut sent_ids, message, state);
            continue;
        }
        let signed = states.iter().flatten().any(GooglePartState::has_signature);
        let normalize_text = !signed
            && message.role == ConversationRole::Assistant
            && states
                .iter()
                .flatten()
                .any(|state| state.scope == GoogleStateScope::NormalizedText);
        let preserve_empty_assistant =
            message.role == ConversationRole::Assistant && states.iter().any(Option::is_some);
        let mut parts = Vec::new();
        if normalize_text {
            let text = message.text();
            if !text.is_empty() {
                parts.push(text_part(&text));
            }
        }
        for (part, state) in message.parts.iter().zip(states) {
            if normalize_text && matches!(part, ConversationPart::Text { .. }) {
                continue;
            }
            if let Some(state) = state {
                let replay = if !signed && let Some(call) = state.unsigned_call {
                    vec![GoogleAssistantPart {
                        function_call: Some(call),
                        ..GoogleAssistantPart::default()
                    }]
                } else {
                    state.parts
                };
                if let ConversationPart::ToolCall { call_id, .. } = part
                    && let Some(call) = replay.iter().find_map(|part| part.function_call.as_ref())
                {
                    sent_ids.insert(call_id.clone(), call.id.clone());
                }
                parts.extend(replay.into_iter().map(GooglePart::Assistant));
                continue;
            }
            match part {
                ConversationPart::Text { text, .. } => parts.push(text_part(text)),
                ConversationPart::Image {
                    mime_type,
                    data_base64,
                    ..
                }
                | ConversationPart::Video {
                    mime_type,
                    data_base64,
                    ..
                } => {
                    parts.push(GooglePart::InlineData {
                        inline_data: GoogleInlineData {
                            mime_type: mime_type.clone(),
                            data: data_base64.clone(),
                        },
                    });
                }
                ConversationPart::ToolCall {
                    call_id,
                    tool_name,
                    arguments,
                    ..
                } => {
                    sent_ids.insert(call_id.clone(), Some(call_id.clone()));
                    parts.push(GooglePart::Assistant(GoogleAssistantPart {
                        function_call: Some(GoogleFunctionCallContext {
                            id: Some(call_id.clone()),
                            name: tool_name.clone(),
                            args: Some(arguments.clone()),
                        }),
                        ..GoogleAssistantPart::default()
                    }));
                }
                ConversationPart::ToolResult {
                    call_id,
                    tool_name,
                    content,
                    ..
                } => {
                    parts.push(GooglePart::FunctionResponse {
                        function_response: GoogleFunctionResponse {
                            id: sent_ids
                                .get(call_id)
                                .cloned()
                                .unwrap_or_else(|| Some(call_id.clone())),
                            name: Some(tool_name.clone()),
                            response: GoogleFunctionResult {
                                result: content.clone(),
                            },
                        },
                    });
                }
                ConversationPart::Reasoning { .. } => {}
            }
        }
        if message.parts.is_empty() && message.role == ConversationRole::User {
            parts.push(text_part(""));
        }
        let role = match message.role {
            ConversationRole::Assistant => "model",
            ConversationRole::User | ConversationRole::Tool => "user",
        };
        if parts.is_empty() && preserve_empty_assistant {
            output.push(GoogleContent {
                role: role.to_owned(),
                parts,
            });
        } else {
            append_parts(&mut output, role, parts);
        }
    }
    Ok(output)
}

fn replay_legacy_message(
    output: &mut Vec<GoogleContent>,
    sent_ids: &mut BTreeMap<String, Option<String>>,
    message: &ConversationMessage,
    state: &GooglePartState,
) {
    let native_calls = state
        .parts
        .iter()
        .filter_map(|part| part.function_call.as_ref());
    sent_ids.extend(
        message
            .tool_calls()
            .into_iter()
            .zip(native_calls)
            .map(|(runtime, native)| (runtime.id, native.id.clone())),
    );
    output.push(GoogleContent {
        role: "model".to_owned(),
        parts: state
            .parts
            .iter()
            .cloned()
            .map(GooglePart::Assistant)
            .collect(),
    });
}

pub(super) fn text_part(text: &str) -> GooglePart {
    GooglePart::Assistant(GoogleAssistantPart {
        text: Some(text.to_owned()),
        ..GoogleAssistantPart::default()
    })
}

fn append_parts(contents: &mut Vec<GoogleContent>, role: &str, parts: Vec<GooglePart>) {
    if parts.is_empty() {
        return;
    }
    if let Some(existing) = contents.last_mut()
        && existing.role == role
        && role == "user"
    {
        existing.parts.extend(parts);
        return;
    }
    contents.push(GoogleContent {
        role: role.to_owned(),
        parts,
    });
}
