# Universal Tool Output Management

## Summary

Move tool-output identification and pagination into the shared tool-calling
runtime. Small results return as inline envelopes without ids; every larger
successful result is retained in an injected, bounded output store and
returned as a UTF-8-safe window with an opaque output id. Models read later
windows through the runtime-intrinsic `tool_output_read` tool. A successful
output that cannot be stored is never dropped: the model receives a degraded
first window with a `remainder_unavailable` reason, because the tool's side
effects already happened.

Individual `Tool` implementations remain unaware of output ids and pagination.
The implementation must keep raw current-run results available to internal
consumers while ensuring model history and normal logs receive only bounded
model-visible envelopes.

The normative contract is
[Tool Output Management](../docs/protocol/tool-output-management.md).

## Scope

This plan covers the reusable contracts and runtime in this repository:

- `ai-interface` output DTOs and logger contract
- `ai-tool-calling` policy, store boundary, in-memory implementation, intrinsic
  reader, dispatch integration, execution records, tests, and smoke coverage
- Workspace and crate documentation for the new public API

Juno's `fna-agent` and `fna-apps` crates are downstream consumers and are not
present in this repository. Their required migration is recorded in the
handoff section so it can be implemented in a separate Juno plan after this API
lands.

## Milestone 1: Shared Output Contracts

Add the model-visible types without changing dispatch behavior. At the end of
this milestone, downstream code can compile against a typed, serialization-
tested output protocol while existing tool execution remains functional.

- [x] Add failing serde tests for inline envelopes without output ids, window
      envelopes, degraded windows carrying each `remainder_unavailable`
      reason, output ids, read requests, omitted `next_offset`, and invalid
      DTO states, including a truncated window with both or neither of
      `next_offset` and `remainder_unavailable` and an `output_id` present
      alongside `remainder_unavailable`.
- [x] Add a `ToolOutputId` newtype with opaque string serialization.
- [x] Add typed inline and window output DTOs with the protocol field names and
      tagged `tool_output` / `tool_output_window` representations; the inline
      DTO has no output id, and the window DTO enforces that `output_id` is
      present exactly when `remainder_unavailable` is absent.
- [x] Add the typed `remainder_unavailable` reason enum with the
      `output_too_large`, `budget_exhausted`, and `store_unavailable` values.
- [x] Add the typed `ToolOutputReadRequest` DTO used by the intrinsic reader.
- [x] Export the new contracts from `ai-interface` at their real owner paths.
- [x] Keep `Tool::call()` and `Tool::call_with_invocation()` returning
      `ToolResult<Value>` so individual tools remain pagination-agnostic.
- [x] Update `ai-interface` documentation with the raw versus model-visible
      boundary.
- [x] Run `cargo fmt --all -- --check` and `cargo test -p ai-interface`.

## Milestone 2: Policy And Output Store

Build the reusable storage seam independently of turn dispatch. At the end of
this milestone, an injected store can retain and page bounded output with
deterministic tests and no unbounded growth.

- [x] Add failing tests for policy validation, successful writes, unavailable
      ids, scope isolation, UTF-8 boundaries, end-of-output reads, invalid
      offsets and lengths, failed-write rollback, and atomic aggregate-budget
      rejection.
- [x] Add `ToolOutputPolicy` with the protocol defaults: 20,000-byte inline and
      read windows, 1,048,576 bytes per stored output, and 16,777,216 bytes
      per runtime, documenting that only windowed outputs are stored and
      counted against the aggregate budget.
- [x] Define the async `ToolOutputStore` trait, dynamic alias, typed store
      requests/results, and its `thiserror` error contract in
      `ai-tool-calling`, with distinct typed errors for per-output overflow,
      aggregate exhaustion, and write failure so dispatch can map each to its
      `remainder_unavailable` reason.
- [x] Generate the store mock with `unimock` and use it for runtime-facing unit
      tests at the trait boundary.
- [x] Add `InMemoryToolOutputStore` behind the trait with atomic budget
      accounting, no eviction, and opaque `toolout_` ids.
- [x] Use `cargo add` for any new external id-generation dependency rather than
      editing a guessed dependency version into `Cargo.toml`.
- [x] Keep production modules and source-adjacent `_tests_` files below the
      300-line hard limit by splitting policy, contracts, store, and windowing
      into cohesive modules.
- [x] Update the `ai-tool-calling` README with store ownership and lifetime.
- [x] Run `cargo fmt --all -- --check`,
      `cargo test -p ai-tool-calling`, and
      `cargo clippy -p ai-tool-calling --all-targets --all-features`.

## Milestone 3: Intrinsic Output Reader

Expose stored windows to the model without routing the read through a normal
tool implementation. At the end of this milestone, every runtime advertises a
reserved reader that preserves the original id and cannot recursively grow the
store.

- [x] Add failing tests for automatic reader registration, its JSON schema,
      reserved-name collisions, replacement of user tools, default read
      arguments, capped lengths, and typed read failures.
- [x] Reserve `tool_output_read` in the tool catalog and reject injected tools
      that declare that name.
- [x] Write the reader's tool description per the protocol's model guidance:
      read further windows only when the task requires them, prefer narrowing
      the original query at its source, and note that offsets and lengths are
      bytes, not tokens.
- [x] Always include the intrinsic definition in model request snapshots,
      including after `replace_tools()`.
- [x] Extend runtime composition to require an injected
      `DynToolOutputStore` and validated `ToolOutputPolicy`; update every call
      site and test fixture.
- [x] Add a `replace_output_store()` runtime API mirroring `replace_tools()`
      so hosts that reuse one runtime across successive runs can swap in a
      fresh store at the run boundary; test that ids from the replaced store
      become unavailable.
- [x] Dispatch `tool_output_read` before ordinary tool lookup and read directly
      from the output store.
- [x] Return a `tool_output_window` using the requested id without writing a
      second output, allocating another id, or recursively wrapping the
      response.
- [x] Make the unavailable-output error text state that the output is no
      longer available and that the original tool call itself succeeded,
      advising a re-run only when the tool is read-only or otherwise safe to
      repeat and user confirmation before repeating a side-effecting call;
      add a test asserting this wording contract.
- [x] Preserve existing activity lifecycle, operation-id, error-message, and
      one-result-per-provider-call behavior for the intrinsic tool.
- [x] Run the targeted reader/catalog tests, `cargo fmt --all -- --check`, and
      `cargo clippy -p ai-tool-calling --all-targets --all-features`.

## Milestone 4: Universal Dispatch Windowing

Apply output management to every successful ordinary tool call. At the end of
this milestone, tool execution is universally bounded while raw current-run
results remain available to internal orchestration.

- [x] Add failing regression tests for a small success, an exact
      20,000-byte inline output, multi-window output, per-output overflow,
      aggregate overflow, store write failure, ordinary tool errors, and a
      handled `{ "ok": false }` result inspected through `raw_output`.
- [x] Serialize and measure each successful raw output once; return inline
      envelopes without touching the store, and store only outputs above the
      inline limit before constructing their first window.
- [x] Assert in the small-success test that inline outputs are never written
      to the store and never consume aggregate budget.
- [x] Degrade instead of dropping when a successful output cannot be stored:
      append a degraded first window with the correct
      `remainder_unavailable` reason for per-output overflow, aggregate
      exhaustion, and store write failure, treat it as a successful result,
      and emit a `tracing` diagnostic.
- [x] Keep typed runtime errors for policy validation and intrinsic-read
      failures; `run()` continues after recoverable read failures.
- [x] Make budget-exhaustion tests order-agnostic: assert that some sibling
      call degrades without pinning which one.
- [x] Change `ToolExecutionRecord` to expose an optional `output_id`,
      `raw_output`, and `model_visible_output` explicitly.
- [x] Append only `model_visible_output` to conversation state and pass only
      that bounded value through `ToolCallLogResult::Success`.
- [x] Ensure intrinsic reads retain the requested id and use their returned
      window as both execution-record representations.
- [x] Verify multi-call batches still append exactly one tool-role message for
      every provider tool-call id when sibling calls fail or degrade.
- [x] Run all `ai-tool-calling` tests and targeted provider request/replay tests
      that consume retained tool messages.
- [x] Extend `cargo xtask smoke-test` with a credential-free multi-window tool
      output flow that follows `next_offset` to completion.

## Milestone 5: Public API And Migration Documentation

Finish the reusable API and document its downstream boundary. At the end of
this milestone, a consumer can compose a correctly scoped runtime and migrate
from direct tool JSON without reading implementation code.

- [x] Update `ai-interface` and `ai-tool-calling` crate READMEs, including
      responsibilities, behavior, a compiling Quick Start, development
      commands, key code, and related protocol links.
- [x] Update the workspace README features, interfaces, and key-code map with
      universal output management and the protocol link.
- [x] Change the protocol status from planned to implemented only after the
      runtime behavior and verification are complete.
- [x] Document that the default in-memory store is fresh per active run, ids
      expire with the store, a store must never be shared across runs that
      belong to different principals, hosts reusing one runtime across runs
      must call `replace_output_store()` at the run boundary, and durable ids
      require a host-owned protected storage design.
- [x] Document the observability consequence: after a run ends the raw output
      is gone everywhere, normal logs only ever contain the bounded envelope,
      and hosts needing raw capture must take it from step execution records
      or a custom store under their own retention and redaction policy.
- [x] Document that redaction must be applied to raw output before the store
      write and envelope construction, because window content is an opaque
      byte range that cannot be redacted afterward.
- [x] Document mixed-format history: conversations persisted before adoption
      contain raw tool JSON, `replace_conversation` accepts arbitrary content,
      and consumers must not assume every tool message is an envelope.
- [x] Document that hosts should avoid suspending a run mid-pagination, may
      rewrite stale persisted window envelopes before replay, and should size
      `max_steps` to budget for pagination rounds.
- [x] Document the breaking constructor, logger payload, conversation envelope,
      and `ToolExecutionRecord` changes for downstream consumers.
- [x] Bump the pre-1.0 workspace minor version and regenerate `Cargo.lock` so
      the breaking public API is not released under `0.2.x`.
- [x] Confirm every changed public Rust item has rustdoc and every changed Rust
      module has module-level documentation.

## Milestone 6: Workspace Verification, Commit, And Review

Validate and publish the completed implementation. At the end of this
milestone, the branch is pushed with all checks passing and AI review findings
are ready for the user to assess.

- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      repeat the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [x] Run `cargo xtask smoke-test` and exercise the universal pagination path.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for unrelated changes, leaked raw output,
      generated artifacts, missing files, and stale app-only terminology.
- [x] Run `git add -A` so all new protocol, source, test, snapshot, lock, and
      documentation files are tracked.
- [x] Commit the completed work using a Conventional Commit title no longer
      than 50 characters and a descriptive body.
- [x] Push the current branch.
- [x] Run `cargo xtask review` after the push so review compares the complete
      branch with `origin/main`.
- [x] Do not auto-fix review findings; report each item with a number, severity,
      codebase context, impact of doing nothing, lettered solution options, and
      a recommended option.

## Downstream Juno Handoff

The follow-up Juno plan must use the released generic API rather than recreate
it locally:

- `fna-agent` creates and injects one fresh output store per active agent run
  and owns any later durable-store, retention, authorization, and redaction
  policy; stores are never shared across runs or principals.
- `fna-agent` persists only model-visible envelopes. Durable handled-failure
  status, image-context selection, prompt/event trace linking, and other
  top-level field inspection move to raw current-run execution records.
- `fna-agent` applies any field-level redaction to raw output before results
  reach the store or envelope, and decides whether it needs a protected
  raw-capture sink for debugging and audit now that normal logs and journals
  only carry bounded envelopes.
- `fna-agent` handles stale pagination across suspends: avoid suspending a run
  mid-pagination where possible, and rewrite persisted window envelopes
  (dropping `next_offset`) before replaying history into a new run, or
  explicitly document that stale reads fail with the expired-output error.
- `fna-apps` removes `AppOutputStore`, `InMemoryAppOutputStore`,
  `AppOutputReadTool`, `app_output_read`, and app-specific window wrapping in
  the same change that adopts the shared runtime, so app output is never
  double-wrapped.
- `fna-apps` retains app/Wasm raw-response enforcement, manifest overrides,
  and upstream/provider pagination; app tools return their raw schema-bound
  JSON to the shared runtime.
- Juno's app protocol, crate READMEs, reserved tool names, tool selection,
  durable-journal tests, trace tests, image-context tests, and app-output tests
  are updated together in that repository.
- Migration tests prove no hidden oversized raw value reaches model history,
  Postgres, ordinary logs, public chat, or audit metadata, and that every app
  output is wrapped exactly once with no envelope-of-envelope or second
  reader.
