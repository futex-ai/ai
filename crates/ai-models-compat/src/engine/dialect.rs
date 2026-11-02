//! Dialect settings and invocation identity shared by the engine stages.

use ai_models_core::ThinkingLevel;

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
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum ReplayRule {
    DeepSeek,
    Kimi,
}

#[derive(Clone, Copy)]
pub(crate) enum ThinkingFormat {
    ToggleEffort,
    Effort,
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
}

#[derive(Clone, Copy)]
pub(crate) enum InputParts {
    PlainText,
    TextImage,
}

#[derive(Clone, Copy)]
pub(crate) struct CompletionContext<'a> {
    pub(crate) dialect: &'a Dialect,
    pub(crate) catalog_model_id: &'a str,
    pub(crate) provider_model_id: &'a str,
    pub(crate) thinking_level: ThinkingLevel,
}
