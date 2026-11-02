//! Ordered portable parts and private Gemini continuation mapping.

use std::collections::BTreeMap;

use ai_interface::{ConversationMessage, ConversationPart, ConversationRole, ModelResult};

use super::{
    request_types::{
        GoogleContent, GoogleFunctionResponse, GoogleFunctionResult, GoogleInlineData, GooglePart,
    },
    state::{GoogleAssistantPart, GoogleFunctionCallContext, GooglePartState},
};

pub(super) fn google_contents(
    messages: &[ConversationMessage],
    model_id: &str,
) -> ModelResult<Vec<GoogleContent>> {
    let mut output = Vec::new();
    let mut sent_ids = BTreeMap::new();
    for message in messages {
        let states = message
            .parts
            .iter()
            .map(|part| match part.provider_state() {
                Some(state) => GooglePartState::decode(state, model_id),
                None => Ok(None),
            })
            .collect::<ModelResult<Vec<_>>>()?;
        let signed = states.iter().flatten().any(GooglePartState::has_signature);
        let mut parts = Vec::new();
        for (part, state) in message.parts.iter().zip(states) {
            if let Some(state) = state {
                let replay = if !signed && let Some(call) = state.unsigned_call {
                    vec![GoogleAssistantPart {
                        function_call: Some(call),
                        ..GoogleAssistantPart::default()
                    }]
                } else {
                    state.parts
                };
                if let ConversationPart::ToolCall { call_id, .. } = part {
                    let id = replay
                        .iter()
                        .find_map(|part| part.function_call.as_ref())
                        .and_then(|call| call.id.clone());
                    sent_ids.insert(call_id.clone(), id);
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
        append_parts(&mut output, role, parts);
    }
    Ok(output)
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
