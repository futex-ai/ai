//! xAI endpoint, compatible wire forms, and deferred defaults.

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, Sampling, StructuredOutput, ThinkingFormat,
    ToolChoiceRule, UsageFields,
};
use std::time::Duration;

pub(super) static DIALECT: Dialect = Dialect {
    provider: "xai",
    label: "xAI",
    endpoint: "https://api.x.ai/v1/chat/completions",
    usage: UsageFields::BoundedCachedAndReasoning,
    replay: ReplayRule::Xai,
    thinking: ThinkingFormat::GrokEffort {
        fixed_prefix: "grok-4.20",
    },
    output_limit: OutputLimit::MaxTokens,
    structured_output: StructuredOutput::JsonSchema,
    sampling: Sampling::Any,
    tool_choice: ToolChoiceRule::Standard,
    input: InputParts::TextImage,
    names: true,
    required_reasoning: false,
    transient_finish: None,
    http_error: None,
    parallel_tools: false,
    clean_eof: false,
    normalizer: None,
    base_error: None,
    deferred_timeout: Some(Duration::from_secs(60)),
};
