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

## Decisions (2026-10-15)

Cal decided the open review items on 2026-10-15:

1. Live judgment workflow: skip a matrix entry whose Actions secret is
   absent instead of gating the whole job on
   `LIVE_JUDGMENT_API_TESTS_ENABLED`. The variable gate is removed.
   Evidence that both secrets exist: the daily `Live judgment APIs` run on
   `main` has passed its TypeSafe entry every day through 2026-10-15, and
   the chat, image, and video workflows use `OPENAI_API_KEY` daily.
2. Keep the shared redaction and judgment error helpers in `ai-models-core`
   (option A).
3. Replace the duplicated adapter call flow with one shared judgment
   transport in `ai-models-core`, in this PR (option B; review round 1,
   finding #1).
4. Move the shared judgment boundary and error contract out of the TypeSafe
   protocol into `docs/protocol/judgment-boundary.md`, in this PR (option C;
   review round 1, finding #2). Cal approved moving that text out of a doc
   on `origin/main`.
5. Keep the live-suite check that input tokens are non-zero (option A).
6. Thinking-level resolution across judgment-only catalog entries: pending.

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

Evidence: review report in `.context/add-openai-judgment-provider/review-round-1.md`.

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
  - Open (round 1, #1): Low, code structure, large effort. Both judgment adapters (`crates/ai-models-openai/src/openai/decisions/client.rs` and `crates/ai-models-typesafe/src/typesafe/client.rs`) repeat the same ~50-line call orchestration and each assembles its own `malformed provider payload` message; the reviewer recommends one shared `ai-models-core` request helper. Needs a user decision. Decided 2026-10-15: fix in this PR, Milestone 7.
  - Open (round 1, #2): Low, docs or spec, large effort. `docs/protocol/openai-judgment-provider.md` (289 lines) and `docs/protocol/typesafe-judgment-provider.md` (393 lines) exceed the ~250-line guidance; the reviewer recommends moving the shared boundary and error contract into a `judgment-boundary.md` page linked from both provider docs. Needs a user decision. Decided 2026-10-15: fix in this PR, Milestone 6.

## Milestone 6: Judgment Boundary Protocol Page

Define the shared contract once so both provider protocols stay under the
250-line guidance, and document the shared transport and the workflow skip
rule before the code changes in Milestones 7 and 8.

- [ ] Create `docs/protocol/judgment-boundary.md` and move into it, without
      changing the wording, the Shared Boundary section of
      `docs/protocol/typesafe-judgment-provider.md`, the shared parts of its
      Error Contract (the `JudgmentError` table, status classification,
      failure-body decoding, and the credential-redaction rules), and the
      `ai-interface` and `ai-models-core` ownership bullets.
- [ ] Add a Shared Transport section to the new page that defines the
      `ai-models-core` judgment transport contract of Milestone 7: the
      `JudgmentTransport` trait, `HttpJudgmentTransport`, the
      `JudgmentEndpoint` value, `JudgmentHttpResponse`, the
      `JudgmentFailureMessage` extractor, `malformed_judgment_payload`,
      the step order (validate, build, apply auth once, send, classify,
      parse, redact), and the rule that build failures precede auth-hook
      failures.
- [ ] Reduce `typesafe-judgment-provider.md` and
      `openai-judgment-provider.md` to their provider-specific sections,
      link the new page from both, and keep each at or under 250 lines
      without dropping any provider-specific rule.
- [ ] Update `docs/protocol/live-judgment-api-tests.md` for Milestone 8:
      no opt-in variable, each matrix entry detects its own secret and is
      skipped with a workflow warning when the secret is absent, and the
      success contract and credential table are unchanged.
- [ ] Update every link to the moved sections: the workspace `README.md`
      protocol list, the `ai-interface`, `ai-models-core`,
      `ai-models-openai`, and `ai-models-typesafe` READMEs, and
      `docs/protocol/live-judgment-api-tests.md`. Do not edit the completed
      `plans/add-typesafe-judgment-provider.md`.

## Milestone 7: Shared Judgment Transport In `ai-models-core`

Both adapters send through one tested transport, so a third provider adds
only its wire mapping and failure-message extractor.

- [ ] Add failing `ai-models-core` tests for `HttpJudgmentTransport` with
      the mocked `json-http` transport and auth hook: the auth hook is
      applied exactly once and its headers are transmitted unchanged; the
      explicit secret and every transmitted header value, plus the token
      after the last whitespace, are returned as the redaction set; a `2xx`
      body is returned as bytes; a `3xx` body is a terminal `Provider`
      error; 429, 408, 409, 425, 5xx, and 4xx statuses classify through the
      extractor with redaction; an empty failure body uses `HTTP <status>`;
      a transport failure is `TransientProvider` and redacted; a failing
      auth hook returns the fixed `authentication hook failed` diagnostic
      without a transport call; and `malformed_judgment_payload` builds the
      fixed prefix.
- [ ] Implement `src/judgment_transport.rs` in `ai-models-core`:
      `JudgmentFailureMessage = fn(&Value) -> String`, the `JudgmentEndpoint`
      value (`provider`, `url`, `timeout`, `failure_message`),
      `JudgmentHttpResponse { body, secrets }`, the unimock-enabled
      `JudgmentTransport` trait with `async fn post(&self, body: Value)`,
      `HttpJudgmentTransport::new(endpoint, model_id, http_client, auth)`
      with `with_explicit_secret`, `with_endpoint`, `with_timeout`, and
      `provider`, `model_id`, `endpoint`, `timeout` accessors, and
      `malformed_judgment_payload(provider, model_id, detail)`.
- [ ] Rewrite `OpenAiJudgmentModel` and `TypeSafeJudgmentModel` to hold one
      `HttpJudgmentTransport`, keep their public constructors and overrides
      unchanged, keep their own failure-message extractors, and reduce
      `judge` to validate, build, serialize, post, parse, redact. Keep every
      existing adapter test; adjust only the construction tests that read
      private fields.
- [ ] Update the `ai-models-core`, `ai-models-openai`, and
      `ai-models-typesafe` READMEs.
- [ ] Run formatting, Clippy, and tests for the three crates.

## Milestone 8: Live Judgment Workflow Skips Entries Without A Secret

The judgment workflow runs every provider whose secret exists and skips the
rest with a visible warning, so no repository variable is needed.

- [ ] Update `xtask/tests/live_judgments/workflow_tests.rs` first: the
      workflow must not reference `LIVE_JUDGMENT_API_TESTS_ENABLED`, must
      detect the credential in a step named `Detect provider credential`
      that runs before `Checkout`, must write a `present` step output and a
      `::warning::` annotation naming the missing secret, must not `exit 1`
      on a missing secret, and must guard every later step with
      `if: steps.credential.outputs.present == 'true'`.
- [ ] Update `.github/workflows/live-judgments.yml` to match, keeping the
      trusted-event conditions, read-only permissions, `fail-fast: false`,
      `max-parallel: 1`, the 20-minute timeout, and the per-entry secret
      exposure.
- [ ] Update the workspace `README.md` and `xtask/README.md` so neither
      mentions the opt-in variable and both describe the per-entry skip.
- [ ] Run the credential-free `live_judgments` suite.

## Milestone 9: Verification, Commit, Push, And Review (Round 2)

- [ ] Run `cargo fmt --all -- --check`, `cargo clippy --workspace
      --all-targets --all-features`, `cargo test --workspace --all-features`,
      `cargo xtask rust-file-length-lint --all`, `cargo xtask smoke-test`,
      and `cargo xtask check`.
- [ ] Run the deletion audit; the only removals are the text moved into
      `judgment-boundary.md` and the TypeSafe helper files moved in
      Milestone 2, both approved.
- [ ] Run `git add -A`, commit each milestone with a Conventional Commit
      title no longer than 50 characters and a descriptive body, and push
      the current branch.
- [ ] Review the complete local diff against `origin/main` with the
      `implementation-review` skill after the push. Report the findings,
      then apply the review-fix rule: fix the `Auto-fix: yes` findings,
      re-review once, and report the rest.

## Post-merge follow-up (non-blocking)

- [ ] Run the billable OpenAI judgment catalog test once with a real
      `OPENAI_API_KEY` and confirm the beta endpoint accepts the account.
