# MiniMax Model Provider

## Summary

Add a first-class `ai-models-minimax` crate that implements
`ai_interface::Model` against MiniMax's OpenAI-compatible Chat Completions API.
The provider must support current MiniMax agent models, tool calling,
interleaved-thinking replay, M3 image input, locally validated structured
output, normalized usage, typed errors, model routing, and credential-free
smoke construction.

The normative behavior is defined by the
[MiniMax Model Provider Protocol](../docs/protocol/minimax-model-provider.md).

## Scope

This plan covers:

- `ai-interface` provider identity and MiniMax replay context
- `ai-models-minimax` catalog, model client, request/response mapping, tests,
  and crate documentation
- Root workspace membership, dependency declarations, README, lockfile, and
  smoke construction
- Provider-focused regression tests and full workspace verification

V1 does not include streaming, MiniMax server tools, video input, legacy
MiniMax models, regional/custom endpoints, live credentialed CI, or an
application configuration UI.

## Milestone 1: Shared Provider And Replay Contracts

Extend the shared interface without changing existing provider behavior. At
the end of this milestone, routing and conversation state can represent
MiniMax explicitly, and every existing crate still builds and tests.

- [x] Add failing `ai-interface` tests for the `minimax` config identifier,
      display value, serde representation, and unknown-provider rejection.
- [x] Add `ProviderKind::MiniMax` with stable snake-case config and serde value
      `minimax`; update exhaustive provider matches without fallback arms.
- [x] Add failing serde tests for MiniMax assistant replay context containing
      `reasoning_content` and complete ordered `reasoning_details`.
- [x] Add typed MiniMax reasoning-detail and provider-conversation DTOs with
      rustdoc on every public item and optional field.
- [x] Update existing provider request mappers to ignore MiniMax context that
      they do not own while preserving their current replay behavior.
- [x] Update `ai-interface` README responsibilities, public behavior, Quick
      Start example, and key-code map for MiniMax provider identity and replay
      context.
- [x] Run `cargo fmt --all -- --check` and
      `cargo test -p ai-interface --all-features`.

## Milestone 2: Provider Crate, Catalog, And Text Calls

Create a usable provider with catalog routing and basic text completion. At the
end of this milestone, callers can construct MiniMax models and perform
non-streaming text calls with typed transport behavior.

- [x] Add `crates/ai-models-minimax` with a thin `lib.rs`, module-level docs,
      `#![warn(unreachable_pub)]`, and focused catalog/client/request/response
      modules that remain below the Rust file-size limit.
- [x] Add the crate to workspace members and workspace dependencies using
      internal `{ workspace = true }` references; add external dependencies
      with `cargo add` without guessing versions.
- [x] Add failing catalog tests for `MiniMax-M3`,
      `MiniMax-M3-thinking-disabled`, `MiniMax-M2.7`, and
      `MiniMax-M2.7-highspeed`, including provider ids, context windows,
      features, thinking levels, and routing tiers.
- [x] Implement exported catalog constants and `known_models()` exactly as
      defined by the protocol, excluding legacy models.
- [x] Add failing model tests for the international Chat Completions endpoint,
      bearer authentication, injected authentication, provider/catalog ids,
      system and plain conversation messages, empty optional fields, and a
      stopped text response.
- [x] Implement `MiniMaxModel` behind `ai_interface::Model`, with `new`,
      `with_auth`, and `with_catalog_auth` constructors and injected
      `DynJsonHttpClient` / `DynJsonHttpAuth` collaborators.
- [x] Implement typed request and response DTOs for non-streaming text calls;
      do not load configuration, inspect environment variables, or make
      ambient credential decisions.
- [x] Reuse shared HTTP status classification and map transport/auth failures
      to transient errors while keeping local serialization/deserialization
      failures internal.
- [x] Create the crate `README.md` with the required ordered sections,
      accurate public behavior, compiling Quick Start, development commands,
      key code, and related docs.
- [x] Run `cargo metadata --format-version 1 --no-deps`,
      `cargo fmt --all -- --check`,
      `cargo clippy -p ai-models-minimax --all-targets --all-features`, and
      `cargo test -p ai-models-minimax --all-features`.

## Milestone 3: Tool Calls And Interleaved-Thinking Replay

Add the agentic path without exposing private reasoning. At the end of this
milestone, multi-round MiniMax tool conversations retain the provider's full
reasoning context and work through the shared tool runtime.

- [x] Add failing request tests for tool definitions, assistant tool-call
      history, tool results, multiple calls, omitted tool names, and modern
      `tool_calls` only.
- [x] Add failing thinking tests for M3 disabled/adaptive mapping, enabled M2.7
      catalog entries, and unconditional `reasoning_split: true`.
- [x] Add failing response tests for visible content, multiple tool calls,
      JSON arguments, provider ids, operation ids, and malformed arguments.
- [x] Add failing replay tests proving `reasoning_content` and every populated
      reasoning-detail field survive response parsing, shared serde, and the
      next assistant request unchanged.
- [x] Add a non-disclosure regression test proving reasoning text is absent
      from normalized `assistant_message`.
- [x] Serialize normalized tools and modern assistant/tool continuation
      messages with the provider's original tool-call ids.
- [x] Map `ThinkingLevel::Disabled` to MiniMax `disabled`, every enabled level
      to `adaptive`, and record the selected normalized level in responses.
- [x] Parse MiniMax reasoning into typed provider context, attach it to the
      response, and replay only MiniMax-owned context on later turns.
- [x] Parse tool arguments through shared helpers and expose calls only for
      `FinishReason::ToolCalls`.
- [x] Run the MiniMax test suite plus targeted
      `ai-tool-calling` conversation/operation-id tests, formatting, and
      provider Clippy.

## Milestone 4: Finish Safety, Usage, Structured Output, And Vision

Complete provider behavior and edge-case safety. At the end of this milestone,
the catalog's advertised capabilities and normalized failure/usage contracts
are covered by regression tests.

- [x] Add failing finish tests for `stop`, `tool_calls`, `length`,
      `content_filter`, unknown, and missing reasons, including suppression of
      tool payloads on every non-tool finish.
- [x] Add failing response-shape tests for no choices, null/empty content,
      absent usage, and malformed typed fields.
- [x] Add failing usage tests for prompt, completion, total, cached, and
      reasoning tokens, including saturating non-overlapping buckets and
      total reconstruction.
- [x] Add failing `base_resp` tests for zero/missing success and documented
      rate-limit, transient, context-limit, auth, balance, parameter, and
      content-policy failure classes on HTTP-success responses.
- [x] Implement MiniMax provider-code classification while retaining the
      numeric code and provider message; continue using the shared classifier
      for HTTP failures.
- [x] Add failing structured-output tests for schema instructions, successful
      local validation, invalid JSON, schema mismatch, and no parsing on
      tool/terminal finishes.
- [x] Implement schema prompting and shared local JSON Schema validation
      without claiming native provider schema enforcement.
- [x] Add failing multimodal tests for ordered text/image parts and base64 data
      URLs; implement shared image serialization and keep video outside V1.
- [x] Confirm M3 advertises vision while M2.7 entries do not, and that every
      model advertises only capabilities exercised by the adapter tests.
- [x] Run all provider, interface, core error/structured-output, and relevant
      tool-runtime tests with a 100% pass rate.

## Milestone 5: Workspace Integration And Documentation

Make MiniMax discoverable and exercise construction in the standard developer
workflow. At the end of this milestone, a downstream composition root can
find, construct, wrap, and validate the provider without reading source.

- [x] Export the provider crate and catalog API from their real owner modules;
      do not add pass-through compatibility modules.
- [x] Construct a MiniMax model with a placeholder credential in
      `cargo xtask smoke-test` without performing a live API request.
- [x] Update xtask smoke imports/dependencies and its README provider list.
- [x] Update the root README feature list, interface map, key-code jumping
      points, protocol links, and credential-free check description.
- [x] Review every changed crate README and protocol statement against the
      implemented public API, model ids, endpoint, replay behavior, and known
      limitations.
- [x] Change the MiniMax protocol status from planned to implemented only after
      all behavior and verification are complete.
- [x] Move this plan from active to completed in `plans/README.md` only after
      every earlier milestone and final verification item is complete.
- [x] Run `cargo xtask smoke-test` and confirm it requires no provider
      credentials or network calls.

## Milestone 6: Full Verification, Commit, Push, And Review

Validate and publish the complete provider. At the end of this milestone, the
branch is pushed with all checks passing and review findings are ready for the
user to assess.

- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      repeat the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [x] Run `cargo xtask smoke-test`.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Add a regression test and omit unavailable MiniMax message content
      instead of serializing a JSON `null`.
- [x] Clarify the protocol boundary between provider-semantic response errors
      and internal typed-deserialization failures.
- [x] Re-run `cargo xtask check` after the final diff-audit fixes.
- [x] Review `git diff origin/main...` for unrelated changes, missing new
      files, reasoning leakage, stale provider names, undocumented public API,
      and generated/lockfile drift.
- [x] Run `git add -A` so every source, test, README, protocol, plan, and
      lockfile change is tracked.
- [x] Commit the completed work using a Conventional Commit title no longer
      than 50 characters and a descriptive body.
- [x] Push the current branch.
- [x] Run `cargo xtask review` after the push so review compares the complete
      branch with `origin/main`.
- [x] Do not auto-fix review findings; report each item with a number, severity,
      codebase context, impact of doing nothing, lettered solution options, and
      a recommended option.
