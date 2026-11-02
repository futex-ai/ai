//! Portable tool-choice validation and wire forms.

use ai_interface::{ModelControl, ModelError, ModelRequest, ModelResult, ModelToolChoice};
use ai_models_core::ThinkingLevel;

use super::{
    dialect::{Dialect, ToolChoiceRule},
    request_types::{
        ChatCompletionsNamedFunction, ChatCompletionsNamedToolChoice, ChatCompletionsToolChoice,
    },
};

pub(super) fn validate_controls(
    dialect: &Dialect,
    model_id: &str,
    thinking: ThinkingLevel,
    request: &ModelRequest,
) -> ModelResult<()> {
    if let ToolChoiceRule::RequiredFor(required_model) = dialect.tool_choice {
        if !request.controls.generation.stop_sequences.is_empty() {
            return Err(ModelError::unsupported_control(
                dialect.provider,
                model_id,
                ModelControl::StopSequences,
            ));
        }
        if matches!(
            request.controls.generation.tool_choice,
            Some(ModelToolChoice::Function(_))
        ) || request.controls.generation.tool_choice == Some(ModelToolChoice::Required)
            && model_id != required_model
        {
            return Err(ModelError::unsupported_control(
                dialect.provider,
                model_id,
                ModelControl::ToolChoice,
            ));
        }
    }
    if matches!(
        dialect.tool_choice,
        ToolChoiceRule::OmitWhileThinking | ToolChoiceRule::AutoWhileThinking
    ) && thinking.is_enabled()
        && matches!(
            request.controls.generation.tool_choice,
            Some(ModelToolChoice::Required | ModelToolChoice::Function(_))
        )
    {
        return Err(ModelError::unsupported_control(
            dialect.provider,
            model_id,
            ModelControl::ToolChoice,
        ));
    }
    Ok(())
}

pub(super) fn tool_choice(
    dialect: &Dialect,
    model_id: &str,
    thinking: ThinkingLevel,
    request: &ModelRequest,
) -> Option<ChatCompletionsToolChoice> {
    if matches!(dialect.tool_choice, ToolChoiceRule::OmitWhileThinking) && thinking.is_enabled() {
        return None;
    }
    let mode = match request.controls.generation.tool_choice.as_ref() {
        Some(ModelToolChoice::None) => "none",
        Some(ModelToolChoice::Auto) => "auto",
        Some(ModelToolChoice::RequiredOrAuto) if matches!(dialect.tool_choice, ToolChoiceRule::RequiredFor(required_model) if required_model != model_id) => {
            "auto"
        }
        Some(ModelToolChoice::RequiredOrAuto)
            if thinking.is_enabled()
                && matches!(dialect.tool_choice, ToolChoiceRule::AutoWhileThinking) =>
        {
            "auto"
        }
        Some(ModelToolChoice::Required | ModelToolChoice::RequiredOrAuto) => "required",
        Some(ModelToolChoice::Function(name)) => {
            return Some(ChatCompletionsToolChoice::Function(
                ChatCompletionsNamedToolChoice {
                    kind: "function",
                    function: ChatCompletionsNamedFunction { name: name.clone() },
                },
            ));
        }
        None if !request.tools.is_empty()
            && matches!(
                dialect.tool_choice,
                ToolChoiceRule::Standard
                    | ToolChoiceRule::AutoWhileThinking
                    | ToolChoiceRule::RequiredFor(_)
            ) =>
        {
            "auto"
        }
        None => return None,
    };
    Some(ChatCompletionsToolChoice::Mode(mode.to_owned()))
}
