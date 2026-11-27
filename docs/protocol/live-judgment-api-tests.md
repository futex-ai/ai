# Live Judgment API Test Protocol

Status: Implemented. The first credentialed OpenAI run is pending in
[Milestone 5](../../plans/shared-judgment-and-openai-decisions.md#milestone-5-registry-smoke-test-and-live-tests).

## Purpose And Status

This protocol defines credentialed end-to-end verification for every
judgment adapter and every catalog entry advertising `ModelFeature::Judgment`.
The
[Add TypeSafe judgment provider plan](../../plans/add-typesafe-judgment-provider.md)
tracks the TypeSafe coverage.

The suite complements deterministic wire-mapping tests. It proves that the
production adapter, current model ids, authentication, upstream API, and
normalized response still work together.

## Scope

The provider set is TypeSafe and OpenAI, the providers that implement
`JudgmentModel`. Each provider test obtains the specs of its crate's
`CATALOG` and selects every spec advertising `ModelFeature::Judgment`.
Adding a judgment model to a catalog therefore adds live coverage
automatically. A credential-free guard fails when a judgment-capable catalog
provider has no registered live adapter or CI matrix entry.

- The catalog guards check only the judgment specs of a registered provider.
- The OpenAI catalog also holds chat and image specs. They stay out of this
  suite.

The chat live suite must treat the `typesafe` provider id as judgment-only:
its provider registry maps that id to no chat adapter, its all-provider guard
expects it to be absent from the chat registry, and its chat catalog filter
also excludes specs advertising `ModelFeature::Judgment`. The image suite
already selects by its own feature and only needs the provider id in its
non-matching list.

The `openai` provider keeps its chat and image coverage. The chat catalog
filter skips `openai/gpt-6-luna`, because that spec advertises
`ModelFeature::Judgment`. The image suite does not select it, because it does
not advertise `ModelFeature::ImageGeneration`.

## Execution

`xtask/tests/live_judgments/mod.rs` owns the ignored provider integration
tests and their credential-free registry, workflow, runner, retry, and
validation tests. The provider tests are ignored, so `cargo xtask check`
never runs them. Every provider test:

1. Reads only `LIVE_JUDGMENT_<PROVIDER>_API_KEY`, where `<PROVIDER>` is the
   provider id in uppercase. A missing or blank value fails the test before
   its first request. The failure names the variable and omits its value.
2. Loads every judgment spec of the provider's catalog, and builds the
   production adapter through the registry with `ReqwestJsonHttpClient`,
   explicit authentication, and the catalog spec.
3. Erases the concrete adapter behind `DynJudgmentModel`.
4. Calls models sequentially and collects all failures for the provider before
   failing the job.
5. Retries only the rate limited and transient error kinds, with at most
   three total attempts. The two fixed retry delays are 100 milliseconds,
   then 250 milliseconds. Every other error kind stops the call without a retry.

The probe request evaluates one short, policy-safe support message as JSON
content with one question of each kind: a condition without criteria asking
whether the message expresses urgency, a choice between `billing`,
`technical`, and `other`, in that order, and a three-level frustration score
with the labels `Calm`, `Frustrated`, and `Very frustrated`. It is the
shortest request that exercises every answer mapping while satisfying
TypeSafe's required score-criteria contract.

## Success Contract

Every successful live call must:

- report the provider id of the registered provider, the catalog provider
  model id as `model_id`, and a non-empty `resolved_model_id`;
- return exactly the three requested answer ids and no others;
- answer each question with the kind of that question, so that a refusal
  fails the probe;
- satisfy the shared `JudgmentResponse::validate_against` postconditions
  defined by the [judgment response protocol](judgment-responses.md)
  (complete distributions over exactly the requested options and levels,
  finite probabilities and confidences within zero to one, a score
  expectation within zero to two, and distribution sums within the shared
  tolerance); and
- report a non-zero input token count.

The production adapter already applies the shared postconditions before
returning, so the runner re-validates only to keep the live contract explicit
and independent of adapter internals.

The probe verifies connectivity and normalized output, not answer quality.
Calibration and threshold tuning remain the consumer's responsibility.

## CI, Cost, And Credentials

`.github/workflows/live-judgments.yml` is eligible for same-repository pull
requests targeting `main`, on the default branch on a daily schedule, and by
manual dispatch. Its provider job runs only when the repository Actions
variable `LIVE_JUDGMENT_API_TESTS_ENABLED` is exactly `true`. Forked and
Dependabot pull requests skip the credentialed job. The workflow must not use
`pull_request_target`, uses read-only permissions, has one matrix entry per
registered provider, and exposes only the current provider's secret to its
credential check and test steps.

| Provider | Actions secret | Test variable |
| --- | --- | --- |
| TypeSafe | `TYPESAFE_API_KEY` | `LIVE_JUDGMENT_TYPESAFE_API_KEY` |
| OpenAI | `OPENAI_API_KEY` | `LIVE_JUDGMENT_OPENAI_API_KEY` |

The workflow builds the test variable from the selected secret name by
adding the `LIVE_JUDGMENT_` prefix. If a job selects a different provider's
test, that test fails because its variable is missing.

Each attempt sends one small request billed on input tokens only. A missing
or non-`true` opt-in variable skips the job. The opt-in should be enabled only
after the secret of every registered provider is provisioned; once enabled, a
missing secret on an eligible event is a failing configuration error.

## Local Invocation

```sh
cargo test --locked -p xtask --test live_judgments
LIVE_JUDGMENT_TYPESAFE_API_KEY="$TYPESAFE_API_KEY" cargo test --locked -p xtask \
  --test live_judgments catalog_tests::typesafe_judgment_catalog \
  -- --ignored --exact --nocapture
LIVE_JUDGMENT_OPENAI_API_KEY="$OPENAI_API_KEY" cargo test --locked -p xtask \
  --test live_judgments catalog_tests::openai_judgment_catalog \
  -- --ignored --exact --nocapture
```

## Extending Coverage

Adding a judgment-capable catalog entry to a registered provider requires no
workflow change. Adding another judgment provider requires implementing
`JudgmentModel`, advertising `ModelFeature::Judgment`, registering the
provider and its ignored catalog test, extending the workflow matrix, and
updating this protocol's credential table in the same change. The provider
id determines the test variable.

## Related Protocols

- [Judgment boundary](judgment-boundary.md) and its
  [response page](judgment-responses.md) define the shared boundary and the
  response check.
- [TypeSafe judgment provider](typesafe-judgment-provider.md) and
  [OpenAI judgment provider](openai-judgment-provider.md) define the wire
  mapping and the error contract of each adapter.
- [Live model API tests](live-model-api-tests.md) and [live image API
  tests](live-image-api-tests.md) define the sibling credentialed suites.
