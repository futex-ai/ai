# Live Judgment API Test Protocol

## Purpose And Status

This protocol defines credentialed end-to-end verification for every
judgment adapter and every catalog entry advertising `ModelFeature::Judgment`.
It is planned and tracked by the
[Add TypeSafe judgment provider plan](../../plans/add-typesafe-judgment-provider.md).

The suite complements deterministic wire-mapping tests. It proves that the
production adapter, current model ids, authentication, upstream API, and
normalized response still work together.

## Scope

The initial provider set is TypeSafe because it is the only provider that
implements `JudgmentModel`. Each provider test obtains entries from its crate's
`known_models()` and selects every entry advertising `ModelFeature::Judgment`.
Adding a judgment model to a catalog therefore adds live coverage
automatically. A credential-free guard fails when a judgment-capable catalog
provider has no registered live adapter or CI matrix entry.

The chat live suite must treat `ProviderKind::TypeSafe` as judgment-only: its
provider registry maps that kind to no chat adapter, its all-provider guard
expects it to be absent from the chat registry, and its chat catalog filter
also excludes entries advertising `ModelFeature::Judgment`. The image and
video suites already select by their own features and only need the new
provider kind in their non-matching lists.

## Execution

`xtask/tests/live_judgments/mod.rs` owns the ignored provider integration
tests and their credential-free registry, workflow, runner, retry, and
validation tests. Every provider test:

1. Requires a non-empty `LIVE_JUDGMENT_API_KEY`; missing credentials fail
   rather than skip.
2. Loads every judgment-capable catalog entry, constructs the production
   adapter with `ReqwestJsonHttpClient`, explicit authentication, and the
   catalog's provider model id.
3. Erases the concrete adapter behind `DynJudgmentModel`.
4. Calls models sequentially and collects all failures for the provider before
   failing the job.
5. Retries only `RateLimited` and `TransientProvider`, with at most three total
   attempts using the shared `STANDARD_TRANSIENT_RETRY_DELAYS` schedule.
   Request, terminal provider, and internal failures are not retried.

The probe request evaluates one short, policy-safe support message as object
state with one question of each kind: a condition asking whether the message
expresses urgency, a choice between `billing`, `technical`, and `other`, and a
three-level frustration score. It is the shortest request that exercises every
answer mapping.

## Success Contract

Every successful live call must:

- report `provider: "typesafe"`, the catalog provider model id as `model_id`,
  and a non-empty `resolved_model_id`;
- return exactly the three requested answer ids with matching answer kinds;
- return a condition probability within zero to one;
- return a choice whose selected label is one of the requested options and
  whose probabilities cover exactly those options, sum to one within a
  0.01 tolerance, and carry a confidence within zero to one;
- return a score whose expected value lies within zero to two, whose
  probability indexes are exactly zero, one, and two summing to one within a
  0.01 tolerance, and whose confidence lies within zero to one; and
- report a non-zero input token count.

The probe verifies connectivity and normalized output, not answer quality.
Calibration and threshold tuning remain the consumer's responsibility.

## CI, Cost, And Credentials

`.github/workflows/live-judgments.yml` runs for same-repository pull requests
targeting `main`, on the default branch on a daily schedule, and by manual
dispatch. Forked and Dependabot pull requests skip the credentialed jobs. The
workflow must not use `pull_request_target`, uses read-only permissions, one
matrix entry per registered provider, and exposes only the current provider's
secret to its credential check and test steps.

| Provider | Secret |
| --- | --- |
| TypeSafe | `TYPESAFE_API_KEY` |

Each attempt sends one small request billed on input tokens only. A missing
secret on an eligible event is a failing configuration error.

## Local Invocation

```sh
cargo test --locked -p xtask --test live_judgments
LIVE_JUDGMENT_API_KEY="$TYPESAFE_API_KEY" cargo test --locked -p xtask \
  --test live_judgments catalog_tests::typesafe_judgment_catalog \
  -- --ignored --exact --nocapture
```

## Extending Coverage

Adding a judgment-capable catalog entry to TypeSafe requires no workflow
change. Adding another judgment provider requires implementing
`JudgmentModel`, advertising `ModelFeature::Judgment`, registering identity,
authentication, construction, test name, and secret in `LiveJudgmentProvider`,
adding an ignored catalog test, extending the workflow matrix, and updating
this protocol's credential table in the same change.

## Related Protocols

- [TypeSafe judgment provider](typesafe-judgment-provider.md) defines the
  shared boundary, wire mapping, and error contract.
- [Live model API tests](live-model-api-tests.md), [live image API
  tests](live-image-api-tests.md), and [live video API
  tests](live-video-api-tests.md) define the sibling credentialed suites.
