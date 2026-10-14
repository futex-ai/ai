# Add OpenAI Decisions Judgment Provider

Status: Active. The plan closes when its PR merges.

## Summary

OpenAI's Decisions API (`POST /v1/decisions`, public beta since 2026-10-06)
evaluates text against named `predicate`, `choice`, and `score` questions and
returns probabilities, a selected choice, and a probability-weighted score.
That is the same shape as the provider-agnostic `JudgmentModel` boundary that
`ai-interface` already defines for TypeSafe Jev. This change adds
`OpenAiJudgmentModel` to `ai-models-openai` so composition roots can choose
OpenAI or TypeSafe behind the same `DynJudgmentModel`.

The normative behavior is defined by the
[OpenAI judgment provider protocol](../docs/protocol/openai-judgment-provider.md),
which builds on the shared boundary in the
[TypeSafe judgment provider protocol](../docs/protocol/typesafe-judgment-provider.md),
and by the
[live judgment API test protocol](../docs/protocol/live-judgment-api-tests.md).

## Design Decisions

- Reuse the shared boundary unchanged for requests. Object and array content
  renders as compact JSON text because the Decisions API accepts only text
  and image input; images stay out of scope.
- Fold optional condition criteria into the predicate instructions with a
  fixed `True when:` / `False when:` suffix instead of dropping them silently
  or rejecting the request.
- Send caller question ids as the OpenAI `name` field and match answers by
  name, so answer order does not matter.
- Map a provider `refusal` answer to a new typed `JudgmentError::Refused`
  variant that names the refused question, instead of a missing-answer
  postcondition failure. The whole call fails because the remaining answers
  describe the same state.
- Move the provider-neutral credential redaction, judgment error
  classification, and `InvalidAnswer` label redaction helpers from
  `ai-models-typesafe` into `ai-models-core` so both adapters share one
  implementation.
- Add one judgment-only catalog entry, `gpt-6-luna-decisions`, whose provider
  model id is `gpt-6-luna`, so a future chat entry for the same model keeps
  its own catalog id.
- Register OpenAI in the existing opt-in live judgment workflow matrix; the
  `LIVE_JUDGMENT_API_TESTS_ENABLED` gate stays unchanged.

## Milestone 1: Protocol And Plan

Define the OpenAI contract before code so no provider or consumer guesswork
remains.

- [x] Verify the OpenAI Decisions guide, SDK request and response types,
      model id, pricing, and input constraints.
- [x] Write `docs/protocol/openai-judgment-provider.md` with identity,
      catalog, request mapping, response mapping, refusal, usage, error, and
      redaction contracts.
- [x] Amend `docs/protocol/typesafe-judgment-provider.md` for the shared
      `Refused` error, provider-specific id handling, and the shared helper
      ownership in `ai-models-core`.
- [x] Amend `docs/protocol/live-judgment-api-tests.md` for the second
      provider, its secret, and the provider-specific success contract.
- [x] Register this plan under Active in `plans/README.md` and link the new
      protocol from the workspace README.

## Milestone 2: Shared Boundary And Shared Helpers

Consumers can branch on refusals, and both adapters share one redaction and
classification implementation.

- [x] Add failing tests for `JudgmentError::Refused` display, constructor,
      and retry classification, then add the variant and constructor.
- [x] Move `redact_secrets` and `secrets_from_headers` into
      `ai-models-core` with their unit tests.
- [x] Move judgment status, request-error, failure-body, and answer-label
      redaction helpers into `ai-models-core` with their unit tests, taking
      the provider name as a parameter.
- [x] Update `ai-models-typesafe` to use the shared helpers with no behavior
      change; keep every public-path regression in the TypeSafe crate.
- [x] Update the `ai-interface`, `ai-models-core`, and `ai-models-typesafe`
      READMEs.
- [x] Run formatting, Clippy, and tests for the three crates.

## Milestone 3: `OpenAiJudgmentModel`

Callers can evaluate questions through OpenAI behind `DynJudgmentModel`.

- [x] Add failing catalog tests for `GPT_6_LUNA`, `GPT_6_LUNA_DECISIONS`,
      the judgment-only feature list, context window, tiers, and thinking
      level, then implement the catalog entry.
- [x] Add failing construction tests for API-key and auth-hook construction,
      endpoint and timeout overrides, and unlisted model pass-through.
- [x] Add failing request tests for exact JSON for every question variant,
      criteria folding, compact JSON rendering of structured content,
      omitted descriptions, names, and no extra fields.
- [x] Add failing response tests for every answer mapping, ignored unnamed
      and unrequested answers, duplicate names and values, malformed bodies,
      refusals, and `InvalidAnswer` propagation.
- [x] Add failing usage tests for full, missing, and null usage, cached and
      reasoning buckets, and total fallback.
- [x] Add failing client tests for bearer auth, the exact endpoint, every
      status class, transport and auth-hook failures, and credential
      redaction.
- [x] Implement `src/openai/decisions/` behind `JudgmentModel` using the
      shared helpers, and export `OpenAiJudgmentModel`.
- [x] Update `crates/ai-models-openai/README.md`.
- [x] Run formatting, Clippy, and tests for `ai-models-openai`.

## Milestone 4: Smoke, Live Coverage, And Documentation

Catalog registration automatically produces credentialed coverage.

- [x] Construct `OpenAiJudgmentModel` in the credential-free smoke test.
- [x] Register OpenAI in `LiveJudgmentProvider`, add the ignored catalog
      test, make the validation provider check provider-specific, and
      generalize the judgment-only guards.
- [x] Add the OpenAI matrix entry to `.github/workflows/live-judgments.yml`.
- [x] Update the workspace `README.md` and `xtask/README.md`.

## Milestone 5: Verification, Commit, Push, And Review

Evidence: gate and test logs in `.context/add-openai-judgment-provider/`
(`xtask-check.log`, `fix-*.log`).

- [x] Run `cargo fmt --all -- --check`, `cargo clippy --workspace
      --all-targets --all-features`, `cargo test --workspace --all-features`,
      `cargo xtask rust-file-length-lint --all`, and `cargo xtask smoke-test`.
- [x] Run `cargo xtask check`.
- [x] Review `git diff origin/main...` for scope, docs, public API, tests,
      credentials, and untracked files.
- [x] Correct the judgment workflow gating text in `xtask/README.md`, the
      workspace `README.md`, and the live judgment protocol: the opt-in
      variable skips the whole job, and each matrix entry needs only its
      own secret (Codex review finding, P3).
- [x] Close the verification gaps: test that `Refused` is not retried and
      that a `null` usage object maps to the default, correct the object
      key order in the OpenAI protocol, and describe the adapter
      constructors, the README CI entry, and `http_status_message`.
- [x] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters and a descriptive body, and push the current branch.
- [ ] Review the complete local diff against `origin/main` with the
      `implementation-review` skill after the push. Report the findings,
      then apply the review-fix rule: fix the `Auto-fix: yes` findings,
      re-review once, and report the rest.

## Post-merge follow-up (non-blocking)

- [ ] Run the billable OpenAI judgment catalog test once with a real
      `OPENAI_API_KEY` and confirm the beta endpoint accepts the account.
