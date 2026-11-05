//! Token normalization from typed accumulated provider reports.

use ai_interface::ModelUsage;
use serde::de::{Error, Unexpected};

use crate::ChatCompletionsUsage;

use super::super::dialect::UsageFields;

pub(super) fn normalize(
    rule: UsageFields,
    usage: ChatCompletionsUsage,
) -> Result<ModelUsage, serde_json::Error> {
    let (cached, cache_miss, reasoning) = match rule {
        UsageFields::CacheHitMiss => (
            usage.prompt_cache_hit_tokens,
            usage.prompt_cache_miss_tokens,
            usage.completion_tokens_details.reasoning_tokens,
        ),
        UsageFields::DirectCached => (
            usage
                .cached_tokens
                .unwrap_or(usage.prompt_tokens_details.cached_tokens),
            None,
            0,
        ),
        UsageFields::CachedInput => (usage.prompt_tokens_details.cached_tokens, None, 0),
        UsageFields::CachedAndReasoning => (
            usage.prompt_tokens_details.cached_tokens,
            None,
            usage.completion_tokens_details.reasoning_tokens,
        ),
        UsageFields::BoundedCachedAndReasoning => {
            bounded(usage.completion_tokens)?;
            bounded(usage.completion_tokens_details.reasoning_tokens)?;
            bounded(usage.prompt_tokens)?;
            bounded(usage.prompt_tokens_details.cached_tokens)?;
            if let Some(total) = usage.total_tokens {
                bounded(total)?;
            }
            (
                usage.prompt_tokens_details.cached_tokens,
                None,
                usage.completion_tokens_details.reasoning_tokens,
            )
        }
    };
    let input_tokens = cache_miss.unwrap_or_else(|| usage.prompt_tokens.saturating_sub(cached));
    let output_tokens = usage.completion_tokens.saturating_sub(reasoning);
    Ok(ModelUsage {
        input_tokens,
        output_tokens,
        cached_input_tokens: cached,
        reasoning_tokens: reasoning,
        total_tokens: usage.total_tokens.unwrap_or_else(|| {
            input_tokens
                .saturating_add(cached)
                .saturating_add(output_tokens)
                .saturating_add(reasoning)
        }),
        estimated_cost_microusd: 0,
        cost_lines: Vec::new(),
    })
}

fn bounded(value: u64) -> Result<(), serde_json::Error> {
    if value > u64::from(u32::MAX) {
        return Err(serde_json::Error::invalid_value(
            Unexpected::Unsigned(value),
            &"u32",
        ));
    }
    Ok(())
}
