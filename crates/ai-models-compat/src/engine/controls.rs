//! Provider wire forms for tool choices that the shared checker resolved.

use ai_interface::Effort;
use ai_models_core::{ResolvedModelControls, ResolvedToolChoice};

use super::{
    dialect::{Dialect, ToolChoiceWire},
    request_types::{
        ChatCompletionsNamedFunction, ChatCompletionsNamedToolChoice, ChatCompletionsToolChoice,
    },
};

pub(super) fn tool_choice(
    dialect: &Dialect,
    has_tools: bool,
    controls: &ResolvedModelControls,
) -> Option<ChatCompletionsToolChoice> {
    let choice = controls.tool_choice.as_ref();
    if matches!(dialect.tool_choice, ToolChoiceWire::OmitWhileThinking)
        && controls.effort.is_none_or(Effort::is_enabled)
        && matches!(
            choice,
            None | Some(ResolvedToolChoice::None | ResolvedToolChoice::Auto)
        )
    {
        return None;
    }
    let mode = match choice {
        Some(ResolvedToolChoice::None) => "none",
        Some(ResolvedToolChoice::Auto) => "auto",
        Some(ResolvedToolChoice::Required) => "required",
        Some(ResolvedToolChoice::Function(name)) => {
            return Some(ChatCompletionsToolChoice::Function(
                ChatCompletionsNamedToolChoice {
                    kind: "function",
                    function: ChatCompletionsNamedFunction { name: name.clone() },
                },
            ));
        }
        None if has_tools && matches!(dialect.tool_choice, ToolChoiceWire::AutoWhenTools) => "auto",
        None => return None,
    };
    Some(ChatCompletionsToolChoice::Mode(mode.to_owned()))
}
