//! Provider usage fields and non-overlapping token buckets.

use ai_interface::ModelUsage;
use serde::Deserialize;
use serde_json::Value;

use super::super::dialect::ReplayRule;

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct DeepSeekUsage {
    #[serde(default)]
    pub(super) prompt_tokens: u64,
    #[serde(default)]
    pub(super) prompt_cache_hit_tokens: u64,
    #[serde(default)]
    pub(super) prompt_cache_miss_tokens: Option<u64>,
    #[serde(default)]
    pub(super) completion_tokens: u64,
    #[serde(default)]
    pub(super) completion_tokens_details: ChatCompletionsCompletionTokenDetails,
    #[serde(default)]
    pub(super) total_tokens: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub(super) struct ChatCompletionsCompletionTokenDetails {
    #[serde(default)]
    pub(super) reasoning_tokens: u64,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct KimiUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: Option<u64>,
    #[serde(default)]
    cached_tokens: Option<u64>,
    #[serde(default)]
    prompt_tokens_details: PromptTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct QwenUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: Option<u64>,
    #[serde(default)]
    prompt_tokens_details: PromptTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct NativeUsage {
    #[serde(default)]
    prompt_tokens: u64,
    #[serde(default)]
    completion_tokens: u64,
    #[serde(default)]
    total_tokens: Option<u64>,
    #[serde(default)]
    prompt_tokens_details: PromptTokenDetails,
    #[serde(default)]
    completion_tokens_details: ChatCompletionsCompletionTokenDetails,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct PromptTokenDetails {
    #[serde(default)]
    cached_tokens: u64,
}

pub(super) fn normalize(
    rule: ReplayRule,
    raw: Option<Value>,
) -> Result<ModelUsage, serde_json::Error> {
    let (prompt, completion, cached, cache_miss, reasoning, total) = match rule {
        ReplayRule::DeepSeek => {
            let usage: DeepSeekUsage = match raw {
                Some(raw) => serde_json::from_value(raw)?,
                None => DeepSeekUsage::default(),
            };
            (
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.prompt_cache_hit_tokens,
                usage.prompt_cache_miss_tokens,
                usage.completion_tokens_details.reasoning_tokens,
                usage.total_tokens,
            )
        }
        ReplayRule::Kimi => {
            let usage: KimiUsage = match raw {
                Some(raw) => serde_json::from_value(raw)?,
                None => KimiUsage::default(),
            };
            (
                usage.prompt_tokens,
                usage.completion_tokens,
                usage
                    .cached_tokens
                    .unwrap_or(usage.prompt_tokens_details.cached_tokens),
                None,
                0,
                usage.total_tokens,
            )
        }
        ReplayRule::Qwen => {
            let usage: QwenUsage = match raw {
                Some(raw) => serde_json::from_value(raw)?,
                None => QwenUsage::default(),
            };
            (
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.prompt_tokens_details.cached_tokens,
                None,
                0,
                usage.total_tokens,
            )
        }
        ReplayRule::MiniMax => {
            let usage: NativeUsage = match raw {
                Some(raw) => serde_json::from_value(raw)?,
                None => NativeUsage::default(),
            };
            (
                usage.prompt_tokens,
                usage.completion_tokens,
                usage.prompt_tokens_details.cached_tokens,
                None,
                usage.completion_tokens_details.reasoning_tokens,
                usage.total_tokens,
            )
        }
    };
    let input_tokens = cache_miss.unwrap_or_else(|| prompt.saturating_sub(cached));
    let output_tokens = completion.saturating_sub(reasoning);
    Ok(ModelUsage {
        input_tokens,
        output_tokens,
        cached_input_tokens: cached,
        reasoning_tokens: reasoning,
        total_tokens: total.unwrap_or_else(|| {
            input_tokens
                .saturating_add(cached)
                .saturating_add(output_tokens)
                .saturating_add(reasoning)
        }),
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    })
}
