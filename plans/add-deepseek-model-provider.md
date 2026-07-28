# DeepSeek Model Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` (recommended) or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add current DeepSeek V4 Pro and Flash models as a first-class,
fully tested provider behind the shared model interface.

**Architecture:** Add provider identity and private replay DTOs to
`ai-interface`, then implement a dedicated `ai-models-deepseek` crate over the
injected `json-http` boundary. Keep DeepSeek-specific thinking, replay,
structured-output, usage, and finish semantics inside that crate while the
tool runtime owns replay retention and log redaction.

**Tech Stack:** Rust, `async-trait`, `serde`, `serde_json`, `thiserror`,
`unimock`, `json-http`, and the existing `ai-interface` / `ai-models-core`
contracts.

## Global Constraints

- Implement the normative
  [DeepSeek Model Provider Protocol](../docs/protocol/deepseek-model-provider.md)
  without adding behavior not defined there.
- Support only `deepseek-v4-pro` and `deepseek-v4-flash` on
  `https://api.deepseek.com/chat/completions`.
- Exclude retired aliases, streaming, vision, beta APIs, Anthropic-format
  access, custom endpoints, and live credential-dependent tests.
- Use injected trait-object collaborators for HTTP and authentication; the
  provider crate must not read environment variables or resolve secrets.
- Add failing tests before production behavior and use `unimock` at Rust trait
  boundaries.
- Keep production and test bodies in separate files, keep every changed Rust
  file at or below 300 lines, and document every public Rust item.
- Do not use `panic!`, `unwrap`, `expect`, untyped production JSON boundaries,
  `anyhow`, `eyre`, or direct diagnostic printing in production code.
- Keep provider reasoning out of normalized assistant content and every
  model-call logger payload.

---

## File Map And Interfaces

Create the provider under `crates/ai-models-deepseek/`:

- `src/catalog.rs` owns the six catalog constants and `known_models()`.
- `src/deepseek/client.rs` owns `DeepSeekModel`, configuration validation,
  injected auth, dispatch, and transport/HTTP error mapping.
- `src/deepseek/request_types.rs` owns typed Chat Completions request DTOs.
- `src/deepseek/request.rs` owns normalized request, thinking, tool,
  structured-output prompt, and replay mapping.
- `src/deepseek/response.rs` owns typed response DTOs and finish, tool,
  structured-output, replay, and usage normalization.
- `src/deepseek/_tests_/*.rs` splits provider tests by observable behavior.
- `src/_tests_/catalog_tests.rs` owns catalog contract tests.

The shared interfaces introduced by the implementation are:

```rust
ProviderKind::DeepSeek

ProviderConversationItem::DeepSeekAssistantMessage {
    content: String,
    reasoning_content: Option<String>,
    tool_calls: Vec<DeepSeekToolCallContext>,
}

pub struct DeepSeekToolCallContext {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

pub enum DeepSeekConfigurationError {
    UnsupportedProviderModel { provider_model_id: String },
    UnsupportedThinkingLevel { thinking_level: &'static str },
}

pub type DeepSeekConfigurationResult<T> =
    std::result::Result<T, DeepSeekConfigurationError>;

impl DeepSeekModel {
    pub fn new(
        http_client: DynJsonHttpClient,
        api_key: impl Into<String>,
    ) -> Self;

    pub fn with_auth(
        http_client: DynJsonHttpClient,
        auth: DynJsonHttpAuth,
    ) -> Self;

    pub fn with_catalog_auth(
        http_client: DynJsonHttpClient,
        catalog_model_id: impl Into<String>,
        provider_model_id: impl Into<String>,
        thinking_level: ThinkingLevel,
        auth: DynJsonHttpAuth,
    ) -> DeepSeekConfigurationResult<Self>;
}

pub fn known_models() -> Vec<KnownModelSpec>;
```

## Milestone 1: Shared Provider And Replay Contracts

Add the shared types without changing existing provider behavior. At the end
of this milestone, routing and retained conversations can represent DeepSeek,
and existing adapters ignore DeepSeek-owned context.

**Files:**

- Modify `crates/ai-interface/src/router.rs`.
- Modify `crates/ai-interface/src/messages.rs`.
- Modify `crates/ai-interface/src/lib.rs`.
- Modify `crates/ai-interface/src/_tests_/router_tests.rs`.
- Modify `crates/ai-interface/src/_tests_/messages_tests.rs`.
- Modify `crates/ai-interface/README.md`.
- Modify `crates/ai-models-core/src/tool_call_identity.rs` and
  `crates/ai-models-core/src/_tests_/tool_call_identity_tests.rs`.
- Modify `crates/ai-models-openai/src/openai/request.rs` and
  `crates/ai-models-openai/src/openai/_tests_/openai_reasoning_replay_tests.rs`.
- Modify `crates/ai-models-minimax/src/minimax/request.rs` and
  `crates/ai-models-minimax/src/minimax/_tests_/replay_tests.rs`.
- Modify `crates/ai-models-xai/src/xai/request.rs` and
  `crates/ai-models-xai/src/xai/_tests_/xai_continuation_tests.rs`.
- Modify `crates/ai-models-kimi/src/kimi/_tests_/request_tests.rs`.

- [x] Add failing routing tests for config parsing, display, snake-case serde,
      and unknown-provider rejection for `deepseek`.
- [x] Add `ProviderKind::DeepSeek` and update every exhaustive provider match
      without catch-all arms.
- [x] Add failing serde tests for a DeepSeek replay item containing empty
      content, optional reasoning, ordered calls, provider ids, names, and raw
      argument strings.
- [x] Add `DeepSeekToolCallContext` and
      `ProviderConversationItem::DeepSeekAssistantMessage` with narrow
      visibility and complete public rustdoc.
- [x] Add the DeepSeek item to deterministic tool-call scope hashing with
      regression tests for every field.
- [x] Add regression tests proving OpenAI, MiniMax, xAI, Kimi, and other
      providers never serialize DeepSeek-owned replay state.
- [x] Update `ai-interface` README responsibilities, behavior, Quick Start,
      key code, and related-doc links for DeepSeek identity and replay.
- [x] Run `cargo fmt --all -- --check` and
      `cargo test -p ai-interface -p ai-models-core --all-features`.

## Milestone 2: Provider Crate, Catalog, And Text Completion

Create a usable text-only provider with explicit construction and typed
transport seams. At the end of this milestone, callers can select any current
DeepSeek catalog variant and complete a non-tool text request.

**Files:**

- Create `crates/ai-models-deepseek/Cargo.toml`.
- Create `crates/ai-models-deepseek/README.md`.
- Create `crates/ai-models-deepseek/src/lib.rs`.
- Create `crates/ai-models-deepseek/src/catalog.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/mod.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/client.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/request.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/request_types.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/response.rs`.
- Create `crates/ai-models-deepseek/src/_tests_/catalog_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/support.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/construction_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/request_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/input_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/response_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/client_tests.rs`.
- Modify root `Cargo.toml` and generated `Cargo.lock`.

- [x] Add the crate to workspace members and dependencies with internal
      `{ workspace = true }` references; add external crates with `cargo add`
      and no guessed versions.
- [x] Add failing catalog tests for all six exact ids, provider ids,
      1,000,000-token context, Pro/Flash intelligence and speed tiers, low
      cost, enabled/disabled features, and high/max/disabled thinking.
- [x] Implement the six exported constants and `known_models()` exactly as
      specified; do not include legacy aliases or vision.
- [x] Add failing configuration tests for default Pro/high construction,
      explicit auth, every valid model/thinking combination, unknown provider
      ids, and rejected Low/Medium/ExtraHigh thinking.
- [x] Implement `DeepSeekConfigurationError`,
      `DeepSeekConfigurationResult<T>`, and `DeepSeekModel` constructors
      behind `ai_interface::Model`.
- [x] Add failing request tests for the exact endpoint, bearer/custom auth,
      `stream: false`, leading system message, all plain roles, optional names,
      tool result ids, empty strings, and omitted unsupported fields.
- [x] Add failing input tests proving every non-empty `content_parts` value
      fails before auth and transport instead of dropping data.
- [x] Implement typed non-streaming request and response DTOs, text mapping,
      injected request dispatch, shared HTTP classification, transient
      transport/auth handling, and internal serialization/deserialization
      handling.
- [x] Add stopped-response tests for provider/catalog ids, selected thinking,
      nullable content, empty choices, and malformed typed wire fields.
- [x] Create the crate README with the required ordered sections, accurate
      limits, compiling Quick Start, development commands, key code, and
      protocol/plan links.
- [x] Run `cargo metadata --format-version 1 --no-deps`,
      `cargo fmt --all -- --check`,
      `cargo clippy -p ai-models-deepseek --all-targets --all-features`, and
      `cargo test -p ai-models-deepseek --all-features`.

## Milestone 3: Thinking, Tools, And Private Replay

Complete the agentic continuation path while protecting reasoning. At the end
of this milestone, tool turns replay the exact DeepSeek assistant state and
work through the shared runtime without exposing private reasoning.

**Files:**

- Create `crates/ai-models-deepseek/src/deepseek/_tests_/thinking_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/tool_call_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/continuation_tests.rs`.
- Modify `crates/ai-models-core/README.md`.
- Modify `crates/ai-tool-calling/src/turn/response.rs`.
- Add `crates/ai-tool-calling/src/_tests_/deepseek_logging_redaction_tests.rs`
  and register it in `src/_tests_/mod.rs`.
- Modify `crates/ai-tool-calling/README.md`.

- [x] Add failing thinking tests for
      `disabled` plus omitted effort, `enabled/high`, `enabled/max`, recorded
      normalized thinking, and the absence of sampling parameters.
- [x] Implement exact thinking serialization and omit `tool_choice` for every
      request, including disabled-thinking tool requests.
- [x] Add failing tool tests for definitions, modern calls, parallel ordering,
      provider ids, JSON argument parsing, raw argument preservation, tool
      results, and absent/null/empty/malformed dispatchable payloads.
- [x] Add failing terminal-safety tests proving tool payloads on every
      non-tool finish are not parsed, dispatched, or retained.
- [x] Implement finish-gated parsing, require non-empty dispatchable calls,
      and preserve raw DeepSeek calls separately from normalized `ToolCall`.
- [x] Add failing continuation tests proving exact content, reasoning, calls,
      and tool-result pairing survive response parsing, shared serde, runtime
      retention, and the next provider request.
- [x] Add tests requiring reasoning on enabled-thinking tool responses while
      allowing it to be absent when thinking is disabled.
- [x] Implement DeepSeek replay only for dispatchable tool-call turns, prefer
      provider-owned raw replay over normalized assistant fields, and ignore
      all foreign replay items.
- [x] Add logger tests proving private DeepSeek replay is removed from both
      request and response log copies but retained in live calls and
      conversation state; update the shared redaction boundary.
- [x] Run DeepSeek tests plus targeted `ai-models-core` identity and
      `ai-tool-calling` conversation/logging tests with a 100% pass rate.

## Milestone 4: Structured Output, Finish Safety, Usage, And Errors

Complete the remaining normalized behavior. At the end of this milestone, the
provider's advertised features, usage accounting, and retry semantics are
fully covered by tests.

**Files:**

- Create `crates/ai-models-deepseek/src/deepseek/_tests_/structured_output_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/usage_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/finish_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/response_shape_tests.rs`.
- Create `crates/ai-models-deepseek/src/deepseek/_tests_/error_tests.rs`.
- Modify the focused DeepSeek request, response, and client modules only.

- [x] Add failing structured-output tests for the augmented system prompt,
      schema name and value, required word `JSON`, raw-output instruction,
      `response_format: {"type":"json_object"}`, successful local validation,
      empty output, invalid JSON, invalid schema, and schema mismatch.
- [x] Implement JSON-object mode plus shared local parsing and validation only
      for a natural stop with no dispatchable calls.
- [x] Add failing finish tests for `stop`, `tool_calls`, `length`,
      `content_filter`, unknown, missing, and
      `insufficient_system_resource`.
- [x] Map the resource-limited finish directly to
      `ModelError::TransientProvider`; preserve normalized behavior for every
      other finish.
- [x] Add failing usage tests for prompt totals, cache hit, cache miss,
      fallback cache subtraction, completion reasoning, visible output,
      provider totals, reconstructed totals, missing usage, and saturating
      arithmetic.
- [x] Implement non-overlapping usage buckets and leave pricing/cost lines for
      composition-root `UsagePricingModel`.
- [x] Add failing error tests for `400`, `401`, `402`, `408`, `409`, `422`,
      `425`, `429`, `500`, `503`, recognized context overflow, transport,
      auth, request serialization, typed response deserialization, missing
      choices, and semantic provider failures.
- [x] Reuse shared HTTP classification, retain typed internal error sources,
      and ensure no API key or authorization value enters error text or debug
      output.
- [x] Run the complete DeepSeek suite plus targeted core
      structured-output/error tests, formatting, and provider Clippy.

## Milestone 5: Workspace Integration And Documentation

Make the provider discoverable and construction-smoke-tested. At the end of
this milestone, downstream composition roots can find, construct, wrap, and
verify DeepSeek without reading implementation internals.

**Files:**

- Modify `xtask/Cargo.toml`, `xtask/src/smoke.rs`, and `xtask/README.md`.
- Modify root `README.md`.
- Modify `crates/ai-models-openai/README.md` and
  `crates/ai-models-xai/README.md` where their foreign-context guarantees
  enumerate provider families.
- Modify `docs/protocol/deepseek-model-provider.md`.
- Modify `plans/README.md` only when the implementation is complete.

- [x] Export the provider crate's model, configuration error/result, catalog
      constants, and `known_models()` from their real owner modules.
- [x] Add `ai-models-deepseek` to xtask and construct the default provider
      with a placeholder credential in `cargo xtask smoke-test` without a
      provider request.
- [x] Update xtask README so the construction list includes every adapter
      actually instantiated by the smoke test, including Kimi and DeepSeek.
- [x] Update the root README feature list, protocol list, interface map,
      smoke-test description, and key-code map for DeepSeek.
- [x] Review `ai-interface`, `ai-models-core`, `ai-tool-calling`, and existing
      provider READMEs for newly stale exhaustive provider/replay language and
      update only affected documentation.
- [x] Reconcile the protocol line by line with implemented ids, endpoint,
      request fields, replay rules, response mapping, usage, errors, and
      exclusions; change its status to implemented only after verification.
- [x] Run `cargo xtask smoke-test` and confirm it requires neither credentials
      nor network access.

## Milestone 6: Full Verification, Commit, Push, And Review

Validate and publish the complete provider. At the end of this milestone, all
changes are tracked and pushed, and review findings are ready for the user to
assess.

- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      repeat the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [x] Run `cargo xtask smoke-test`.
- [x] Run `cargo xtask check`; fix failures and repeat until it passes.
- [x] Audit `git diff origin/main...` for unrelated changes, untracked files,
      private reasoning leakage, legacy DeepSeek aliases, false vision
      claims, endpoint drift, missing docs, and lockfile drift.
- [x] Move this plan from Active to Completed in `plans/README.md` only after
      all earlier milestones and final checks are complete.
- [x] Run `git add -A` so every source, test, README, protocol, plan, and
      lockfile file is tracked.
- [ ] Commit the completed work with a Conventional Commit title no longer
      than 50 characters and a descriptive body.
- [ ] Push the current branch.
- [ ] Run `cargo xtask review` after the push so the reviewer compares the
      complete branch with `origin/main`.
- [ ] Do not auto-fix review findings. Report each item with a number,
      severity, codebase/feature context, impact of doing nothing, lettered
      solution options, and the recommended option.
