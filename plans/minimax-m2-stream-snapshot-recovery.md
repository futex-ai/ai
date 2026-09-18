# MiniMax M2.x Stream Snapshot Recovery

## Summary

The credentialed `MiniMax catalog` job fails intermittently with
`assistant response did not contain the probe marker` and, on
`MiniMax-M2.7-highspeed`, `synchronous completion emitted no assistant text
events`. The same job failed on `main` on 2026-08-26, 08-28, 08-29, 08-31,
09-04, and 09-15 with the same marker message, so this is a pre-existing
adapter defect, not a regression from the branch that surfaced it on PR #20.

The MiniMax adapter treats every M2.x `delta.content` value as a cumulative
snapshot and keeps only the latest one. MiniMax's published streaming example
shows incremental fragments that split mid-word, and its own SDK sample
recovers new text by slicing a buffer, which only distinguishes the two shapes
when a snapshot extends its predecessor. When an M2.x stream sends incremental
fragments, or ends with an empty `content` value beside `finish_reason`, the
adapter discards every earlier fragment and reports the last fragment or an
empty string. The chat live runner then fails the whole provider job on a
single content mismatch because it retries only transport-class errors.

This plan makes the adapter robust to both stream shapes without a live
credential, keeps the documented revision behaviour, and lets the live runner
absorb a rare non-deterministic probe without masking real failures.

## Design Decisions

- Infer the M2.x content shape per stream instead of assuming one: a value
  that starts with the accumulated text is a snapshot, and a strict extension
  of nonempty retained text establishes snapshot evidence. Other nonempty
  values append until that evidence exists and become replacing snapshots
  afterward. An empty `content` value never replaces accumulated text.
- Keep M3 incremental accumulation unchanged.
- Emit M2.x assistant text once from the validated terminal value, as today,
  so event parity is preserved whichever shape the provider used.
- Give the chat live runner one bounded retry, only when a completed response
  fails the probe-marker check, so a single non-deterministic reply cannot
  fail a provider job while identity, finish-reason, usage, and other event
  failures still fail immediately. A terminal-parity failure against the same
  marker-missing response may accompany the retry.

## Milestone 1: Adapter Robustness

At the end of this milestone the M2.x normalizer accepts incremental
fragments, extending snapshots, replacing snapshots, and empty terminal
content, and every shape reproduces the same terminal text and event parity.

- [x] Add failing `ai-models-minimax` regressions for an M2.7 stream of
      incremental mid-word fragments, a stream whose final `finish_reason`
      chunk carries empty `content`, a stream mixing an extending snapshot
      with a later replacing snapshot, and the existing replacement case,
      asserting terminal text, event parity, and buffered parity.
- [x] Implement shape inference in `stream_normalizer.rs` so a nonempty value
      that starts with the retained text replaces it, other nonempty values
      append until prefix-extension establishes snapshot evidence and replace
      afterward, and an empty value is ignored.
- [x] Keep the existing reasoning-details snapshot handling and M3 behaviour
      covered and unchanged.
- [x] Update `crates/ai-models-minimax/README.md`,
      `docs/protocol/minimax-model-provider.md`,
      `docs/protocol/model-completion-streaming.md`, and
      `docs/protocol/model-completion-events.md` to describe the inferred
      shape and the empty-terminal rule.

## Milestone 2: Live Runner Resilience

At the end of this milestone one non-deterministic probe reply no longer fails
a provider job, while every deterministic contract failure still does.

- [x] Add failing `xtask` runner tests proving a completed response that
      misses the probe marker is retried once, that a second miss fails, and
      that identity, finish-reason, usage, and non-parity event failures are
      not retried.
- [x] Implement the bounded marker retry in
      `xtask/tests/live_models/runner_tests.rs` without changing the
      transient retry wrapper.
- [x] Update `docs/protocol/live-model-api-tests.md` and `xtask/README.md`
      with the retry rule and its cost.

## Milestone 3: Verification, Commit, Push, And Review

- [x] Run `cargo fmt --all -- --check`, `cargo clippy --workspace
      --all-targets --all-features -- -D warnings`,
      `cargo test --workspace --all-features`,
      `cargo xtask rust-file-length-lint --all`, and `cargo xtask check`.
- [x] Review `git diff origin/main...` for scope, docs, tests, and untracked
      files.
- [x] Move this plan to Completed in `plans/README.md`.
- [ ] Run `git add -A`, commit with a Conventional Commit message, and push
      the current branch without renaming it.
- [ ] Run `cargo xtask review` after the push against `origin/main` and
      report findings without automatically fixing them.
- [ ] Confirm the `MiniMax catalog` job on the pull request after the push.
- [x] Treat a value equal to the retained text as a repeated fragment before
      snapshot evidence and a repeated snapshot afterward, with regressions
      for both.
