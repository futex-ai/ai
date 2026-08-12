# Anthropic Prompt Caching Protocol

## Purpose

Enable Anthropic prompt caching so multi-turn agent loops stop paying full
input-token rates on every resent conversation prefix. Anthropic serves cached
prefix tokens at roughly 0.1x the base input price and with lower first-token
latency; without `cache_control` breakpoints in the request, no prefix is ever
cached and every turn re-bills the whole history at 1x.

This protocol covers two contracts:

1. The `ai-models-anthropic` request builder places `cache_control`
   breakpoints deterministically on every request.
2. The normalized usage and pricing types account for cache writes, which
   Anthropic bills at a premium (1.25x base input for the 5-minute TTL, 2x for
   the 1-hour TTL), separately from regular and cached input tokens.

## Status

Proposed. Implementation plan:
[plans/anthropic-prompt-caching.md](../../plans/anthropic-prompt-caching.md).

## Scope

In scope: request-side breakpoint placement, a typed per-model cache
configuration with TTL selection, normalized cache-write usage, and the
matching caller-supplied pricing field.

Out of scope: cache pre-warming (`max_tokens: 0`), the cache-diagnostics beta,
mid-conversation system messages, tool search, per-request cache overrides on
`ModelRequest`, reordering or sorting caller-supplied tools, provider caching
work for non-Anthropic adapters, and catalog feature flags for caching.

## Ownership

- `ai-models-anthropic` owns breakpoint placement, cache configuration, TTL
  serialization, and usage-field mapping from Anthropic responses.
- `ai-interface` owns the normalized `ModelUsage.cache_write_input_tokens`
  field and the `ModelUsageUnitKind::CacheWriteInputToken` unit kind.
- `ai-models-core` owns the `ModelPricing` cache-write price field and the
  cache-write cost line emitted by `price_usage`.
- Composition roots own prices, including choosing a cache-write unit price
  that matches the configured TTL multiplier, and choosing the TTL itself.

## Anthropic API Contract (background)

The implementation relies on these documented Anthropic behaviors:

- Caching is a prefix match over the rendered prompt in the order
  `tools` -> `system` -> `messages`. A breakpoint caches everything up to and
  including its block, so a breakpoint at the end of `messages` also covers
  tools and the system prompt.
- At most 4 `cache_control` breakpoints per request. Breakpoints attach to
  system text blocks, tool definitions, and message content blocks of type
  `text`, `image`, `tool_use`, and `tool_result`.
- `cache_control` is `{"type": "ephemeral"}` with an optional `"ttl"` of
  `"5m"` (default when omitted) or `"1h"`.
- Prefixes shorter than a model-dependent minimum (512 tokens on Claude Opus
  5/Fable 5, 1024 on Opus 4.8/Sonnet 5/Sonnet 4.6, 2048 on Opus 4.7, 4096 on
  Opus 4.6/Haiku 4.5) are silently not cached. No error, no write charge; the
  marker is harmless.
- Each breakpoint looks backward at most 20 content blocks for an existing
  cache entry to extend. A turn that appends more than 20 blocks with only a
  tail breakpoint misses the previous entry entirely.
- Response `usage` reports disjoint buckets: `input_tokens` (uncached,
  unwritten), `cache_creation_input_tokens` (written this request, billed at
  the TTL write premium), and `cache_read_input_tokens` (served from cache,
  billed at roughly 0.1x).
- Caches are model-scoped, and adding, moving, or removing `cache_control`
  markers does not by itself invalidate cached content.

## Configuration Contract

`ai-models-anthropic` exposes typed cache configuration:

```rust
pub enum AnthropicPromptCache {
    /// No cache_control markers are emitted.
    Disabled,
    /// Markers are emitted with the selected TTL.
    Enabled { ttl: AnthropicCacheTtl },
}

pub enum AnthropicCacheTtl {
    /// Five-minute TTL; serialized by omitting the "ttl" field.
    FiveMinutes,
    /// One-hour TTL; serialized as "ttl": "1h".
    OneHour,
}

impl AnthropicModel {
    /// Replaces the prompt-cache configuration for this model instance.
    pub fn with_prompt_cache(self, prompt_cache: AnthropicPromptCache) -> Self;
}
```

- The default for every existing constructor (`new`, `with_auth`,
  `with_catalog_auth`) is `Enabled { ttl: FiveMinutes }`. Caching is on by
  default because the request builder exists to serve multi-turn agent loops,
  where the 5-minute write premium breaks even after a single reuse.
- `Disabled` restores the current wire behavior exactly, except that `system`
  is serialized as a block array as defined below.
- One TTL applies to all markers in a request. Mixing TTLs is not supported.
- No environment variables are read; configuration is injected by the caller.

## Breakpoint Placement Contract

All placement is deterministic and stateless: identical `ModelRequest` input
plus identical configuration produces identical marker positions.

### Prefix marker (at most 1)

- The `system` field changes from a JSON string to an array of text blocks:
  `[{"type": "text", "text": <system prompt>}]`. When the effective system
  prompt (including the appended structured-output schema instruction) is
  empty, the `system` field is omitted entirely.
- When caching is enabled and the system prompt is non-empty, the last system
  block carries `cache_control`. This caches tools plus system as a shared
  prefix across conversations that reuse the same agent configuration.
- When the system prompt is empty and `tools` is non-empty, the last tool
  definition carries `cache_control` instead.
- When both are empty, no prefix marker is emitted.

### Message markers (at most 3)

Flatten the content blocks of the built Anthropic messages, in order, and
traverse from the final block toward the first:

1. Place a marker on the final content block.
2. After placing a marker, traverse 20 further blocks and place the next
   marker on the block reached.
3. Stop after 3 message markers or when the list is exhausted.

Rationale: consecutive requests in an agent loop share all previous blocks.
The previous request's final marker sits `A` blocks behind the new final
block, where `A` is the number of blocks the turn appended (one assistant
text block, one `tool_use` block per call, and one `tool_result` block per
call, so `A = 1 + 2N` for `N` tool calls). With markers at offsets 0, 20, and
40 from the tail, some new marker lies within the 20-block lookback of the
previous entry whenever `A < 60` (up to 29 parallel tool calls). Larger turns
degrade to a cache write without a read, never to an error.

### Marker budget and wire shape

- Total markers per request never exceed 4 (1 prefix + 3 message markers).
- Marker serialization: `"cache_control": {"type": "ephemeral"}` for the
  five-minute TTL and `"cache_control": {"type": "ephemeral", "ttl": "1h"}`
  for the one-hour TTL. The field is omitted from unmarked blocks.
- Markers never change block ordering, block content, or any other request
  field, and are never attached to block types Anthropic rejects.

## Caller Invariants

Effective caching additionally depends on the caller honoring the prefix
match. These are documented contracts, not runtime checks:

- The system prompt must be byte-stable across turns of one conversation. No
  timestamps, request ids, or other per-request interpolation.
- Tool definitions must keep a stable order and stable content across turns.
  The builder preserves caller order and must not sort.
- `ModelRequest.messages` must be append-only across turns.
- Model id and thinking level are fixed per `AnthropicModel` instance, which
  already guarantees the cache-scoping fields cannot drift mid-conversation.

## Usage Normalization Contract

`ModelUsage` gains a cache-write bucket so the four input-side buckets are
disjoint and each maps to exactly one price:

```rust
pub struct ModelUsage {
    pub input_tokens: u64,             // uncached, unwritten input
    pub output_tokens: u64,
    pub cached_input_tokens: u64,      // served from cache (reads)
    pub cache_write_input_tokens: u64, // written to cache this request (new)
    pub reasoning_tokens: u64,
    pub total_tokens: u64,             // sum of the buckets above
    // estimated_cost_microusd, cost_lines unchanged
}
```

- The new field carries `#[serde(default)]` so previously stored payloads
  deserialize unchanged.
- Anthropic mapping: `input_tokens` <- `usage.input_tokens` (the current
  fold of `cache_creation_input_tokens` into `input_tokens` is removed),
  `cache_write_input_tokens` <- `usage.cache_creation_input_tokens`,
  `cached_input_tokens` <- `usage.cache_read_input_tokens`, and
  `total_tokens` is the saturating sum of all buckets.
- Every other provider adapter and the mock model report
  `cache_write_input_tokens: 0`; no other provider currently bills a
  write premium.

## Pricing Contract

`ModelPricing` gains a matching optional price, and `price_usage` emits a
matching line:

```rust
pub struct ModelPricing {
    // ...existing fields...
    /// Cache-write-token price in micro-USD per one million tokens.
    pub cache_write_input_token_usd_micros_per_million: Option<u64>,
}

pub enum ModelUsageUnitKind {
    // ...existing kinds...
    /// Cache-write input token. as_str: "cache_write_input_token".
    CacheWriteInputToken,
}
```

- `price_usage` pushes a `CacheWriteInputToken` line whenever
  `cache_write_input_tokens > 0`, with the same unpriced semantics as every
  other kind: `Unknown` measurement state without a price, `Free` under
  `free_when_unpriced` or a zero price, and `ModelPricing::free` prices it at
  zero.
- Composition roots must set the cache-write unit price to match the TTL they
  configure: 1.25x the base input price for `FiveMinutes`, 2x for `OneHour`.
  The library does not derive one price from another.

## Error Handling

No new error variants. Marker placement is pure and infallible; requests that
were valid before remain valid. Below-minimum prefixes and oversized turns
degrade to uncached behavior, never to failures.

## Verification

- Request-builder unit tests assert exact marker positions and TTL
  serialization for: defaults, disabled caching, one-hour TTL, empty system
  with and without tools, single-turn requests, multi-turn tool loops, a
  history long enough to require stride markers, and the 4-marker budget.
- Response tests assert the three-way usage split and totals.
- Pricing tests assert the cache-write line, including unpriced and free
  states.
- Live verification for composition roots: a second identical-prefix request
  within the TTL must report `cached_input_tokens > 0`; persistent zeros mean
  a caller invariant is being violated.
