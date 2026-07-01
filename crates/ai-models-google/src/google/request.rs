//! Google Gemini `generateContent` request mapping.

use ai_interface::{
    ConversationContentPart, ConversationMessage, ConversationRole, ModelRequest, ToolDefinition,
};
use ai_models_core::ThinkingLevel;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub(super) struct GoogleRequest {
    #[serde(rename = "systemInstruction")]
    system_instruction: GoogleInstruction,
    contents: Vec<GoogleContent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GoogleToolGroup>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    generation_config: Option<GoogleGenerationConfig>,
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
    parameters: Value,
}

#[derive(Debug, Serialize)]
struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseMimeType")]
    response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseJsonSchema")]
    response_json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingConfig")]
    thinking_config: Option<GoogleThinkingConfig>,
}

#[derive(Debug, Serialize)]
struct GoogleThinkingConfig {
    #[serde(rename = "thinkingBudget")]
    thinking_budget: i32,
}

pub(super) fn build_request(
    request: &ModelRequest,
    thinking_level: ThinkingLevel,
) -> GoogleRequest {
    let generation_config = generation_config(request, thinking_level);

    GoogleRequest {
        system_instruction: GoogleInstruction {
            parts: vec![GooglePart {
                text: Some(request.system_prompt.clone()),
                function_call: None,
                function_response: None,
                inline_data: None,
            }],
        },
        contents: google_contents(&request.messages),
        tools: if request.tools.is_empty() {
            Vec::new()
        } else {
            vec![GoogleToolGroup {
                function_declarations: request.tools.iter().map(tool).collect(),
            }]
        },
        generation_config,
    }
}

fn generation_config(
    request: &ModelRequest,
    thinking_level: ThinkingLevel,
) -> Option<GoogleGenerationConfig> {
    let response_schema = request
        .response_schema
        .as_ref()
        .map(|response_schema| response_schema.schema.clone());
    let thinking_config = thinking_budget(thinking_level)
        .map(|thinking_budget| GoogleThinkingConfig { thinking_budget });
    if response_schema.is_none() && thinking_config.is_none() {
        return None;
    }
    Some(GoogleGenerationConfig {
        response_mime_type: response_schema
            .as_ref()
            .map(|_| "application/json".to_owned()),
        response_json_schema: response_schema,
        thinking_config,
    })
}

fn thinking_budget(thinking_level: ThinkingLevel) -> Option<i32> {
    match thinking_level {
        ThinkingLevel::Disabled => None,
        ThinkingLevel::Low => Some(1024),
        ThinkingLevel::Medium => Some(4096),
        ThinkingLevel::High => Some(8192),
        ThinkingLevel::ExtraHigh => Some(16_384),
        ThinkingLevel::Max => Some(32_768),
    }
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
        parameters: tool.input_schema.clone(),
    }
}
