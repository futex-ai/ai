# Add Kimi Model Provider

## Summary

Add a first-class `ai-models-kimi` crate for Kimi K3. The adapter will use
Moonshot AI's non-streaming Chat Completions API while implementing the shared
model, routing, tool-calling, structured-output, replay, usage, and error
contracts.

The normative behavior is defined by the
[Kimi Model Provider Protocol](../docs/protocol/kimi-model-provider.md).

## Scope

This change includes:

- a typed `kimi` routing provider and Kimi assistant replay context in
  `ai-interface`;
- Kimi K3 low-, high-, and max-reasoning catalog entries;
- a new `ai-models-kimi` provider crate with injected HTTP and auth seams;
- text, image, structured-output, custom-tool, parallel-tool, and continuation
  behavior;
- credential-free tests, smoke construction, and public documentation.

K2.x and Moonshot V1 models, streaming, Partial Mode, video/file upload,
dynamic and official tools, cache-key tuning, safety identifiers, and live
credential-dependent tests are out of scope.

## Milestone 1: Shared Routing And Replay Contracts

Extend shared types before adding provider runtime behavior. At the end of
this milestone, downstream code can represent Kimi routes and losslessly
retain a Kimi assistant message without affecting existing providers.

- [x] Add failing `ai-interface` tests for `ProviderKind::Kimi` config parsing,
      display, JSON serialization, and JSON deserialization.
- [x] Add `ProviderKind::Kimi` with stable config value `kimi` and update all
      exhaustive provider matches.
- [x] Add failing serde tests for the Kimi assistant replay item, including
      nullable content, nullable reasoning content, ordered parallel raw tool
      calls, and omitted optional fields.
- [x] Add a documented Kimi raw tool-call DTO and
      `ProviderConversationItem::KimiAssistantMessage` at the real
      `ai-interface` owner path.
- [x] Extend synthetic tool-call scope hashing with every Kimi replay field and
      add regression coverage proving context changes alter the scope.
- [x] Update OpenAI and xAI request mappers to ignore Kimi-owned context, with
      regression coverage that existing provider replay remains unchanged.
- [x] Update `ai-interface/README.md` with the Kimi replay boundary and the
      requirement that provider reasoning is retained but not surfaced as
      assistant text.
- [x] Run `cargo fmt --all -- --check`,
      `cargo test -p ai-interface`, and
      `cargo test -p ai-models-core`.

## Milestone 2: Kimi Crate And One-Turn Completion

Create the provider crate and complete one-turn text, vision, structured
output, catalog, and usage behavior. At the end of this milestone, a caller
can construct a Kimi K3 model and complete non-tool requests through the shared
`Model` trait.

- [x] Scaffold `crates/ai-models-kimi` with a thin `lib.rs`, cohesive catalog,
      client, request, request-type, response, and source-adjacent `_tests_`
      modules; keep every Rust file under 300 lines.
- [x] Add `ai-models-kimi` to workspace members and workspace dependencies
      without introducing duplicated external dependency versions.
- [x] Add failing catalog tests for the exact K3 catalog ids, shared provider
      id, 1,000,000-token context window, feature flags, intelligence, speed,
      cost, provider model id, and low/high/max thinking metadata.
- [x] Implement `KIMI_K3`, `KIMI_K3_THINKING_HIGH`,
      `KIMI_K3_THINKING_LOW`, and `known_models()` exactly as specified.
- [x] Add failing construction tests for the injected HTTP/auth boundaries and
      rejection of unsupported provider model ids or thinking levels.
- [x] Implement `KimiModel` behind `ai_interface::Model`, using explicit
      credentials and the fixed Moonshot Chat Completions endpoint without
      reading ambient config or environment variables.
- [x] Add failing request tests for the leading system message, every shared
      conversation role, optional names, plain content, image data URLs,
      foreign provider-context isolation, and exact low/high/max
      `reasoning_effort` mapping.
- [x] Serialize K3 requests without fixed sampling parameters, K2 `thinking`,
      streaming, partial, file, video, or unsupported platform fields.
- [x] Add failing structured-output tests for the non-strict
      `response_format` request, successful local JSON Schema validation,
      invalid JSON, schema mismatch, and non-stop finish handling.
- [x] Add failing response tests for nullable content, missing choices,
      malformed payloads, all normalized finish reasons, catalog/provider ids,
      and suppression of reasoning content from normalized assistant text.
- [x] Add failing usage tests for cached-input subtraction, missing usage,
      missing provider totals, saturating arithmetic, and the absence of a
      separately reported reasoning-token quantity.
- [x] Implement response and usage mapping without adding mutable price tables
      to the provider crate.
- [x] Add `crates/ai-models-kimi/README.md` with the required crate README
      sections, a compiling K3 Quick Start, supported behavior, exclusions,
      key code, and related docs.
- [x] Run `cargo fmt --all -- --check`,
      `cargo test -p ai-models-kimi`, and
      `cargo clippy -p ai-models-kimi --all-targets --all-features`.

## Milestone 3: Tool Calls And Preserved Thinking

Add complete agent-loop behavior after basic completions work. At the end of
this milestone, Kimi can issue parallel custom tool calls and continue from
their results with its exact prior assistant reasoning and raw calls intact.

- [x] Add failing request tests for custom function definitions,
      `tool_choice: "auto"`, omitted provider strict mode, multiple normalized
      assistant tool calls, and tool-result messages that contain
      `tool_call_id` but omit `name`.
- [x] Add failing response tests for ordered parallel tool calls, raw JSON
      argument preservation, invalid argument JSON, provider ids, and
      `finish_reason: "tool_calls"`.
- [x] Parse and expose calls only for `FinishReason::ToolCalls`; add regression
      tests proving stop, length, content-filter, custom, and missing finish
      reasons suppress otherwise valid tool payloads.
- [x] Capture nullable raw assistant content, raw reasoning content, and raw
      tool calls in `ProviderConversationItem::KimiAssistantMessage`.
- [x] Add continuation tests that round-trip the exact Kimi assistant item
      before every matching tool result, including whitespace-sensitive raw
      argument strings and multiple calls.
- [x] Prefer Kimi replay context for provider-produced assistant messages,
      fall back to normalized fields for caller-authored messages, and ignore
      replay items owned by other providers.
- [x] Verify the tool-calling runtime retains the Kimi provider context on
      assistant conversation messages without exposing reasoning as visible
      text or tool output.
- [x] Run Kimi request/response/continuation tests,
      `cargo test -p ai-tool-calling`, and existing OpenAI and xAI replay
      regression tests.

## Milestone 4: Errors, Smoke Coverage, And Documentation

Harden failure behavior and integrate the provider into workspace-facing
surfaces. At the end of this milestone, Kimi construction is covered by the
credential-free smoke test and consumers can discover and use the crate from
the public documentation.

- [x] Add mocked transport tests for bearer auth, the exact endpoint, 429 rate
      limits, transient statuses, ordinary provider statuses, transport/auth
      failures, request serialization failures, and malformed responses;
      assert credentials never appear in errors.
- [x] Use shared HTTP error classification and structured parsing helpers
      without matching behavior on provider error-message substrings.
- [x] Add `ai-models-kimi` to `xtask` and construct `KIMI_K3` in the
      credential-free smoke test without making a live API request.
- [x] Update the workspace `README.md` feature list, interface map, key-code
      pointers, and protocol links for Kimi.
- [x] Update neighboring crate READMEs where the supported provider or replay
      boundary changes.
- [x] Change the Kimi protocol status from planned to implemented only after
      all contract behavior is present.
- [x] Review all changed public Rust items for rustdoc, all changed modules for
      module-level docs, import ordering, narrow visibility, typed errors, and
      trait-backed side-effect boundaries.
- [x] Run `cargo xtask smoke-test` and confirm it remains credential-free.

## Milestone 5: Workspace Verification, Commit, And Review

Validate and publish the completed change. At the end of this milestone, all
checks pass, the branch is pushed, and review findings are ready for the user
to assess without automatic fixes.

- [x] Update `plans/README.md` to move this plan from active to completed.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      repeat the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [x] Run `cargo xtask smoke-test`.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for unrelated changes, missing tests,
      stale documentation, untracked files, credentials, and generated
      artifacts.
- [x] Run `git add -A`.
- [x] Commit the completed implementation with a Conventional Commit message
      whose title is at most 50 characters and whose body summarizes behavior
      and verification.
- [x] Push the current branch without renaming it.
- [x] Run `cargo xtask review` after the push so the AI reviewer checks the
      committed diff against `origin/main`.
- [x] Report every review finding without automatically fixing it; number each
      item, assign severity, explain provider/codebase context and impact of
      doing nothing, give lettered solution options, and recommend one option.

## Milestone 6: Review-Hardened Terminal Tool Suppression

Address the valid post-implementation review finding without weakening strict
validation for dispatchable calls. At the end of this milestone, structurally
partial Kimi tool payloads are ignored for terminal finish reasons and still
rejected when Kimi marks them as dispatchable.

- [x] Add a failing regression test proving terminal responses suppress a
      structurally partial tool call before strict tool-call decoding.
- [x] Retain Kimi tool calls as raw response-boundary data until finish-reason
      handling, then decode them only for `FinishReason::ToolCalls`.
- [x] Add regression coverage proving structurally partial dispatchable calls
      remain typed provider errors.
- [x] Add a failing regression test for dispatchable responses with omitted,
      null, or empty tool-call payloads.
- [x] Reject empty dispatchable tool-call responses at the provider boundary
      while retaining non-dispatchable suppression.
- [x] Keep structured-output finish handling covered with a valid tool-call
      response instead of an internally inconsistent empty response.
- [x] Keep finish-reason normalization coverage internally consistent by
      attaching a valid call to the `tool_calls` case.
- [x] Run focused Kimi tests, formatting, file-length lint, Clippy, full
      workspace tests, smoke tests, and `cargo xtask check`.
- [x] Commit the review fix with a Conventional Commit message and push the
      current branch.
- [x] Repeat `cargo xtask review` and resolve every remaining valid finding
      within the review-cycle limit.

## Milestone 7: Private Logging And Empty Content

Close the valid review gaps found after merging the latest target branch. At
the end of this milestone, Kimi replay reasoning stays available to the model
without entering model-call logs, and empty user/tool messages remain valid
Chat Completions strings.

- [x] Add failing success- and error-path regression tests proving Kimi replay
      context reaches the model and retained conversation but is removed from
      every model-call logger copy.
- [x] Generalize the model-call logging redaction boundary to remove private
      Kimi and MiniMax replay items without mutating live conversation state.
- [x] Add failing request regressions for empty user and tool content.
- [x] Serialize empty user and tool content as strings while preserving
      nullable content for Kimi assistant replay.
- [x] Clarify the Kimi protocol and tool-calling README logging/content
      contracts.
- [x] Run focused regressions, formatting, file-length lint, strict workspace
      Clippy, full all-feature workspace tests, the credential-free smoke test,
      and `cargo xtask check`.
- [ ] Commit the review fixes with a Conventional Commit message and push the
      current branch.
- [ ] Repeat `cargo xtask review` and resolve every remaining valid finding
      within the review-cycle limit.
