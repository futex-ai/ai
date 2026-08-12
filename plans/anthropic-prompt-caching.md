# Anthropic Prompt Caching Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Emit Anthropic `cache_control` breakpoints from the request builder
so multi-turn agent loops pay cache-read rates (~0.1x) on resent conversation
prefixes, and meter the cache writes this enables (billed at 1.25x/2x) as
their own normalized usage bucket instead of folding them into plain input.

**Architecture:** Extend the shared usage/pricing contracts first
(`ai-interface`, `ai-models-core`) so cache writes are priced correctly the
moment they start occurring, then add deterministic breakpoint placement and
typed cache configuration inside `ai-models-anthropic`. Response-side
plumbing for cache reads (`cached_input_tokens`,
`ModelUsageUnitKind::CachedInputToken`, the cached-input price field) already
exists and lights up unchanged.

**Tech Stack:** Rust, `serde`, `serde_json`, `async-trait`, `unimock`,
`json-http`, and the existing `ai-interface` / `ai-models-core` contracts.

## Global Constraints

- Implement the normative
  [Anthropic Prompt Caching Protocol](../docs/protocol/anthropic-prompt-caching.md)
  without adding behavior not defined there.
- Caching defaults to enabled with the five-minute TTL on every existing
  `AnthropicModel` constructor; `Disabled` restores today's wire behavior
  except for the system-block-array change the protocol defines.
- Marker placement is pure, deterministic, and infallible: no new error
  variants, no environment reads, no network work, no live-credential tests.
- Only the Anthropic adapter reports nonzero `cache_write_input_tokens`;
  every other provider and the mock model report zero.
- Add failing tests before production behavior; keep test bodies in
  `_tests_/` files, keep every changed Rust file at or below 300 lines, and
  document every public Rust item.
- Do not use `panic!`, `unwrap`, `expect`, `map_err`, untyped production JSON
  boundaries, `anyhow`, or `eyre` in production code.

---

## File Map And Interfaces

Shared contracts:

- `crates/ai-interface/src/usage.rs` gains the cache-write bucket and unit
  kind.
- `crates/ai-models-core/src/pricing.rs` gains the cache-write price and cost
  line.

Anthropic adapter (`crates/ai-models-anthropic/`):

- `src/anthropic/cache.rs` (new) owns `AnthropicPromptCache`,
  `AnthropicCacheTtl`, the serialized `cache_control` type, and marker
  placement over the built request.
- `src/anthropic/request.rs` builds system blocks and delegates marker
  placement to `cache.rs`.
- `src/anthropic/mod.rs` stores the configuration on `AnthropicModel` and
  exposes `with_prompt_cache`.
- `src/lib.rs` exports the new configuration types from their owner module.

The shared interfaces introduced by the implementation are:

```rust
// ai-interface
pub struct ModelUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: u64,
    #[serde(default)]
    pub cache_write_input_tokens: u64, // new
    pub reasoning_tokens: u64,
    pub total_tokens: u64,
    // estimated_cost_microusd, cost_lines unchanged
}

pub enum ModelUsageUnitKind {
    InputToken,
    OutputToken,
    CachedInputToken,
    CacheWriteInputToken, // new; as_str "cache_write_input_token"
    ReasoningToken,
}

// ai-models-core
pub struct ModelPricing {
    // ...existing fields...
    pub cache_write_input_token_usd_micros_per_million: Option<u64>, // new
}

// ai-models-anthropic
pub enum AnthropicPromptCache {
    Disabled,
    Enabled { ttl: AnthropicCacheTtl },
}

pub enum AnthropicCacheTtl {
    FiveMinutes,
    OneHour,
}

impl AnthropicModel {
    pub fn with_prompt_cache(self, prompt_cache: AnthropicPromptCache) -> Self;
}
```

## Milestone 1: Cache-Write Usage And Pricing Accounting

Add the cache-write bucket to the shared usage and pricing contracts and map
Anthropic's `cache_creation_input_tokens` into it. At the end of this
milestone every provider compiles and reports disjoint usage buckets, cache
writes are priced when a price is configured, and no request-side behavior
has changed yet.

**Files:**

- Modify `crates/ai-interface/src/usage.rs`.
- Modify `crates/ai-interface/src/mock_model.rs`.
- Modify `crates/ai-interface/README.md`.
- Modify `crates/ai-models-core/src/pricing.rs` and
  `crates/ai-models-core/src/_tests_/pricing_tests.rs`.
- Modify `crates/ai-models-core/README.md`.
- Modify `crates/ai-models-anthropic/src/anthropic/response.rs` and the
  affected tests under `crates/ai-models-anthropic/src/anthropic/_tests_/`.
- Zero-fill `ModelUsage` literals in the other provider adapters:
  `crates/ai-models-openai/src/openai/response/mod.rs`,
  `crates/ai-models-google/src/google/response.rs`,
  `crates/ai-models-deepseek/src/deepseek/response.rs`,
  `crates/ai-models-kimi/src/kimi/response.rs`,
  `crates/ai-models-qwen/src/qwen/response.rs`,
  `crates/ai-models-xai/src/xai/response.rs`,
  `crates/ai-models-minimax/src/minimax/response.rs`.
- Update every remaining `ModelUsage` literal the compiler reports across
  workspace tests (including `ai-tool-calling` and `ai-mcp` test support).

- [x] Add failing `ai-interface` serde tests: the new field round-trips,
      defaults to zero when absent from stored payloads, and
      `ModelUsageUnitKind::CacheWriteInputToken.as_str()` returns
      `cache_write_input_token`.
- [x] Add `cache_write_input_tokens` and `CacheWriteInputToken` with complete
      public rustdoc.
- [x] Add failing pricing tests: priced cache-write line with correct
      micro-USD rounding, `Unknown` state when unpriced, `Free` under
      `free_when_unpriced` and under `ModelPricing::free`, no line at zero
      quantity, and `estimated_cost_microusd` including the new line.
- [x] Add `cache_write_input_token_usd_micros_per_million` to `ModelPricing`,
      update `ModelPricing::free`, and emit the line from `price_usage`.
- [x] Add failing Anthropic usage tests: `input_tokens` excludes
      `cache_creation_input_tokens`, `cache_write_input_tokens` carries it,
      `cached_input_tokens` still carries `cache_read_input_tokens`,
      `total_tokens` sums all buckets, and missing usage fields default to
      zero.
- [x] Stop folding `cache_creation_input_tokens` into `input_tokens` in the
      Anthropic response mapping and populate the new bucket.
- [x] Zero-fill the new field in every other provider adapter, the mock
      model, and every test literal; verify no non-Anthropic adapter maps a
      provider value into it.
- [x] Update `ai-interface` and `ai-models-core` READMEs for the new usage
      bucket, unit kind, and price field.
- [x] Run `cargo fmt --all -- --check`,
      `cargo clippy --workspace --all-targets --all-features`, and
      `cargo test --workspace --all-features` with a 100% pass rate.

## Milestone 2: Anthropic Request Cache Configuration And Breakpoints

Emit `cache_control` markers from the request builder. At the end of this
milestone caching is live by default for every `AnthropicModel`, callers can
disable it or select the one-hour TTL, and marker placement matches the
protocol exactly.

**Files:**

- Create `crates/ai-models-anthropic/src/anthropic/cache.rs`.
- Create
  `crates/ai-models-anthropic/src/anthropic/_tests_/anthropic_cache_tests.rs`.
- Modify `crates/ai-models-anthropic/src/anthropic/request.rs`.
- Modify `crates/ai-models-anthropic/src/anthropic/mod.rs`.
- Modify `crates/ai-models-anthropic/src/lib.rs`.
- Modify existing tests under
  `crates/ai-models-anthropic/src/anthropic/_tests_/` for the system
  block-array shape.
- Modify `crates/ai-models-anthropic/README.md`.

- [x] Add failing system-shape tests: `system` serializes as a text-block
      array carrying the prompt (including the structured-output suffix), and
      the field is omitted when the effective system prompt is empty.
- [x] Convert `system` to typed blocks in the request builder and update
      existing request tests to the new shape.
- [x] Add failing configuration tests: every constructor defaults to
      `Enabled { ttl: FiveMinutes }`, `with_prompt_cache(Disabled)` emits no
      `cache_control` anywhere, and `OneHour` serializes
      `{"type": "ephemeral", "ttl": "1h"}` while `FiveMinutes` omits `ttl`.
- [x] Add `AnthropicPromptCache`, `AnthropicCacheTtl`, and the serialized
      cache-control type in `cache.rs` with complete public rustdoc; store
      the configuration on `AnthropicModel`, add `with_prompt_cache`, and
      export the types from `lib.rs`.
- [x] Add failing prefix-marker tests: marker on the last system block when
      the system prompt is non-empty, on the last tool definition when the
      system prompt is empty and tools exist, and absent when both are empty.
- [x] Add failing message-marker tests: single-turn request marks only the
      final block; a multi-turn tool loop marks the final block; a history
      long enough for stride placement marks blocks at offsets 0, 20, and 40
      from the tail; total markers never exceed 4; markers attach only to
      `text`, `image`, `tool_use`, and `tool_result` blocks; unmarked blocks
      omit the field entirely.
- [x] Implement deterministic marker placement in `cache.rs` per the
      protocol: one prefix marker plus tail-stride message markers with a
      20-block stride and a 3-marker cap, applied after message building.
- [x] Keep `request.rs`, `cache.rs`, and `mod.rs` each at or below 300 lines,
      with placement logic and cache types owned by `cache.rs`.
- [x] Update the `ai-models-anthropic` README: responsibilities, caching
      behavior and defaults, `with_prompt_cache` Quick Start example, caller
      invariants (stable system/tool bytes, append-only history), key code,
      and a link to the protocol doc.
- [x] Run `cargo fmt --all -- --check`,
      `cargo clippy -p ai-models-anthropic --all-targets --all-features`, and
      `cargo test -p ai-models-anthropic --all-features` with a 100% pass
      rate.

## Milestone 3: Documentation Reconciliation, Verification, And Review

Reconcile the docs with the landed implementation, verify the workspace, and
publish the change. At the end of this milestone all changes are tracked and
pushed and review findings are ready for the user to assess.

**Files:**

- Modify `docs/protocol/anthropic-prompt-caching.md` (status only).
- Modify root `README.md`.
- Modify `plans/README.md`.

- [x] Reconcile the protocol doc line by line with the implemented types,
      marker algorithm, wire shapes, usage mapping, and pricing fields;
      change its status to implemented only after verification.
- [x] Update the root `README.md` key-features list for Anthropic prompt
      caching and confirm the protocol and key-code lists link the new doc.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all`
      and re-run the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features` and require a 100% pass
      rate.
- [x] Run `cargo xtask smoke-test` and confirm it requires neither
      credentials nor network access.
- [x] Run `cargo xtask check`; fix failures and repeat until it passes.
- [x] Audit `git diff origin/main...` for unrelated changes, untracked
      files, stale README claims, and lockfile drift.
- [x] Move this plan from Active to Completed in `plans/README.md` only
      after all earlier milestones and final checks are complete.
- [x] Run `git add -A` so every source, test, README, protocol, plan, and
      lockfile change is tracked.
- [x] Commit the completed work with a Conventional Commit title no longer
      than 50 characters and a descriptive body.
- [x] Push the current branch.
- [x] Run `cargo xtask review` after the push so the reviewer compares the
      complete branch with `origin/main`.
- [x] Do not auto-fix review findings. Report each item with a number,
      severity, codebase/feature context, impact of doing nothing, lettered
      solution options, and the recommended option.
