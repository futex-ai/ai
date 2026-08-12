//! Anthropic prompt-cache configuration and wire types.

use serde::Serialize;

use super::request::{AnthropicBlock, AnthropicRequest, AnthropicSystemBlock};

const MESSAGE_MARKER_STRIDE: usize = 20;
const MAX_MESSAGE_MARKERS: usize = 3;

/// Prompt-cache behavior for one Anthropic model instance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnthropicPromptCache {
    /// Emits no `cache_control` markers.
    Disabled,
    /// Emits prompt-cache markers with the selected lifetime.
    Enabled {
        /// Lifetime applied to every marker in a request.
        ttl: AnthropicCacheTtl,
    },
}

/// Lifetime for Anthropic prompt-cache entries.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AnthropicCacheTtl {
    /// Five-minute lifetime, serialized by omitting the `ttl` field.
    FiveMinutes,
    /// One-hour lifetime, serialized as `"ttl": "1h"`.
    OneHour,
}

/// Serialized Anthropic `cache_control` marker.
#[derive(Clone, Copy, Debug, Serialize)]
pub(super) struct AnthropicCacheControl {
    #[serde(rename = "type")]
    kind: AnthropicCacheControlKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    ttl: Option<AnthropicCacheControlTtl>,
}

impl AnthropicCacheControl {
    /// Builds a marker for the configured cache lifetime.
    pub(super) fn new(ttl: AnthropicCacheTtl) -> Self {
        Self {
            kind: AnthropicCacheControlKind::Ephemeral,
            ttl: match ttl {
                AnthropicCacheTtl::FiveMinutes => None,
                AnthropicCacheTtl::OneHour => Some(AnthropicCacheControlTtl::OneHour),
            },
        }
    }
}

/// Applies deterministic prompt-cache markers to a built request.
pub(super) fn apply_prompt_cache(
    request: &mut AnthropicRequest,
    prompt_cache: AnthropicPromptCache,
) {
    let AnthropicPromptCache::Enabled { ttl } = prompt_cache else {
        return;
    };
    let cache_control = AnthropicCacheControl::new(ttl);
    mark_prefix(request, cache_control);
    mark_message_tail(request, cache_control);
}

fn mark_prefix(request: &mut AnthropicRequest, cache_control: AnthropicCacheControl) {
    if request
        .system
        .iter_mut()
        .rev()
        .any(|block| mark_system_block(block, cache_control))
    {
        return;
    }
    if let Some(tool) = request.tools.last_mut() {
        tool.cache_control = Some(cache_control);
    }
}

fn mark_message_tail(request: &mut AnthropicRequest, cache_control: AnthropicCacheControl) {
    let mut tail_offset = 0usize;
    let mut next_marker_offset = 0usize;
    let mut marker_count = 0usize;

    for message in request.messages.iter_mut().rev() {
        for block in message.content.iter_mut().rev() {
            if tail_offset >= next_marker_offset && mark_block(block, cache_control) {
                marker_count += 1;
                if marker_count == MAX_MESSAGE_MARKERS {
                    return;
                }
                next_marker_offset = tail_offset.saturating_add(MESSAGE_MARKER_STRIDE);
            }
            tail_offset = tail_offset.saturating_add(1);
        }
    }
}

fn mark_system_block(
    block: &mut AnthropicSystemBlock,
    cache_control: AnthropicCacheControl,
) -> bool {
    let AnthropicSystemBlock::Text {
        text,
        cache_control: marker,
    } = block;
    if text.trim().is_empty() {
        return false;
    }
    *marker = Some(cache_control);
    true
}

fn mark_block(block: &mut AnthropicBlock, cache_control: AnthropicCacheControl) -> bool {
    let marker = match block {
        AnthropicBlock::Text {
            text,
            cache_control,
        } => {
            if text.trim().is_empty() {
                return false;
            }
            cache_control
        }
        AnthropicBlock::Image { cache_control, .. }
        | AnthropicBlock::ToolUse { cache_control, .. }
        | AnthropicBlock::ToolResult { cache_control, .. } => cache_control,
    };
    *marker = Some(cache_control);
    true
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum AnthropicCacheControlKind {
    Ephemeral,
}

#[derive(Clone, Copy, Debug, Serialize)]
enum AnthropicCacheControlTtl {
    #[serde(rename = "1h")]
    OneHour,
}
