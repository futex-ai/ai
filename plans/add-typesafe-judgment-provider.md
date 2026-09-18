# Add TypeSafe Judgment Provider

## Summary

Add TypeSafe's System One models (Jev) as a new `ai-models-typesafe` crate
behind a new provider-agnostic `JudgmentModel` boundary in `ai-interface`.
TypeSafe evaluates state against typed questions and returns calibrated
probabilities instead of generated text, so it does not fit the chat `Model`
trait. The design follows the `ImageGenerator` and `VideoGenerator` modality
pattern: interface-owned DTOs and typed errors, a unimock-enabled trait and
dyn alias, a deterministic mock, a provider crate with injected HTTP and auth
seams, catalog metadata, and a catalog-driven live suite.

The normative behavior is defined by the
[TypeSafe judgment provider protocol](../docs/protocol/typesafe-judgment-provider.md)
and the
[live judgment API test protocol](../docs/protocol/live-judgment-api-tests.md).

## Design Decisions

- New trait, not a `Model` adapter: forcing questions into a chat request and
  probabilities into `assistant_message` or `structured_output` would hide the
  distribution and confidence semantics consumers need.
- Shared with existing code: `ProviderKind`, `ModelFeature`, `KnownModelSpec`,
  `ModelUsage`, `price_usage`, `json-http` client/auth mocks, the
  `ErrorContract` internal-error shape, the smoke test, and the live-suite
  layout. A new boundary-neutral `classify_http_status` helper in
  `ai-models-core` replaces the duplicated status tables in the OpenAI and
  Google image and video error modules.
- Neutral vocabulary at the boundary: `Condition`, `Choice`, and `Score`
  questions with `options` and `levels`, mapped to TypeSafe's `noul`,
  `choice`, and `score` wire types inside the provider crate.
- Question ids stay caller-owned in a `BTreeMap` so serialization and
  answers are deterministic.
- Local validation before transport; typed `JudgmentQuestionProblem` values
  instead of provider 422 round trips for obvious mistakes.
- Catalog entries `jev-latest` and `jev-1.13.0`; no `jev-preview`.
- Token counting, question fan-out, model listing, request-id capture, and
  confidence-threshold policy remain out of scope.

## Milestone 1: Protocol And Plan

Define the contract before code so no provider or consumer guesswork remains.

- [x] Verify the TypeSafe HTTP API, primitive, model, alias, limit, retry, and
      error documentation and the live unauthenticated error body.
- [x] Write the judgment provider protocol with boundary DTOs, validation,
      wire mapping, response validation, usage, and error contract.
- [x] Write the live judgment API test protocol.
- [x] Register this plan under Active in `plans/README.md`.
- [x] Link both protocols from the workspace README.

## Milestone 2: Shared Boundary In `ai-interface`

Consumers can compile against and deterministically mock the boundary without
the provider crate.

- [x] Add failing tests for `ProviderKind::TypeSafe` config parsing, display,
      and serde, then add the variant and update every exhaustive match: the
      `ai-interface` config/display helpers and the `from_kind` registries and
      non-matching provider lists in the `xtask` chat, image, and video live
      suites (the chat registry maps TypeSafe to `None`).
- [x] Add failing tests for `ModelFeature::Judgment` display and serde, then
      add the variant.
- [x] Add failing serde tests for `JudgmentContent` (untagged text, object,
      array), every `JudgmentQuestion` variant, `JudgmentConditionCriteria`,
      every `JudgmentAnswer` variant, `JudgmentRequest`, and
      `JudgmentResponse`.
- [x] Add failing tests for `From<&str>`, `From<String>`, and
      `TryFrom<serde_json::Value>` on `JudgmentContent`, including typed
      `UnsupportedContent` rejection of null, boolean, and number values, and
      for the `condition`, `choice`, `score`, and criteria constructors.
- [x] Add failing tests for `JudgmentError` display prefixes, constructors,
      tracked internal metadata, and every `JudgmentQuestionProblem`.
- [x] Implement the DTOs, typed error with `ErrorContract`, result alias,
      unimock-enabled `JudgmentModel`, and `DynJudgmentModel` in a new
      `src/judgment/` module family (`content.rs`, `question.rs`,
      `answer.rs`, `error.rs`, `model.rs`, `mod.rs`), each under 300 lines.
- [x] Add failing tests and implement the pure `JudgmentRequest::validate`
      method used by the mock and provider adapters.
- [x] Add failing tests and implement `MockJudgmentModel` in a top-level
      `src/mock_judgment_model.rs` with deterministic answers and shared
      validation.
- [x] Export the public API from `lib.rs` and update
      `crates/ai-interface/README.md`.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy -p ai-interface
      --all-targets --all-features`, and `cargo test -p ai-interface`.

## Milestone 3: Shared Status Classification In `ai-models-core`

Boundary-neutral HTTP status classification exists once and is reused.

- [x] Add failing tests for `classify_http_status` covering 429, 408, 409,
      425, 5xx including 529, 401, 403, 422, and `None` for 2xx inputs.
- [x] Implement `HttpFailureClass` and
      `classify_http_status(status) -> Option<HttpFailureClass>`, and make
      the existing `classify_json_http_error` delegate to it without changing
      `ModelError` behavior.
- [x] Replace the private transient-status helpers in the OpenAI and Google
      image and video error modules with the shared classifier, keeping their
      existing tests green.
- [x] Update `crates/ai-models-core/README.md`.
- [x] Run formatting, Clippy, and tests for `ai-models-core`,
      `ai-models-openai`, and `ai-models-google`.

## Milestone 4: `ai-models-typesafe` Provider Crate

Callers can evaluate questions through `TypeSafeJudgmentModel`.

- [x] Scaffold `crates/ai-models-typesafe` with a thin `lib.rs`, `catalog.rs`,
      and a `typesafe/` module family (`client.rs`, `request.rs`,
      `response.rs`, `error.rs`, `mod.rs`) with source-adjacent `_tests_`
      modules; add it to workspace members and dependencies.
- [x] Add failing catalog tests for `JEV_LATEST`, `JEV_1_13_0`, the provider,
      the judgment-only feature list, context window, tiers, and thinking
      level, then implement `known_models()`.
- [x] Add failing construction tests for API-key and auth-hook construction,
      endpoint and timeout overrides, and pass-through of a versioned provider
      model id that the catalog does not list.
- [x] Add failing request tests for exact JSON for every question variant,
      omitted optional criteria, null option and level descriptions, verbatim
      state, the `model` field, and no extra fields.
- [x] Add failing response tests for every answer mapping, resolved model id,
      ignored legend and unknown fields, score index parsing, malformed-body
      `Provider` errors, and `InvalidAnswer` propagation from the shared
      `validate_against` postconditions.
- [x] Add failing usage tests for present, missing, and saturating usage.
- [x] Add failing client tests with the mocked `json-http` transport for
      bearer auth, the exact endpoint, 429, 529, 5xx, 401, 403, 422, transport
      and auth-hook failures, malformed bodies, and credential absence in
      errors.
- [x] Implement the adapter behind `JudgmentModel` using injected
      `DynJsonHttpClient` and `DynJsonHttpAuth`, shared validation, shared
      status classification, and typed error translation with no
      `map_err`, `unwrap`, or string matching on error messages.
- [x] Add `crates/ai-models-typesafe/README.md` with the required sections and
      a compiling Quick Start.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy -p ai-models-typesafe
      --all-targets --all-features`, and `cargo test -p ai-models-typesafe`.

## Milestone 5: Smoke, Live Coverage, And Documentation

Catalog registration automatically produces credentialed coverage.

- [x] Add `ai-models-typesafe` to `xtask` and construct `JEV_LATEST` in the
      credential-free smoke test without a live request.
- [x] Add the `live_judgments` test target under `xtask/tests/live_judgments/`
      with `mod.rs`, `provider_tests.rs`, `runner_tests.rs`,
      `retry_tests.rs`, `validation_tests.rs`, `catalog_tests.rs`,
      `workflow_tests.rs`, and `layout_tests.rs` following the image suite.
- [x] Add credential-free guards proving every judgment-capable catalog
      provider is registered, adapters construct behind `DynJudgmentModel`,
      the probe request is the documented shape, and validation rejects each
      contract violation.
- [x] Update `xtask/tests/live_models.rs` so the all-provider guard treats
      TypeSafe as judgment-only, `chat_catalog` also excludes
      `ModelFeature::Judgment` entries, and a guard proves judgment entries
      never reach the chat, image, or video runners.
- [x] Add `.github/workflows/live-judgments.yml` mirroring the image workflow
      with the `TYPESAFE_API_KEY` secret.
- [x] Update the workspace `README.md` feature list, interface map, live-test
      instructions, key-code pointers, and CI section; update `xtask/README.md`.
- [x] Update `docs/protocol/live-model-api-tests.md`,
      `live-image-api-tests.md`, and `live-video-api-tests.md` to name the
      judgment suite as a sibling.
- [x] Change both new protocols from planned to implemented.

## Milestone 6: Verification, Commit, Push, And Review

- [x] Run `cargo fmt --all -- --check`, `cargo clippy --workspace
      --all-targets --all-features`, `cargo test --workspace --all-features`,
      `cargo xtask rust-file-length-lint --all`, and `cargo xtask smoke-test`.
- [x] Run `cargo xtask check` and fix failures until it passes.
- [x] Review `git diff origin/main...` for scope, docs, public API, tests,
      credentials, and untracked files.
- [x] Move this plan to Completed in `plans/README.md`.
- [ ] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters and a descriptive body, and push the current branch
      without renaming it.
- [ ] Run `cargo xtask review` after the push against `origin/main`.
- [ ] Do not auto-fix review findings; report each with a number, severity,
      context, impact, lettered options, and a recommended option.

## Milestone 7: Boundary Review Follow-Up

Address the design-review findings on the shared boundary before the provider
crate depends on it. This milestone was executed between Milestones 3 and 4.
At the end of this milestone, successful judgment responses carry checked
answer postconditions, score keys cannot collide, and the shared crate tests
its own public consumer paths.

- [x] Add failing tests for `JudgmentQuestionKind`, `JudgmentQuestion::kind`,
      and `JudgmentAnswer::kind`, then implement them.
- [x] Add failing tests for every `JudgmentAnswerProblem` display message and
      every `validate_against` failure, plus an accepting case and a proof
      that `MockJudgmentModel` output validates, then implement
      `JudgmentAnswerProblem`, `PROBABILITY_SUM_TOLERANCE`, the typed
      `InvalidAnswer` error variant with its constructor, and the pure
      `JudgmentResponse::validate_against` method.
- [x] Add failing text-level tests for duplicate, non-canonical, negative,
      and overflowing score level keys, then replace the score-probability
      deserializer with a map visitor that rejects duplicate indexes and
      reports failures through `serde::de::Error::invalid_value` instead of
      ad hoc formatting.
- [x] Add tests that drive `MockJudgmentModel` through `DynJudgmentModel`,
      configure the generated `JudgmentModelMock`, and round-trip questions
      with object and array instructions and populated structured criteria.
- [x] Move the judgment DTO tests beside their owning modules under
      `src/judgment/_tests_/` with explicit path declarations, keeping the
      mock-model tests beside the top-level mock source.
- [x] Align the protocol constructor wording with the real
      `JudgmentConditionCriteria::new` signature.
- [x] Update `crates/ai-interface/README.md` for the answer postconditions.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy -p ai-interface
      --all-targets --all-features -- -D warnings`, and
      `cargo test -p ai-interface --all-features`.

## Milestone 8: Provider Review Follow-Up

Address the design-review findings on the provider crate. At the end of this
milestone, duplicate score keys and unrequested answer fragments can no longer
corrupt or fail a judgment, non-2xx statuses are never treated as success,
credentials are redacted from every diagnostic, and the crate's tests pin the
remaining protocol rules.

- [x] Add a failing regression through `judge` proving a repeated score key
      whose surviving values satisfy every postcondition is rejected, then
      read successful responses through `send_bytes` and deserialize typed
      bodies directly from bytes.
- [x] Add a failing regression proving an unrequested answer id with an
      undecodable fragment is ignored, then hold answer entries as raw JSON
      fragments and decode only requested ids.
- [x] Add a failing regression proving a `3xx` status with a valid-looking
      body is a `Provider` error, then treat only `200..300` as success.
- [x] Add failing sentinel-secret regressions for a bearer key, a custom auth
      header, an auth-hook diagnostic, a transport diagnostic, and a provider
      body that echo the credential, asserting neither `Display` nor `Debug`
      contains it, then add the redaction step described by the protocol.
- [x] Add a failing test that malformed-body errors retain the decoder
      diagnostic after the fixed prefix, then implement it.
- [x] Add failing wire-mapping tests for structured instructions and
      descriptions, `no`-only and present-but-empty condition criteria, exact
      per-input local validation variants, and a JSON `null` error body
      retained as `null`, then fix the null fallback.
- [x] Update the crate README and the protocol wording where behavior
      changed.
- [x] Run `cargo fmt --all -- --check`, `cargo clippy -p ai-models-typesafe
      --all-targets --all-features -- -D warnings`, and
      `cargo test -p ai-models-typesafe --all-features`.
