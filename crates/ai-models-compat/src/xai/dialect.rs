//! xAI endpoint, compatible wire forms, and deferred defaults.

use std::time::Duration;

use crate::engine::dialect::{
    Dialect, InputParts, OutputLimit, ReplayRule, StructuredOutput, ThinkingFormat, ToolChoiceWire,
    UsageFields,
};

pub(super) static DIALECT: Dialect = Dialect {
    provider: "xai",
    request_id_headers: &["x-request-id", "request-id"],
    label: "xAI",
    base_url: "https://api.x.ai",
    path: "/v1/chat/completions",
    usage: UsageFields::BoundedCachedAndReasoning,
    replay: ReplayRule::Xai,
    thinking: ThinkingFormat::GrokEffort {
        fixed_prefix: "grok-4.20",
    },
    output_limit: OutputLimit::MaxTokens,
    structured_output: StructuredOutput::JsonSchema,
    tool_choice: ToolChoiceWire::AutoWhenTools {
        omit_without_tools: true,
    },
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
