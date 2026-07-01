# Copy AI Crates From Juno

## Summary

Copy every Rust crate with an `ai-` prefix from
`/Users/calummoore/projects/futex/juno/crates` into this repository and make the
resulting workspace build, test, and document itself independently from Juno.

AI crates identified:

- `ai-interface`
- `ai-models-anthropic`
- `ai-models-core`
- `ai-models-google`
- `ai-models-multi`
- `ai-models-openai`
- `ai-models-xai`
- `ai-tool-calling`

Required support crate identified:

- `json-http`

The provider crates depend on the Juno workspace crate `json-http`, which does
not have an `ai-` prefix. Copy `json-http` into this repository as an explicit
support crate so the migrated provider crates build without depending on the
original Juno checkout.

## Milestone 1: Workspace Bootstrap

Create a standalone Rust workspace that can own the copied AI crates and run the
required local checks. At the end of this milestone, the repository should have a
valid Cargo workspace even before provider behavior is changed.

- [x] Add root `Cargo.toml` with workspace package metadata, resolver, members,
      and shared dependency declarations needed by the copied crates.
- [x] Add or adapt workspace tooling so `cargo xtask check`,
      `cargo xtask review`, and `cargo xtask rust-file-length-lint --all` are
      available in this repository.
- [x] Add root `.gitignore` entries for Rust build outputs and local tool
      artifacts.
- [x] Confirm the root `README.md` explains the workspace purpose, developer
      entry points, and links to `plans/README.md`.
- [x] Run `cargo metadata` to verify the workspace manifest parses.

## Milestone 2: Copy AI And Support Crates

Bring the AI crates across without changing behavior. At the end of this
milestone, all `ai-*` crate source plus the required `json-http` support crate
source, tests, and crate READMEs should be present under `crates/` and tracked
by Git.

- [x] Copy `crates/ai-interface` from Juno.
- [x] Copy `crates/ai-models-core` from Juno.
- [x] Copy `crates/ai-models-anthropic` from Juno.
- [x] Copy `crates/ai-models-google` from Juno.
- [x] Copy `crates/ai-models-openai` from Juno.
- [x] Copy `crates/ai-models-xai` from Juno.
- [x] Copy `crates/ai-models-multi` from Juno.
- [x] Copy `crates/ai-tool-calling` from Juno.
- [x] Copy `crates/json-http` from Juno as a required support crate for the
      provider crates.
- [x] Preserve source-adjacent `_tests_` directories, crate `README.md` files,
      snapshot files, and fixtures from each copied crate.
- [x] Add every copied crate to workspace members and workspace dependencies,
      including `json-http`.
- [x] Review each copied crate README against the repo's Rust crate README
      requirements and update any stale Juno-specific references, including the
      `json-http` support boundary.

## Milestone 3: Resolve Non-AI Support Dependencies

Make the copied crates independent from unpublished Juno-only workspace
assumptions. At the end of this milestone, dependency resolution should not rely
on the original Juno checkout.

- [x] Inventory all copied manifests for `workspace = true`, path, git, and
      unpublished dependencies.
- [x] Verify copied provider crates resolve `json-http` through this
      repository's workspace dependency instead of the Juno checkout.
- [x] Document `json-http` as a support crate owned by this workspace for AI
      provider HTTP client behavior.
- [x] Convert inherited workspace dependency versions into this repository's
      root `Cargo.toml` without pinning unnecessary external versions by hand.
- [x] Run `cargo metadata` again and fix dependency resolution errors.

## Milestone 4: Compile, Test, And Smoke Test

Validate the copied crates as a working product surface. At the end of this
milestone, all copied crates should pass formatting, linting, tests, and a
basic runtime-facing smoke check.

- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run a smoke check that exercises model-provider construction and
      tool-calling registration without requiring live provider API calls.
- [x] Run `cargo xtask check` and fix any failures until it passes.

## Milestone 5: GitHub CI Workflows

Add repository CI so the copied workspace is checked automatically on pull
requests and branch pushes. At the end of this milestone, GitHub Actions should
exercise the same core checks expected locally without requiring live provider
API keys.

- [x] Create `.github/workflows/ci.yml` for `pull_request` and `push` events
      against the main development branches.
- [x] Configure the CI workflow to install the pinned Rust toolchain or the
      repository's default Rust toolchain.
- [x] Configure CI to cache Cargo dependencies and build outputs where it is
      safe and useful.
- [x] Run `cargo fmt --all -- --check` in CI.
- [x] Run `cargo clippy --workspace --all-targets --all-features` in CI.
- [x] Run `cargo test --workspace --all-features` in CI.
- [x] Run `cargo xtask rust-file-length-lint --all` in CI.
- [x] Run `cargo xtask check` in CI after the faster targeted checks.
- [x] Ensure CI and smoke tests do not require live Anthropic, Google, OpenAI,
      or xAI credentials.
- [x] Document the CI workflow in the root `README.md`.

## Milestone 6: Documentation, PR, And Review

Finish the migration with documentation, a committed diff, and reviewer
feedback. Do not auto-fix review findings without explicit user direction.

- [x] Update the root `README.md` with the final workspace summary, developer
      setup, crate map, public interfaces, and key code jumping-in points.
- [x] Update `plans/README.md` to move this plan from active to completed after
      all previous milestones are done.
- [x] Review `git diff origin/main...` for accidental unrelated changes,
      generated artifacts, missing files, and stale Juno paths.
- [x] Run `git add -A`.
- [x] Commit the completed work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Create a GitHub pull request against `main` after pushing the branch.
- [x] Confirm the GitHub CI workflows start for the pull request and report
      their status in the final handoff.
- [x] Run `cargo xtask review` after pushing so the AI reviewer checks the diff
      against `origin/main`.
- [x] Report each review finding in the final message with severity, context,
      impact, solution options, and a recommended option.

## Milestone 7: Review Follow-ups

Address the reviewer findings selected by the user after the initial PR review.
At the end of this milestone, Gemini prompt-level safety blocks should be
terminal filtered responses, and xAI structured-output requests should accept
the shared non-strict schema contract.

- [x] Add regression tests for Gemini candidate-less prompt blocks and xAI
      non-strict structured-output requests.
- [x] Map Gemini `promptFeedback.blockReason` responses with no candidates to
      `FinishReason::Filtered`.
- [x] Set xAI structured-output `json_schema.strict` to `false`.
- [x] Update crate READMEs with the clarified provider behavior.
- [x] Run targeted regression tests for the changed Google and xAI adapters.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing and report any findings.

## Milestone 8: Tool-Call Safety Review Follow-ups

Address the reviewer findings selected by the user after the second PR review.
At the end of this milestone, OpenAI incomplete function-call responses should
not be dispatchable, and Gemini no-argument function calls should decode as the
empty object expected by typed tool schemas.

- [x] Add regression tests for incomplete OpenAI function-call responses and
      Gemini no-argument function calls.
- [x] Preserve OpenAI incomplete, failed, cancelled, and unknown response
      statuses before accepting tool-call output items.
- [x] Suppress OpenAI tool-call payload parsing when the finish reason is not
      `ToolCalls`.
- [x] Default omitted Gemini `functionCall.args` values to `{}`.
- [x] Update crate READMEs with the clarified provider behavior.
- [x] Split OpenAI response parser and response tests to keep changed Rust files
      under the 300-line file-length limit.
- [x] Run targeted regression tests for the changed OpenAI and Google adapters.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing and report any findings.

## Milestone 9: Provider Retry and Truncation Follow-ups

Address the reviewer findings selected by the user after the third PR review.
At the end of this milestone, xAI truncated tool-call responses should preserve
their terminal finish reason without parsing partial arguments, and HTTP 409
provider responses should be retryable by the shared retry wrapper.

- [x] Add regression tests for truncated xAI tool-call responses and HTTP 409
      error classification.
- [x] Preserve xAI terminal finish reasons before accepting tool-call payloads.
- [x] Classify HTTP 409 model responses as transient provider failures.
- [x] Update crate READMEs with the clarified provider behavior.
- [x] Run targeted regression tests for the changed xAI and core helpers.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing and report any findings.

## Milestone 10: Terminal Tool-Call Suppression Follow-ups

Address the reviewer findings selected by the user after the fourth PR review.
At the end of this milestone, Gemini and Anthropic responses should only expose
tool calls when the normalized finish reason is `ToolCalls`; terminal,
filtered, truncated, missing, or unknown finishes should preserve their reason
without exposing parsed calls.

- [x] Add regression tests for Gemini and Anthropic terminal responses that
      include provider tool-call payloads.
- [x] Suppress Gemini tool calls unless the normalized finish reason is
      `ToolCalls`.
- [x] Suppress Anthropic tool calls unless the normalized finish reason is
      `ToolCalls`.
- [x] Update crate READMEs with the clarified provider behavior.
- [x] Run targeted regression tests for the changed Google and Anthropic
      adapters.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing and report any findings.

## Milestone 11: OpenAI Replay And xAI Tool Continuation Follow-ups

Address the reviewer findings selected by the user after the fifth PR review.
At the end of this milestone, xAI tool-result continuation messages should use
only provider-supported tool message fields, and OpenAI Responses replay should
preserve raw function-call output items needed for stateless tool-calling
continuations.

- [x] Add regression tests for xAI tool-role message names and OpenAI raw
      function-call replay context.
- [x] Suppress xAI `name` serialization on tool-role continuation messages.
- [x] Preserve OpenAI Responses function-call output items in provider replay
      context.
- [x] Replay preserved OpenAI function-call items without duplicating
      normalized tool calls.
- [x] Update crate READMEs with the clarified provider replay behavior.
- [x] Run targeted regression tests for the changed xAI, OpenAI, and interface
      crates.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing and report any findings.

## Milestone 12: OpenAI Item Identity And Transcription Retry Follow-ups

Address the reviewer findings selected by the user after the sixth PR review.
At the end of this milestone, OpenAI Responses stateless replay should retain
the provider function-call item id, and OpenAI audio transcription HTTP
timeouts/conflicts should be surfaced as transient failures.

- [x] Add regression tests for OpenAI function-call item ids and transcription
      retryable status classification.
- [x] Preserve OpenAI Responses function-call provider item ids in provider
      replay context.
- [x] Serialize preserved OpenAI function-call item ids during request replay.
- [x] Classify OpenAI transcription `408`, `409`, and `425` statuses as
      transient provider failures.
- [x] Update crate READMEs with the clarified provider replay and transcription
      retry behavior.
- [x] Run targeted regression tests for the changed OpenAI and interface
      crates.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing and report any findings.

## Milestone 13: OpenAI Phase And Quick Start Follow-ups

Address the reviewer findings selected by the user after the seventh PR review.
At the end of this milestone, OpenAI Responses stateless replay should preserve
assistant message phase metadata for tool preambles, and the `ai-tool-calling`
Quick Start example should compile against the current public API.

- [x] Add a regression test for OpenAI assistant message phase metadata in
      stateless replay context.
- [x] Preserve OpenAI Responses assistant message `phase` metadata in provider
      replay context.
- [x] Serialize preserved assistant message `phase` metadata during OpenAI
      request replay in the original provider-context order.
- [x] Update the `ai-tool-calling` Quick Start example to use the current
      conversation helper and import the turn extension trait.
- [x] Update crate READMEs with the clarified provider replay behavior.
- [x] Run targeted regression tests for the changed OpenAI, interface, and
      tool-calling crates.
- [x] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [x] Run `cargo xtask rust-file-length-lint --all`.
- [x] Run `cargo clippy --workspace --all-targets --all-features`.
- [x] Run `cargo test --workspace --all-features`.
- [x] Run `cargo xtask check`.
- [x] Review `git diff origin/main...` for accidental unrelated changes,
      generated artifacts, missing files, and stale Juno paths.
- [x] Run `git add -A`.
- [x] Commit the follow-up work with a Conventional Commit message.
- [x] Push the current branch.
- [x] Confirm the GitHub CI workflow status for the pull request.
- [x] Run `cargo xtask review` after pushing.
- [x] Add regression tests for truncated OpenAI function-call replay context
      and phase-less assistant message replay ordering.
- [x] Drop OpenAI function-call replay context when the normalized finish
      reason is not `ToolCalls`.
- [x] Preserve phase-less OpenAI assistant message markers when tool-call
      replay context needs ordering.
- [x] Run targeted regression tests for the second OpenAI replay follow-up.
- [x] Re-run the full local verification gates after the second follow-up.
- [ ] Commit and push the second follow-up work with a Conventional Commit
      message.
- [ ] Confirm the GitHub CI workflow status for the second follow-up.
- [ ] Run `cargo xtask review` again after the second follow-up push.
- [ ] Repeat recommended review findings until the reviewer returns no
      recommended fixes or only findings the user explicitly defers.
