# Universal Tool Output Management

## Summary

Move tool-output identification and pagination into the shared tool-calling
runtime. Every successful ordinary tool call will retain its complete JSON in
an injected, bounded output store and return either an inline envelope or a
UTF-8-safe window with an opaque output id. Models will read later windows
through the runtime-intrinsic `tool_output_read` tool.

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

- [ ] Add failing serde tests for inline envelopes, window envelopes, output
      ids, read requests, omitted `next_offset`, and invalid DTO states.
- [ ] Add a `ToolOutputId` newtype with opaque string serialization.
- [ ] Add typed inline and window output DTOs with the protocol field names and
      tagged `tool_output` / `tool_output_window` representations.
- [ ] Add the typed `ToolOutputReadRequest` DTO used by the intrinsic reader.
- [ ] Export the new contracts from `ai-interface` at their real owner paths.
- [ ] Keep `Tool::call()` and `Tool::call_with_invocation()` returning
      `ToolResult<Value>` so individual tools remain pagination-agnostic.
- [ ] Update `ai-interface` documentation with the raw versus model-visible
      boundary.
- [ ] Run `cargo fmt --all -- --check` and `cargo test -p ai-interface`.

## Milestone 2: Policy And Output Store

Build the reusable storage seam independently of turn dispatch. At the end of
this milestone, an injected store can retain and page bounded output with
deterministic tests and no unbounded growth.

- [ ] Add failing tests for policy validation, successful writes, unavailable
      ids, scope isolation, UTF-8 boundaries, end-of-output reads, invalid
      offsets and lengths, failed-write rollback, and aggregate exhaustion.
- [ ] Add `ToolOutputPolicy` with the protocol defaults: 20,000-byte inline and
      read windows, 1,048,576 bytes per output, and 16,777,216 bytes per
      runtime.
- [ ] Define the async `ToolOutputStore` trait, dynamic alias, typed store
      requests/results, and its `thiserror` error contract in
      `ai-tool-calling`.
- [ ] Generate the store mock with `unimock` and use it for runtime-facing unit
      tests at the trait boundary.
- [ ] Add `InMemoryToolOutputStore` behind the trait with atomic budget
      accounting, no eviction, and opaque `toolout_` ids.
- [ ] Use `cargo add` for any new external id-generation dependency rather than
      editing a guessed dependency version into `Cargo.toml`.
- [ ] Keep production modules and source-adjacent `_tests_` files below the
      300-line hard limit by splitting policy, contracts, store, and windowing
      into cohesive modules.
- [ ] Update the `ai-tool-calling` README with store ownership and lifetime.
- [ ] Run `cargo fmt --all -- --check`,
      `cargo test -p ai-tool-calling`, and
      `cargo clippy -p ai-tool-calling --all-targets --all-features`.

## Milestone 3: Intrinsic Output Reader

Expose stored windows to the model without routing the read through a normal
tool implementation. At the end of this milestone, every runtime advertises a
reserved reader that preserves the original id and cannot recursively grow the
store.

- [ ] Add failing tests for automatic reader registration, its JSON schema,
      reserved-name collisions, replacement of user tools, default read
      arguments, capped lengths, and typed read failures.
- [ ] Reserve `tool_output_read` in the tool catalog and reject injected tools
      that declare that name.
- [ ] Always include the intrinsic definition in model request snapshots,
      including after `replace_tools()`.
- [ ] Extend runtime composition to require an injected
      `DynToolOutputStore` and validated `ToolOutputPolicy`; update every call
      site and test fixture.
- [ ] Dispatch `tool_output_read` before ordinary tool lookup and read directly
      from the output store.
- [ ] Return a `tool_output_window` using the requested id without writing a
      second output, allocating another id, or recursively wrapping the
      response.
- [ ] Preserve existing activity lifecycle, operation-id, error-message, and
      one-result-per-provider-call behavior for the intrinsic tool.
- [ ] Run the targeted reader/catalog tests, `cargo fmt --all -- --check`, and
      `cargo clippy -p ai-tool-calling --all-targets --all-features`.

## Milestone 4: Universal Dispatch Windowing

Apply output management to every successful ordinary tool call. At the end of
this milestone, tool execution is universally bounded while raw current-run
results remain available to internal orchestration.

- [ ] Add failing regression tests for a small success, exact 20,000-byte
      output, multi-window output, per-output overflow, aggregate overflow,
      store failure, ordinary tool errors, and a handled `{ "ok": false }`
      result.
- [ ] Serialize and measure each successful raw output once, reserve aggregate
      capacity, and store every accepted result before constructing its
      model-visible envelope.
- [ ] Return a structured inline envelope for output at or below the inline
      limit and an initial UTF-8-safe window above it.
- [ ] Add typed runtime errors for policy, size, capacity, store, and read
      failures; append a bounded tool-error message and make `run()` continue
      after recoverable output-management failures.
- [ ] Change `ToolExecutionRecord` to expose `output_id`, `raw_output`, and
      `model_visible_output` explicitly.
- [ ] Append only `model_visible_output` to conversation state and pass only
      that bounded value through `ToolCallLogResult::Success`.
- [ ] Ensure intrinsic reads retain the requested id and use their returned
      window as both execution-record representations.
- [ ] Verify multi-call batches still append exactly one tool-role message for
      every provider tool-call id when sibling calls fail.
- [ ] Run all `ai-tool-calling` tests and targeted provider request/replay tests
      that consume retained tool messages.
- [ ] Extend `cargo xtask smoke-test` with a credential-free multi-window tool
      output flow that follows `next_offset` to completion.

## Milestone 5: Public API And Migration Documentation

Finish the reusable API and document its downstream boundary. At the end of
this milestone, a consumer can compose a correctly scoped runtime and migrate
from direct tool JSON without reading implementation code.

- [ ] Update `ai-interface` and `ai-tool-calling` crate READMEs, including
      responsibilities, behavior, a compiling Quick Start, development
      commands, key code, and related protocol links.
- [ ] Update the workspace README features, interfaces, and key-code map with
      universal output management and the protocol link.
- [ ] Change the protocol status from planned to implemented only after the
      runtime behavior and verification are complete.
- [ ] Document that the default in-memory store is fresh per active run, ids
      expire with the store, and durable ids require a host-owned protected
      storage design.
- [ ] Document the breaking constructor, logger payload, conversation envelope,
      and `ToolExecutionRecord` changes for downstream consumers.
- [ ] Bump the pre-1.0 workspace minor version and regenerate `Cargo.lock` so
      the breaking public API is not released under `0.2.x`.
- [ ] Confirm every changed public Rust item has rustdoc and every changed Rust
      module has module-level documentation.

## Milestone 6: Workspace Verification, Commit, And Review

Validate and publish the completed implementation. At the end of this
milestone, the branch is pushed with all checks passing and AI review findings
are ready for the user to assess.

- [ ] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      repeat the check.
- [ ] Run `cargo xtask rust-file-length-lint --all`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features`.
- [ ] Run `cargo test --workspace --all-features` and require a 100% pass rate.
- [ ] Run `cargo xtask smoke-test` and exercise the universal pagination path.
- [ ] Run `cargo xtask check` and fix failures until it passes.
- [ ] Review `git diff origin/main...` for unrelated changes, leaked raw output,
      generated artifacts, missing files, and stale app-only terminology.
- [ ] Run `git add -A` so all new protocol, source, test, snapshot, lock, and
      documentation files are tracked.
- [ ] Commit the completed work using a Conventional Commit title no longer
      than 50 characters and a descriptive body.
- [ ] Push the current branch.
- [ ] Run `cargo xtask review` after the push so review compares the complete
      branch with `origin/main`.
- [ ] Do not auto-fix review findings; report each item with a number, severity,
      codebase context, impact of doing nothing, lettered solution options, and
      a recommended option.

## Downstream Juno Handoff

The follow-up Juno plan must use the released generic API rather than recreate
it locally:

- `fna-agent` creates and injects one fresh output store per active agent run
  and owns any later durable-store, retention, authorization, and redaction
  policy.
- `fna-agent` persists only model-visible envelopes. Durable handled-failure
  status, image-context selection, prompt/event trace linking, and other
  top-level field inspection move to raw current-run execution records.
- `fna-apps` removes `AppOutputStore`, `InMemoryAppOutputStore`,
  `AppOutputReadTool`, `app_output_read`, and app-specific window wrapping.
- `fna-apps` retains app/Wasm raw-response enforcement, manifest overrides,
  and upstream/provider pagination; app tools return their raw schema-bound
  JSON to the shared runtime.
- Juno's app protocol, crate READMEs, reserved tool names, tool selection,
  durable-journal tests, trace tests, image-context tests, and app-output tests
  are updated together in that repository.
- Migration tests prove no hidden oversized raw value reaches model history,
  Postgres, ordinary logs, public chat, or audit metadata.
