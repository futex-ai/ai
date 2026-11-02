//! Kimi endpoint and wire settings.

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, Sampling, StructuredOutput, ThinkingFormat,
    ToolChoiceRule,
};

pub(super) static DIALECT: Dialect = Dialect {
    provider: "kimi",
    label: "Kimi",
    endpoint: "https://api.moonshot.ai/v1/chat/completions",
    replay: ReplayRule::Kimi,
    thinking: ThinkingFormat::Effort,
    output_limit: OutputLimit::MaxCompletionTokens,
    structured_output: StructuredOutput::JsonSchema,
    sampling: Sampling::Fixed,
    tool_choice: ToolChoiceRule::Standard,
    input: InputParts::TextImage,
    deferred_timeout: None,
    clean_eof: false,
    normalizer: None,
    base_error: None,
    names: true,
    parallel_tools: false,
    http_error: None,
    transient_finish: None,
    required_reasoning: false,
};
