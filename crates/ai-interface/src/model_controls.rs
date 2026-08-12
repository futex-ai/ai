//! Portable generation and execution controls for model calls.

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Portable control whose support is decided by a provider adapter.
pub enum ModelControl {
    /// Sampling temperature.
    Temperature,
    /// Nucleus-sampling probability.
    TopP,
    /// Maximum generated-token count.
    MaxOutputTokens,
    /// Ordered output stop sequences.
    StopSequences,
    /// Tool-selection behavior.
    ToolChoice,
    /// Native completion lifecycle.
    CompletionMode,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
/// Portable tool-selection intent.
pub enum ModelToolChoice {
    /// Prevent the model from calling a tool.
    None,
    /// Let the model decide whether and which tool to call.
    Auto,
    /// Require the model to call one or more available tools.
    Required,
    /// Require one named function.
    Function(String),
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
/// Optional generation controls applied by a provider adapter.
pub struct ModelGenerationControls {
    /// Sampling temperature, when supported by the selected model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    /// Nucleus-sampling probability, when supported by the selected model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    /// Caller-requested maximum generated-token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    /// Ordered sequences that stop generation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop_sequences: Vec<String>,
    /// Caller-requested tool-selection behavior.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ModelToolChoice>,
}

impl ModelGenerationControls {
    /// Returns whether every generation control is absent.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Portable preference for the provider's native completion lifecycle.
pub enum ModelCompletionMode {
    /// Use the adapter's ordinary immediate request lifecycle.
    #[default]
    Synchronous,
    /// Prefer a native deferred lifecycle and otherwise call synchronously.
    PreferDeferred,
    /// Require a native deferred lifecycle.
    RequireDeferred,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Execution controls for one provider-adapter invocation.
pub struct ModelExecutionControls {
    /// Total duration available to the adapter invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_timeout: Option<Duration>,
    /// Preferred provider completion lifecycle.
    #[serde(default, skip_serializing_if = "is_synchronous")]
    pub completion_mode: ModelCompletionMode,
}

impl ModelExecutionControls {
    /// Returns whether every execution control has its default value.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }

    /// Resolves whether an adapter should use its native deferred lifecycle.
    pub fn resolve_deferred(&self, supports_deferred: bool) -> Result<bool, ModelControl> {
        match self.completion_mode {
            ModelCompletionMode::Synchronous => Ok(false),
            ModelCompletionMode::PreferDeferred => Ok(supports_deferred),
            ModelCompletionMode::RequireDeferred if supports_deferred => Ok(true),
            ModelCompletionMode::RequireDeferred => Err(ModelControl::CompletionMode),
        }
    }
}

fn is_synchronous(mode: &ModelCompletionMode) -> bool {
    *mode == ModelCompletionMode::Synchronous
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
/// Portable controls supplied with a model request.
pub struct ModelCallControls {
    /// Optional provider-normalized generation controls.
    #[serde(default, skip_serializing_if = "ModelGenerationControls::is_default")]
    pub generation: ModelGenerationControls,
    /// Optional provider-normalized execution controls.
    #[serde(default, skip_serializing_if = "ModelExecutionControls::is_default")]
    pub execution: ModelExecutionControls,
}

impl ModelCallControls {
    /// Returns whether all controls have their default values.
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}
