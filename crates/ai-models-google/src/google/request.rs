//! Google Gemini request and continuation mapping.

use ai_interface::{ModelRequest, ModelResult, ToolDefinition};
use ai_models_core::ResolvedModelControls;

use super::{
    contents::{google_contents, text_part},
    request_types::{
        GoogleFunctionDeclaration, GoogleGenerationConfig, GoogleInstruction, GoogleRequest,
        GoogleToolGroup,
    },
    thinking::thinking_config,
    tool_config::tool_config,
};

pub(super) fn build_request(
    model_id: &str,
    request: &ModelRequest,
    controls: &ResolvedModelControls,
) -> ModelResult<GoogleRequest> {
    request.validate_messages("google", model_id)?;
    Ok(GoogleRequest {
        system_instruction: request
            .nonblank_system_prompt()
            .map(|text| GoogleInstruction {
                parts: vec![text_part(text)],
            }),
        contents: google_contents(&request.messages, model_id)?,
        tools: if request.tools.is_empty() {
            Vec::new()
        } else {
            vec![GoogleToolGroup {
                function_declarations: request.tools.iter().map(tool).collect(),
            }]
        },
        generation_config: generation_config(model_id, request, controls),
        tool_config: tool_config(controls.tool_choice.as_ref()),
    })
}

fn generation_config(
    model_id: &str,
    request: &ModelRequest,
    controls: &ResolvedModelControls,
) -> Option<GoogleGenerationConfig> {
    let response_schema = request
        .response_schema
        .as_ref()
        .map(|response_schema| response_schema.schema.clone());
    let thinking_config = thinking_config(model_id, controls.effort);
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

fn tool(tool: &ToolDefinition) -> GoogleFunctionDeclaration {
    GoogleFunctionDeclaration {
        name: tool.name.clone(),
        description: tool.description.clone(),
        parameters_json_schema: tool.input_schema.clone(),
    }
}
