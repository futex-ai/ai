//! Typed Gemini request bodies.

use ai_interface::GoogleAssistantPart;
use serde::Serialize;
use serde_json::Value;

use super::{thinking::GoogleThinkingConfig, tool_config::GoogleToolConfig};

#[derive(Debug, Serialize)]
pub(super) struct GoogleRequest {
    #[serde(rename = "systemInstruction")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) system_instruction: Option<GoogleInstruction>,
    pub(super) contents: Vec<GoogleContent>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(super) tools: Vec<GoogleToolGroup>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "generationConfig")]
    pub(super) generation_config: Option<GoogleGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "toolConfig")]
    pub(super) tool_config: Option<GoogleToolConfig>,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleInstruction {
    pub(super) parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleContent {
    pub(super) role: String,
    pub(super) parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub(super) enum GooglePart {
    Assistant(GoogleAssistantPart),
    FunctionResponse {
        #[serde(rename = "functionResponse")]
        function_response: GoogleFunctionResponse,
    },
    InlineData {
        #[serde(rename = "inlineData")]
        inline_data: GoogleInlineData,
    },
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleInlineData {
    #[serde(rename = "mimeType")]
    pub(super) mime_type: String,
    pub(super) data: String,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleFunctionResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) name: Option<String>,
    pub(super) response: GoogleFunctionResult,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleFunctionResult {
    pub(super) result: String,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleToolGroup {
    #[serde(rename = "functionDeclarations")]
    pub(super) function_declarations: Vec<GoogleFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleFunctionDeclaration {
    pub(super) name: String,
    pub(super) description: String,
    #[serde(rename = "parametersJsonSchema")]
    pub(super) parameters_json_schema: Value,
}

#[derive(Debug, Serialize)]
pub(super) struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseMimeType")]
    pub(super) response_mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "responseJsonSchema")]
    pub(super) response_json_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "thinkingConfig")]
    pub(super) thinking_config: Option<GoogleThinkingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "topP")]
    pub(super) top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxOutputTokens")]
    pub(super) max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Vec::is_empty", rename = "stopSequences")]
    pub(super) stop_sequences: Vec<String>,
}
