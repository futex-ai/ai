//! DeepSeek endpoint and wire settings.

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, StructuredOutput, ThinkingFormat, ToolChoiceWire,
    UsageFields,
};

pub(super) static DIALECT: Dialect = Dialect {
    provider: "deepseek",
    label: "DeepSeek",
    endpoint: "https://api.deepseek.com/chat/completions",
    usage: UsageFields::CacheHitMiss,
    replay: ReplayRule::DeepSeek,
    thinking: ThinkingFormat::ToggleEffort,
    output_limit: OutputLimit::MaxTokens,
    structured_output: StructuredOutput::JsonObjectPrompt,
    tool_choice: ToolChoiceWire::OmitWhileThinking,
    input: InputParts::PlainText,
    deferred_timeout: None,
    clean_eof: false,
    normalizer: None,
    base_error: None,
    names: true,
    parallel_tools: false,
    http_error: None,
    transient_finish: Some((
        "insufficient_system_resource",
        "DeepSeek reported insufficient system resources",
    )),
    required_reasoning: true,
};
