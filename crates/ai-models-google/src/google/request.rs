//! Google Gemini request and continuation mapping.

use std::collections::BTreeMap;

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, GoogleAssistantPart,
    GoogleFunctionCallContext, ModelRequest, ProviderConversationItem, ToolCall, ToolDefinition,
};
use ai_models_core::ThinkingLevel;

use super::{
    request_types::{
        GoogleContent, GoogleFunctionDeclaration, GoogleFunctionResponse, GoogleFunctionResult,
        GoogleGenerationConfig, GoogleInlineData, GoogleInstruction, GooglePart, GoogleRequest,
        GoogleToolGroup,
    },
    thinking::thinking_config,
    tool_config::tool_config,
};

pub(super) fn build_request(
    model_id: &str,
    request: &ModelRequest,
    thinking_level: ThinkingLevel,
) -> GoogleRequest {
    GoogleRequest {
        system_instruction: request
            .nonblank_system_prompt()
            .map(|text| GoogleInstruction {
                parts: vec![text_part(text)],
            }),
        contents: google_contents(&request.messages),
        tools: if request.tools.is_empty() {
            Vec::new()
        } else {
            vec![GoogleToolGroup {
                function_declarations: request.tools.iter().map(tool).collect(),
            }]
        },
        generation_config: generation_config(model_id, request, thinking_level),
        tool_config: tool_config(request.controls.generation.tool_choice.as_ref()),
    }
}

fn generation_config(
    model_id: &str,
    request: &ModelRequest,
    thinking_level: ThinkingLevel,
) -> Option<GoogleGenerationConfig> {
    let response_schema = request
        .response_schema
        .as_ref()
        .map(|response_schema| response_schema.schema.clone());
    let thinking_config = thinking_config(model_id, thinking_level);
    let controls = &request.controls.generation;
    if response_schema.is_none()
        && thinking_config.is_none()
        && controls.temperature.is_none()
        && controls.top_p.is_none()
        && controls.max_output_tokens.is_none()
        && controls.stop_sequences.is_empty()
    {
        return None;
    }
    Some(GoogleGenerationConfig {
        response_mime_type: response_schema
            .as_ref()
            .map(|_| "application/json".to_owned()),
        response_json_schema: response_schema,
        thinking_config,
        temperature: controls.temperature,
        top_p: controls.top_p,
        max_output_tokens: controls.max_output_tokens,
        stop_sequences: controls.stop_sequences.clone(),
    })
}

fn google_contents(messages: &[ConversationMessage]) -> Vec<GoogleContent> {
    let mut output = Vec::new();
    let mut sent_ids = BTreeMap::new();
    for message in messages {
        match message.role {
            ConversationRole::User => append_parts(&mut output, "user", user_parts(message)),
            ConversationRole::Assistant => {
                let replay_parts = message.provider_context.iter().find_map(|item| match item {
                    ProviderConversationItem::GoogleAssistantMessage { parts } => Some(parts),
                    _ => None,
                });
                sent_ids.extend(tool_call_ids(
                    &message.tool_calls,
                    replay_parts.map(Vec::as_slice),
                ));
                if let Some(parts) = replay_parts {
                    output.push(GoogleContent {
                        role: "model".to_owned(),
                        parts: parts.iter().cloned().map(GooglePart::Assistant).collect(),
                    });
                    continue;
                }
                let mut parts = Vec::new();
                if !message.content.is_empty() {
                    parts.push(text_part(&message.content));
                }
                parts.extend(message.tool_calls.iter().map(|call| {
                    GooglePart::Assistant(GoogleAssistantPart {
                        function_call: Some(GoogleFunctionCallContext {
                            id: sent_ids
                                .get(call.id.as_str())
                                .copied()
                                .flatten()
                                .map(str::to_owned),
                            name: call.name.clone(),
                            args: Some(call.input.clone()),
                        }),
                        ..GoogleAssistantPart::default()
                    })
                }));
                append_parts(&mut output, "model", parts);
            }
            ConversationRole::Tool => append_parts(
                &mut output,
                "user",
                vec![GooglePart::FunctionResponse {
                    function_response: GoogleFunctionResponse {
                        id: message
                            .tool_call_id
                            .as_deref()
                            .and_then(|id| sent_ids.get(id).copied().unwrap_or(Some(id)))
                            .map(str::to_owned),
                        name: message.name.clone(),
                        response: GoogleFunctionResult {
                            result: message.content.clone(),
                        },
                    },
                }],
            ),
        }
    }
    output
}

/// Maps runtime call ids to the optional ids sent in this assistant message.
fn tool_call_ids<'a>(
    calls: &'a [ToolCall],
    replay_parts: Option<&'a [GoogleAssistantPart]>,
) -> BTreeMap<&'a str, Option<&'a str>> {
    if let Some(parts) = replay_parts {
        let provider_calls = parts.iter().filter_map(|part| part.function_call.as_ref());
        calls
            .iter()
            .zip(provider_calls)
            .map(|(runtime, provider)| (runtime.id.as_str(), provider.id.as_deref()))
            .collect()
    } else {
        calls
            .iter()
            .map(|call| (call.id.as_str(), Some(call.id.as_str())))
            .collect()
    }
}

fn user_parts(message: &ConversationMessage) -> Vec<GooglePart> {
    if message.content_parts.is_empty() {
        return vec![text_part(&message.content)];
    }
    message.content_parts.iter().map(content_part).collect()
}

fn text_part(text: &str) -> GooglePart {
    GooglePart::Assistant(GoogleAssistantPart {
        text: Some(text.to_owned()),
        ..GoogleAssistantPart::default()
    })
}

fn content_part(part: &ConversationContentPart) -> GooglePart {
    match part {
        ConversationContentPart::Text { text } => text_part(text),
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        }
        | ConversationContentPart::Video {
            mime_type,
            data_base64,
        } => GooglePart::InlineData {
            inline_data: GoogleInlineData {
                mime_type: mime_type.clone(),
                data: data_base64.clone(),
            },
        },
    }
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

fn tool(tool: &ToolDefinition) -> GoogleFunctionDeclaration {
    GoogleFunctionDeclaration {
        name: tool.name.clone(),
        description: tool.description.clone(),
        parameters_json_schema: tool.input_schema.clone(),
    }
}
