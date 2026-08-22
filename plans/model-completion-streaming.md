# Model Completion Streaming

## Goal

Move model completion calls from buffered JSON responses to internally
accumulated SSE streams. Long reasoning and generation calls should tolerate
healthy activity, fail stalled streams promptly, avoid automatic replay after
partial generation, and preserve the existing `ModelResponse` contract.

The approved contract is
[`docs/protocol/model-completion-streaming.md`](../docs/protocol/model-completion-streaming.md).
Protocol and plan creation were approved by Cal in the 2026-08-22 handoff.

## Design Decisions

- Keep `Model::complete` buffered at the public boundary and stream only inside
  provider adapters.
- Use 10-second connect, 120-second idle, 3,600-second completion, and
  600-second buffered defaults.
- Name the new errors `Interrupted`, `IdleTimeout`, `DeadlineExceeded`, and
  `SseUnsupported`.
- Retry failures before stream progress, but never automatically retry an
  interruption after progress.
- Preserve `MultiModel`'s fall-through-on-any-error behavior and document the
  possible second bill.
- Require accurate in-stream usage; keep a provider buffered when its current
  API cannot supply it.
- Keep xAI deferred completion and `ai-mcp` SSE unchanged.

## Milestone 2: JSON HTTP Streaming Foundation

At the end of this milestone, injected and reqwest transports can return a
decoded, timeout-aware SSE stream without changing buffered callers.

- [x] Add failing pure decoder tests for LF, CRLF, standalone CR, mixed
      endings, multiline and colonless `data`, comments, ignored fields,
      multiple events, EOF dispatch, UTF-8, and every chunk-split boundary.
- [x] Implement `JsonHttpSseEvent` and the pure incremental decoder in focused
      modules below the Rust file cap.
- [x] Add failing unimock tests for `JsonHttpSseStream`, dynamic stream
      ownership, and the default `execute_sse` `SseUnsupported` behavior.
- [x] Add `JsonHttpRequestBuilder::send_sse` so auth hooks and typed JSON
      serialization use the same builder path as buffered requests.
- [x] Add `idle_timeout` to `JsonHttpRequest`, document `timeout` as the
      overall deadline, and retain compatibility for existing transports.
- [x] Add typed idle/deadline/content-type/decoder errors with event progress,
      plus a 64-KiB bounded HTTP-status diagnostic.
- [x] Add failing reqwest integration tests using a local SSE server for
      immediate event delivery, stalled open/reads, overall deadline,
      non-success JSON/text bodies, invalid content type, split chunks, and
      disconnects before and after progress.
- [x] Implement manual stream timers without reqwest's request timeout and add
      a 10-second client connect timeout for both execution paths.
- [x] Raise the buffered default timeout from 60 to 600 seconds.
- [x] Update `crates/json-http/README.md` and any clarified protocol details.
- [x] Run formatting, strict Clippy, the complete `json-http` tests, workspace
      tests, smoke tests, file-length lint, and `cargo xtask check` to 100%.
- [x] Review `git diff origin/main...`, stage all files, commit with a
      Conventional Commit, and push the current branch without renaming it.

## Milestone 3: Shared Model Support

At the end of this milestone, model wrappers can distinguish safe retries from
partial generations and compatible providers can share one tested accumulator.

- [x] Add failing `ai-interface` tests, then add documented
      `ModelError::Interrupted` fields and constructor behavior.
- [x] Add failing classification tests for HTTP status errors and transport,
      timeout, decode, EOF, and native errors before versus after progress.
- [x] Implement shared pure stream-error classification in `ai-models-core`.
- [x] Add failing fixture tests for content, reasoning, interleaved indexed
      tool calls, fragmented arguments, finish reason, usage, `[DONE]`, and
      malformed or incomplete chat-completions streams.
- [x] Implement the pure shared chat-completions accumulator below the file
      cap and shape its output for existing response mappers.
- [x] Prove `RetryingModel` does not retry `Interrupted` and `MultiModel` does
      fall through after it.
- [x] Update `ai-interface`, `ai-models-core`, and `ai-models-multi` READMEs,
      including the fallback re-billing consequence.
- [x] Run targeted and workspace formatting, strict Clippy, tests, smoke tests,
      file-length lint, and `cargo xtask check` to 100%.
- [x] Review the complete diff, stage all files, commit conventionally, and
      push the current branch.

## Milestone 4: Anthropic Streaming

At the end of this milestone, Anthropic text, tools, thinking, signatures, and
usage complete through SSE with buffered-response parity.

- [x] Add failing request tests for `"stream": true`, overall deadline, and
      idle timeout.
- [x] Add failing event fixtures for message start/delta/stop, every content
      block and delta type, fragmented tool JSON, ping, provider error, EOF,
      and interruption progress.
- [x] Implement the typed Anthropic accumulator and event loop through the
      existing response mapper.
- [x] Assert streamed fixtures equal buffered `ModelResponse` fixtures for
      text, tools, thinking/signatures, finish reason, context, and usage.
- [x] Add and run the ignored live streaming test when credentials are
      available; keep its credential-free guard passing otherwise.
- [x] Update the Anthropic README and protocol with verified behavior.
- [x] Run all targeted/workspace checks and `cargo xtask check` to 100%, then
      review the diff, stage, commit conventionally, and push.

## Milestone 5: OpenAI Responses Streaming

At the end of this milestone, OpenAI completions use stream liveness while the
existing parser remains the single owner of complete Responses mapping.

- [x] Add failing request and event tests for streaming, terminal completed,
      failed, incomplete, error, EOF, and interruption progress cases.
- [x] Implement event consumption and pass
      `response.completed.response` and `response.incomplete.response` through
      the buffered response mapper.
- [x] Assert parity for text, reasoning, provider-built tools, function calls,
      structured output, finish reason, provider context, and usage.
- [x] Add and run the credentialed live streaming test when available, with a
      passing credential-free guard.
- [x] Update the OpenAI README and protocol.
- [x] Run all targeted/workspace checks and `cargo xtask check` to 100%, then
      review the diff, stage, commit conventionally, and push.

## Milestone 6: Google Streaming

At the end of this milestone, Google completions merge streamed candidate
fragments without changing normalized response behavior.

- [x] Add failing endpoint tests for `:streamGenerateContent?alt=sse`, auth,
      stream timeouts, and the documented Firna URL-matching incompatibility.
- [x] Add failing fixtures for split text, complete function calls, thinking
      parts, final finish reason, usage metadata, provider errors, and EOF.
- [x] Implement typed fragment merging through the existing Google mapper.
- [x] Assert streamed and buffered response parity across text, tools,
      thinking/context, finish reason, structured output, and usage.
- [x] Add and run the credentialed live streaming test when available, with a
      passing credential-free guard.
- [x] Update the Google README and protocol.
- [x] Run all targeted/workspace checks and `cargo xtask check` to 100%, then
      review the diff, stage, commit conventionally, and push.

## Milestone 7: Chat-Completions Provider Family

At the end of this milestone, every compatible DeepSeek, Kimi, MiniMax,
QwenCloud, and synchronous xAI completion uses the shared stream accumulator;
any incompatible provider has a documented buffered exception.

- [x] Verify each provider's current official streaming and usage contract,
      exact opt-in field, terminal sentinel, reasoning fields, and known
      deviations; record dated findings in the protocol before implementation.
- [x] Extend the shared accumulator for standard error payloads, legacy xAI
      function calls, and Kimi's current direct cached-token usage field.
- [x] Normalize MiniMax cumulative stream fields into deltas while preserving
      complete reasoning details for replay.
- [x] For each provider, add failing request, delta, usage, error, EOF,
      interruption, and buffered-parity tests before changing production code.
- [x] Enable `"stream": true` and the provider-supported usage option, then
      route events through the shared accumulator and existing mapper.
- [x] Keep any provider without accurate in-stream usage buffered under the
      600-second timeout and document why; never synthesize usage.
- [x] Leave xAI deferred submit/poll behavior unchanged and prove it remains
      buffered.
- [x] Add and run one live streaming test per enabled provider when credentials
      are available, with all credential-free guards passing.
- [x] Update every affected provider README and the live-test protocol.
- [x] Run all targeted/workspace checks and `cargo xtask check` to 100%, then
      review the diff, stage, commit conventionally, and push.

## Milestone 8: Documentation, Verification, And Review

At the end of this milestone, the final implementation, protocol, crate docs,
and downstream handoff are aligned and independently reviewed.

- [x] Sweep the root and affected crate READMEs; align the protocol with every
      final error, timeout, provider usage flag, and buffered exception.
- [x] Confirm Firna's revision-bump notes prominently cover
      `BoundedModelHttpTransport::execute_sse`, Google
      `:streamGenerateContent` URL matching, `Interrupted`, and overall timeout
      semantics.
- [x] Run every targeted test, all credential-free live-test guards,
      `cargo fmt --all -- --check`, workspace strict Clippy, workspace tests,
      smoke tests, and `cargo xtask rust-file-length-lint --all` to 100%.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for scope, public API, docs, tests, and
      untracked files.
- [x] Stage every file with `git add -A`, commit with a Conventional Commit
      title of at most 50 characters and descriptive body, and push the current
      branch without renaming it.
- [ ] Run `cargo xtask review` only after the push so it reviews the local diff
      against `origin/main` (blocked on 2026-08-22 because the reviewer's Codex
      subprocess received `401 Unauthorized` before inspecting the diff).
- [x] Do not automatically fix review findings. Report each numbered with
      severity, codebase/feature context, impact of doing nothing, lettered
      solution options, and the recommended option.
- [ ] After the implementation is merged, mark the protocol Implemented and
      move this plan from Active to Completed in `plans/README.md`.
