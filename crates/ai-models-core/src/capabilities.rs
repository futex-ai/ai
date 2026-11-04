//! Pure capability resolution before a provider starts transport.

use std::time::Duration;

use ai_interface::{
    Effort, ForcedToolChoice, ModelControl, ModelError, ModelRequest, ModelResult, ModelSpec,
    ModelToolChoice, Sampling,
};

#[derive(Clone, Debug, Eq, PartialEq)]
/// Tool-selection behavior ready for provider serialization.
pub enum ResolvedToolChoice {
    /// Prevent the model from calling a tool.
    None,
    /// Let the model select a tool automatically.
    Auto,
    /// Require the model to call an available tool.
    Required,
    /// Require the named function.
    Function(String),
}

#[derive(Clone, Debug, PartialEq)]
/// Portable controls after every model capability rule has been applied.
pub struct ResolvedModelControls {
    /// Supported effort that never exceeds the request, or `None` for provider defaults.
    pub effort: Option<Effort>,
    /// Sampling temperature, absent when sampling is fixed.
    pub temperature: Option<f64>,
    /// Nucleus-sampling probability, absent when sampling is fixed.
    pub top_p: Option<f64>,
    /// Output-token limit after applying the model ceiling and required default.
    pub max_output_tokens: Option<u32>,
    /// Supported stop sequences in caller order.
    pub stop_sequences: Vec<String>,
    /// Tool-selection behavior without an unresolved fallback choice.
    pub tool_choice: Option<ResolvedToolChoice>,
    /// Total duration available to this adapter invocation.
    pub total_timeout: Option<Duration>,
    /// Whether the adapter must use its native deferred lifecycle.
    pub deferred: bool,
}

/// Resolves every portable control against one model spec without I/O.
pub fn resolve_controls(
    request: &ModelRequest,
    spec: &ModelSpec,
) -> ModelResult<ResolvedModelControls> {
    let generation = &request.controls.generation;
    let requested_effort = generation.effort.or(spec.default_effort);
    let effort = match requested_effort {
        Some(requested) => Some(
            spec.efforts
                .iter()
                .filter(|effort| *effort <= requested)
                .max()
                .ok_or_else(|| unsupported(spec, ModelControl::Effort))?,
        ),
        None => None,
    };
    let sampling_adjustable = match spec.sampling {
        Sampling::Adjustable => true,
        Sampling::FixedWhileThinking => effort == Some(Effort::Disabled),
        Sampling::Fixed => false,
    };
    if !spec.supports_stop_sequences && !generation.stop_sequences.is_empty() {
        return Err(unsupported(spec, ModelControl::StopSequences));
    }
    let tool_choice = resolve_tool_choice(generation.tool_choice.as_ref(), spec, effort)?;
    let deferred = match request
        .controls
        .execution
        .resolve_deferred(spec.supports_deferred)
    {
        Ok(deferred) => deferred,
        Err(control) => return Err(unsupported(spec, control)),
    };
    Ok(ResolvedModelControls {
        effort,
        temperature: generation.temperature.filter(|_| sampling_adjustable),
        top_p: generation.top_p.filter(|_| sampling_adjustable),
        max_output_tokens: generation
            .max_output_tokens
            .map(|limit| limit.min(spec.max_output_tokens))
            .or(spec.requires_output_limit.then_some(spec.max_output_tokens)),
        stop_sequences: generation.stop_sequences.clone(),
        tool_choice,
        total_timeout: request.controls.execution.total_timeout,
        deferred,
    })
}

fn resolve_tool_choice(
    choice: Option<&ModelToolChoice>,
    spec: &ModelSpec,
    effort: Option<Effort>,
) -> ModelResult<Option<ResolvedToolChoice>> {
    let required_allowed = match spec.forced_tool_choice {
        ForcedToolChoice::Always | ForcedToolChoice::RequiredOnly => true,
        ForcedToolChoice::OnlyWithoutThinking => effort == Some(Effort::Disabled),
        ForcedToolChoice::Never => false,
    };
    let function_allowed = match spec.forced_tool_choice {
        ForcedToolChoice::Always => true,
        ForcedToolChoice::OnlyWithoutThinking => effort == Some(Effort::Disabled),
        ForcedToolChoice::RequiredOnly | ForcedToolChoice::Never => false,
    };
    Ok(match choice {
        None => None,
        Some(ModelToolChoice::None) => Some(ResolvedToolChoice::None),
        Some(ModelToolChoice::Auto) => Some(ResolvedToolChoice::Auto),
        Some(ModelToolChoice::RequiredOrAuto) => Some(if required_allowed {
            ResolvedToolChoice::Required
        } else {
            ResolvedToolChoice::Auto
        }),
        Some(ModelToolChoice::Required) if required_allowed => Some(ResolvedToolChoice::Required),
        Some(ModelToolChoice::Function(name)) if function_allowed => {
            Some(ResolvedToolChoice::Function(name.clone()))
        }
        Some(ModelToolChoice::Required | ModelToolChoice::Function(_)) => {
            return Err(unsupported(spec, ModelControl::ToolChoice));
        }
    })
}

fn unsupported(spec: &ModelSpec, control: ModelControl) -> ModelError {
    ModelError::unsupported_control(
        spec.provider.as_str(),
        spec.provider_model_id.as_ref(),
        control,
    )
}

#[cfg(test)]
#[path = "_tests_/capabilities/mod.rs"]
mod capabilities_tests;
