//! Dialect settings and invocation identity shared by the engine stages.

use ai_interface::ModelError;
use ai_models_core::ThinkingLevel;
use serde_json::Value;

pub(crate) struct Dialect {
    pub(crate) provider: &'static str,
    pub(crate) label: &'static str,
    pub(crate) endpoint: &'static str,
    pub(crate) replay: ReplayRule,
    pub(crate) thinking: ThinkingFormat,
    pub(crate) output_limit: OutputLimit,
    pub(crate) structured_output: StructuredOutput,
    pub(crate) sampling: Sampling,
    pub(crate) tool_choice: ToolChoiceRule,
    pub(crate) input: InputParts,
    pub(crate) names: bool,
    pub(crate) required_reasoning: bool,
    pub(crate) transient_finish: Option<(&'static str, &'static str)>,
    pub(crate) http_error: Option<fn(&str, u16, &Value) -> ModelError>,
    pub(crate) parallel_tools: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ReplayRule {
    DeepSeek,
    Kimi,
    Qwen,
}

#[derive(Clone, Copy)]
pub(crate) enum ThinkingFormat {
    ToggleEffort,
    Effort,
    Flags,
}

#[derive(Clone, Copy)]
pub(crate) enum OutputLimit {
    MaxTokens,
    MaxCompletionTokens,
}

#[derive(Clone, Copy)]
pub(crate) enum StructuredOutput {
    JsonObjectPrompt,
    JsonSchema,
    JsonObjectWhenDisabled(&'static [&'static str]),
}

#[derive(Clone, Copy)]
pub(crate) enum Sampling {
    WithoutThinking,
    Fixed,
}

#[derive(Clone, Copy)]
pub(crate) enum ToolChoiceRule {
    OmitWhileThinking,
    Standard,
    AutoWhileThinking,
}

#[derive(Clone, Copy)]
pub(crate) enum InputParts {
    PlainText,
    TextImage,
    UserTextImage { plain_text_model: &'static str },
}

#[derive(Clone, Copy)]
pub(crate) struct CompletionContext<'a> {
    pub(crate) dialect: &'a Dialect,
    pub(crate) catalog_model_id: &'a str,
    pub(crate) provider_model_id: &'a str,
    pub(crate) thinking_level: ThinkingLevel,
}
