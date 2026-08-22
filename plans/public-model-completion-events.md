# Public Model Completion Events

## Goal

Expose ordered assistant, reasoning, and fallback-restart events through the
public `ai-interface::Model` boundary while preserving the existing terminal
`ModelResponse` and unchanged `complete` behavior.

The approved contract is
[`docs/protocol/model-completion-events.md`](../docs/protocol/model-completion-events.md).
Protocol and plan creation were approved by Cal in the 2026-08-22 handoff.

## Design Decisions

- Name the opt-in entrypoint `complete_with_events` and the observer boundary
  `ModelCompletionEventSink`.
- Make sink delivery asynchronous, ordered, and infallible.
- Keep the event enum typed and non-exhaustive.
- Suppress all deltas for schema-constrained completions in version one.
- Emit provider-exposed reasoning text as a separate event.
- Emit a fallback restart only after public text from the failed lane and only
  when another lane will start.
- Preserve xAI deferred completion as a no-event buffered lifecycle.

## Milestone 1: Protocol And Plan

At the end of this milestone, the approved public event contract and its
implementation sequence are discoverable and consistent with the internal SSE
protocol.

- [x] Add the approved public completion-events protocol with API, parity,
      wrapper, provider, verification, and downstream Firna semantics.
- [x] Reconcile the internal streaming protocol with the new opt-in public
      boundary.
- [x] Add this plan to `plans/README.md` and link the protocol from the
      workspace README.
- [x] Validate the changed Markdown and review `git diff origin/main...`.
- [x] Stage all files, commit with a Conventional Commit, and push the current
      branch without renaming it.

## Milestone 2: Interface And Wrapper Semantics

At the end of this milestone, any `DynModel` supports the opt-in method and all
provider-agnostic wrappers preserve its coherent event sequence.

- [x] Add failing `ai-interface` tests for the typed event DTO, unimock-able
      sink, and default no-event implementation.
- [x] Add `ModelCompletionEvent`, `ModelCompletionEventSink`, and
      `Model::complete_with_events` with complete public documentation.
- [x] Add failing retry tests proving transient attempts leak no events and
      interruptions are not retried.
- [x] Add failing fallback tests for pre-delta failure, post-delta restart,
      final-lane failure, and nested restart tracking.
- [x] Add failing concurrency and usage-pricing pass-through tests.
- [x] Implement retry, fallback, concurrency, and pricing event semantics.
- [x] Update the interface, core, and multi crate READMEs.
- [x] Run formatting, strict Clippy, relevant crate tests, workspace tests,
      smoke tests, file-length lint, and `cargo xtask check` to 100%.
- [x] Review `git diff origin/main...`, stage all files, commit with a
      Conventional Commit, and push the current branch without renaming it.

## Milestone 3: Anthropic, OpenAI, And Google

At the end of this milestone, the three native completion protocols expose
assistant and provider-available reasoning events with terminal parity.

- [x] Add failing event-order, assistant-parity, reasoning, structured-output
      suppression, and interrupted-stream tests for Anthropic.
- [x] Emit Anthropic text and thinking deltas without changing accumulation.
- [x] Add the equivalent failing coverage for OpenAI Responses, including
      output-text and reasoning-summary delta events.
- [x] Emit OpenAI events while retaining terminal-object response parsing.
- [x] Add the equivalent failing coverage for Google candidate text and thought
      parts.
- [x] Emit Google events while retaining its current fragment merger.
- [x] Update the three provider crate READMEs and clarified protocol details.
- [x] Run formatting, strict Clippy, relevant crate tests, workspace tests,
      smoke tests, file-length lint, and `cargo xtask check` to 100%.
- [x] Review `git diff origin/main...`, stage all files, commit with a
      Conventional Commit, and push the current branch without renaming it.

## Milestone 4: Chat-Completions Providers

At the end of this milestone, the shared accumulator exposes deltas for all
five compatible synchronous adapters while xAI deferred calls stay silent.

- [ ] Add failing shared-accumulator tests for ordered assistant/reasoning
      observations without changing buffered output.
- [ ] Add an event-observing accumulator path and preserve its pure terminal
      response contract.
- [ ] Add failing parity, reasoning, suppression, and interruption coverage for
      DeepSeek, Kimi, QwenCloud, MiniMax, and synchronous xAI.
- [ ] Wire all five synchronous provider clients to the shared event path.
- [ ] Add xAI deferred no-event coverage.
- [ ] Update core and provider READMEs and clarified protocol details.
- [ ] Run formatting, strict Clippy, relevant crate tests, workspace tests,
      smoke tests, file-length lint, and `cargo xtask check` to 100%.
- [ ] Review `git diff origin/main...`, stage all files, commit with a
      Conventional Commit, and push the current branch without renaming it.

## Milestone 5: Live Coverage And Documentation Sweep

At the end of this milestone, credentialed tests exercise the public event
boundary and all user-facing crate guidance describes the shipped behavior.

- [ ] Add credential-free guards for synchronous event probes across all eight
      providers and xAI deferred silence.
- [ ] Extend credentialed live tests with assistant-delta observation and
      terminal parity for every synchronous streaming provider.
- [ ] Update `docs/protocol/live-model-api-tests.md` and the workspace README.
- [ ] Sweep every affected crate README for boundary, quick-start, key-code,
      and related-doc accuracy.
- [ ] Run formatting, strict Clippy, all tests, smoke tests, file-length lint,
      and `cargo xtask check` to 100%.
- [ ] Move this plan from Active to Completed in `plans/README.md`.
- [ ] Review `git diff origin/main...`, stage every file, commit the completed
      work with a Conventional Commit, and push the current branch.

## Closing Review

- [ ] After the final check, commit, and push, run `cargo xtask review` against
      `origin/main`.
- [ ] Report every review finding with a number, severity, context, impact,
      lettered solution options, and a recommended option; do not fix findings
      automatically.
