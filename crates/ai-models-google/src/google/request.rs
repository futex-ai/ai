//! Google Gemini `generateContent` request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde::Serialize;
use serde_json::Value;

use super::thinking::{GoogleThinkingConfig, thinking_config};
use super::tool_config::{GoogleToolConfig, tool_config};

#[derive(Debug, Serialize)]
pub(super) struct GoogleRequest {
    #[serde(rename = "systemInstruction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GoogleInstruction>,
    contents: Vec<GoogleContent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GoogleToolGroup>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    generation_config: Option<GoogleGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "toolConfig")]
    tool_config: Option<GoogleToolConfig>,
}

#[derive(Debug, Serialize)]
struct GoogleInstruction {
    parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize)]
struct GoogleContent {
    role: String,
    parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize)]
struct GooglePart {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    function_call: Option<GoogleFunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    function_response: Option<GoogleFunctionResponse>,
    #[serde(rename = "inlineData", skip_serializing_if = "Option::is_none")]
    inline_data: Option<GoogleInlineData>,
}

#[derive(Debug, Serialize)]
struct GoogleInlineData {
    #[serde(rename = "mimeType")]
    mime_type: String,
    data: String,
}

#[derive(Debug, Serialize)]
struct GoogleFunctionCall {
    id: String,
    name: String,
    args: Value,
}

#[derive(Debug, Serialize)]
struct GoogleFunctionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    response: GoogleFunctionResult,
}

#[derive(Debug, Serialize)]
struct GoogleFunctionResult {
    result: String,
}

#[derive(Debug, Serialize)]
struct GoogleToolGroup {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<GoogleFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GoogleFunctionDeclaration {
    name: String,
    description: String,
    #[serde(rename = "parametersJsonSchema")]
    parameters_json_schema: Value,
}

#[derive(Debug, Serialize)]
struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseMimeType")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseJsonSchema")]
    response_json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingConfig")]
    thinking_config: Option<GoogleThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topP")]
    top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "stopSequences")]
    stop_sequences: Vec<String>,
}

pub(super) fn build_request(
    model_id: &str,
    request: &ModelRequest,
    thinking_level: ThinkingLevel,
) -> GoogleRequest {
    let generation_config = generation_config(model_id, request, thinking_level);

    GoogleRequest {
        system_instruction: (!request.system_prompt.is_empty()).then(|| GoogleInstruction {
            parts: vec![GooglePart {
                text: Some(request.system_prompt.clone()),
                function_call: None,
                function_response: None,
                inline_data: None,
            }],
        }),
        contents: google_contents(&request.messages),
        tools: if request.tools.is_empty() {
            Vec::new()
        } else {
            vec![GoogleToolGroup {
                function_declarations: request.tools.iter().map(tool).collect(),
            }]
        },
        generation_config,
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

    for message in messages {
        match message.role {
            ConversationRole::User => append_parts(&mut output, "user", user_parts(message)),
            ConversationRole::Assistant => {
                let mut parts = Vec::new();
                if !message.content.is_empty() {
                    parts.push(GooglePart {
                        text: Some(message.content.clone()),
                        function_call: None,
                        function_response: None,
                        inline_data: None,
                    });
                }
                parts.extend(message.tool_calls.iter().map(|call| GooglePart {
                    text: None,
                    function_call: Some(GoogleFunctionCall {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        args: call.input.clone(),
                    }),
                    function_response: None,
                    inline_data: None,
                }));
                append_parts(&mut output, "model", parts);
            }
            ConversationRole::Tool => append_parts(
                &mut output,
                "user",
                vec![GooglePart {
                    text: None,
                    function_call: None,
                    function_response: Some(GoogleFunctionResponse {
                        id: message.tool_call_id.clone(),
                        name: message.name.clone(),
                        response: GoogleFunctionResult {
                            result: message.content.clone(),
                        },
                    }),
                    inline_data: None,
                }],
            ),
        }
    }

    output
}

fn user_parts(message: &ConversationMessage) -> Vec<GooglePart> {
    if message.content_parts.is_empty() {
        return vec![GooglePart {
            text: Some(message.content.clone()),
            function_call: None,
            function_response: None,
            inline_data: None,
        }];
    }
    message.content_parts.iter().map(content_part).collect()
}

fn content_part(part: &ConversationContentPart) -> GooglePart {
    match part {
        ConversationContentPart::Text { text } => GooglePart {
            text: Some(text.clone()),
            function_call: None,
            function_response: None,
            inline_data: None,
        },
        ConversationContentPart::Image {
            mime_type,
            data_base64,
        } => GooglePart {
            text: None,
            function_call: None,
            function_response: None,
            inline_data: Some(GoogleInlineData {
                mime_type: mime_type.clone(),
                data: data_base64.clone(),
            }),
        },
    }
}

fn append_parts(contents: &mut Vec<GoogleContent>, role: &str, parts: Vec<GooglePart>) {
    if parts.is_empty() {
        return;
    }
    if let Some(existing) = contents.last_mut()
        && existing.role == role
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
