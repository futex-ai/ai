//! DeepSeek endpoint and wire settings.

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, Sampling, StructuredOutput, ThinkingFormat,
    ToolChoiceRule,
};

pub(super) static DIALECT: Dialect = Dialect {
    provider: "deepseek",
    label: "DeepSeek",
    endpoint: "https://api.deepseek.com/chat/completions",
    replay: ReplayRule::DeepSeek,
    thinking: ThinkingFormat::ToggleEffort,
    output_limit: OutputLimit::MaxTokens,
    structured_output: StructuredOutput::JsonObjectPrompt,
    sampling: Sampling::WithoutThinking,
    tool_choice: ToolChoiceRule::OmitWhileThinking,
    input: InputParts::PlainText,
    names: true,
    parallel_tools: false,
    http_error: None,
    transient_finish: Some((
        "insufficient_system_resource",
        "DeepSeek reported insufficient system resources",
    )),
    required_reasoning: true,
};
