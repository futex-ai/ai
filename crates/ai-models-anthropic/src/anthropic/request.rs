//! Anthropic messages request mapping.

use ai_interface::{Effort, ModelRequest, ModelResult, ToolDefinition};
use ai_models_core::{ErrorContext, ResolvedModelControls, ResolvedToolChoice};

use super::{
    request_messages::anthropic_messages,
    request_types::{
        AnthropicOutputConfig, AnthropicRequest, AnthropicThinking, AnthropicTool,
        AnthropicToolChoice,
    },
};

pub(super) fn build_request(
    context: &ErrorContext,
    model_id: &str,
    request: &ModelRequest,
    controls: &ResolvedModelControls,
) -> ModelResult<AnthropicRequest> {
    Ok(AnthropicRequest {
        stream: true,
        model: model_id.to_owned(),
        max_tokens: controls.max_output_tokens,
        system: system_prompt(request),
        messages: anthropic_messages(context, model_id, &request.messages)?,
        tools: request.tools.iter().map(tool).collect(),
        thinking: thinking(controls.effort),
        output_config: output_config(controls.effort),
        temperature: controls.temperature,
        top_p: controls.top_p,
        stop_sequences: controls.stop_sequences.clone(),
        tool_choice: tool_choice(controls.tool_choice.as_ref()),
    })
}

fn system_prompt(request: &ModelRequest) -> Option<String> {
    let Some(response_schema) = request.response_schema.as_ref() else {
        return request.nonblank_system_prompt().map(str::to_owned);
    };
    Some(format!(
        "{}\n\nWhen you are ready to provide the final answer, return raw JSON only with no markdown fences or extra prose. The JSON must match schema `{}` exactly.\nSchema: {}",
        request.system_prompt, response_schema.name, response_schema.schema
    ))
}

fn tool_choice(choice: Option<&ResolvedToolChoice>) -> Option<AnthropicToolChoice> {
    match choice {
        Some(ResolvedToolChoice::None) => Some(AnthropicToolChoice {
            kind: "none".to_owned(),
            name: None,
        }),
        Some(ResolvedToolChoice::Auto) => Some(AnthropicToolChoice {
            kind: "auto".to_owned(),
            name: None,
        }),
        Some(ResolvedToolChoice::Required) => Some(AnthropicToolChoice {
            kind: "any".to_owned(),
            name: None,
        }),
        Some(ResolvedToolChoice::Function(name)) => Some(AnthropicToolChoice {
            kind: "tool".to_owned(),
            name: Some(name.clone()),
        }),
        None => None,
    }
}

fn tool(tool: &ToolDefinition) -> AnthropicTool {
    AnthropicTool {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: tool.input_schema.clone(),
    }
}

fn thinking(thinking_level: Effort) -> Option<AnthropicThinking> {
    thinking_level.is_enabled().then(|| AnthropicThinking {
        kind: "adaptive".to_owned(),
        display: "omitted".to_owned(),
    })
}

fn output_config(thinking_level: Effort) -> Option<AnthropicOutputConfig> {
    effort(thinking_level).map(|effort| AnthropicOutputConfig {
        effort: effort.to_owned(),
    })
}

fn effort(thinking_level: Effort) -> Option<&'static str> {
    match thinking_level {
        Effort::Disabled => None,
        Effort::Low => Some("low"),
        Effort::Medium => Some("medium"),
        Effort::High => Some("high"),
        Effort::ExtraHigh => Some("xhigh"),
        Effort::Max => Some("max"),
    }
}
