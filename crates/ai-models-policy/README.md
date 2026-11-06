# ai-models-policy

`ai-models-policy` holds the provider-neutral policy layers of the workspace:
retry, fallback, concurrency, and usage pricing. Each layer is an
`ai_interface::Model` that wraps any other `Model`. Depend on it at a
composition root that builds provider adapters and must retry them, chain
them, limit them, or price their usage. The crate depends only on
`ai-interface`. The provider adapters live in the provider crates, such as
[`ai-models-anthropic`](../ai-models-anthropic/README.md), and the shared
adapter helpers live in [`ai-models-core`](../ai-models-core/README.md).

## Responsibilities

- Retry rate-limited and transient failures before the first model event,
  with jitter, the provider's retry-after, and a bounded wait.
- Move to the next model only when an explicit fallback policy allows the
  error kind, and report every failed attempt.
- Limit the number of concurrent calls of one model.
- Price the usage event of one model from a deployment price snapshot.
- Keep the event order of the model stream, and pass every request on
  unchanged.
- Decide from error kinds, never from a provider id or the text of an error.

## What This Crate Does

`RetryingModel` retries the `RateLimited` and `Transient` kinds of
`ErrorKind`. It opens the inner stream again only before that stream returned
its first event, so a consumer never sees an event of a retried attempt. It
never retries the `Interrupted` kind. It also never repeats a composed model,
such as a fallback, whose error lists an earlier interrupted attempt or an
earlier attempt that emitted events.

`RetryingModel::with_standard_retry(inner)` uses `RetryConfig::default()`:
three attempts, the base delays of `STANDARD_TRANSIENT_RETRY_DELAYS`, which
are 100 ms and 250 ms, and a maximum wait of 30 s. Each wait is a random
duration from half of its base delay to all of it. When the error carries a
retry-after, the wait is the larger of the two. A retry-after above
`max_wait` returns the original error at once. Each base delay allows one
retry, so the retries stop when the base delays run out, also when
`max_attempts` allows more. `RetryingModel::new(inner, sleeper, random,
config)` takes a `DynSleeper`, a `DynRetryRandom`, and a `RetryConfig`.
Every attempt gets the complete request with its full total timeout, and a
wait does not count against it. A refused or exhausted retry returns the
original error with its core, its source, and its earlier attempts.

`FallbackModel::new(models, policy)` tries the models in order with the
policy that the caller gives. It has no default policy. `AnyError` moves on
after every kind. `AnotherModelCanSucceed` moves on when
`ErrorKind::can_succeed_with_another_model()` is true. A custom
`FallbackPolicy` implements `should_fallback(kind)` and sees only the kind.
Before the next model starts, the stream emits one `ModelEvent::Fallback`
with the error of the failed attempt and its `emitted_events` flag. A
consumer discards every event since the start of the stream or since the
previous fallback event. When the policy stops, or when no model remains,
the layer returns the last error, which lists every earlier failed attempt
in order. A successful response lists every failed attempt in
`ModelResponse::attempts`. Nested fallback layers report each failed attempt
once, also when two attempts failed in the same way. An empty model list
fails every call with the `Internal` kind, the provider `local`, and the
model `fallback`.

Both presets move on after an interrupted attempt. The provider can bill the
partial generation of that attempt, and the next model bills again. A caller
that cannot accept the second charge uses a custom policy that stops on the
`Interrupted` kind.

Each model of a fallback resolves the unchanged request against its own
spec, so the models can resolve different efforts. A model whose spec cannot
honour a control fails before transport with the `UnsupportedControl` kind,
and the policy decides whether the next model runs. The fold removes the
parts and the provider state of each failed attempt, so the response keeps
only the provider state of the model that succeeds.

`ConcurrencyLimitedModel::new(inner, model_id, max_concurrent)` allows at
most `max_concurrent` calls at the same time. A limit of zero sets no limit.
A call holds its permit from `stream` until its stream returns `None`,
returns an error, or is dropped. Waiting calls get their permits in arrival
order.

`UsagePricingModel::new(inner, pricing)` applies a deployment `ModelPricing`
snapshot to the usage event, and forwards every other event unchanged.
`price_usage` applies the same calculation to a `ModelUsage` value. Provider
crates report token quantities only and own no price tables.

Wrap each model with retry, concurrency, and pricing before the fallback
layer, so that each model keeps its own limits and prices. A `ModelRouter`
can return the fallback layer as the model of a route.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{DynModel, MockModel};
use ai_models_policy::{
    AnotherModelCanSucceed, ConcurrencyLimitedModel, FallbackModel, ModelPricing, RetryingModel,
    UsagePricingModel,
};

fn lane(model: DynModel, model_id: &str) -> DynModel {
    let retried: DynModel = Arc::new(RetryingModel::with_standard_retry(model));
    let limited: DynModel = Arc::new(ConcurrencyLimitedModel::new(retried, model_id, 4));
    Arc::new(UsagePricingModel::new(limited, ModelPricing::free("2026-11")))
}

fn build_model() -> FallbackModel {
    FallbackModel::new(
        vec![
            lane(Arc::new(MockModel::with_provider("openai", "gpt-5.6-sol")), "gpt-5.6-sol"),
            lane(
                Arc::new(MockModel::with_provider("anthropic", "claude-sonnet-5")),
                "claude-sonnet-5",
            ),
        ],
        Arc::new(AnotherModelCanSucceed),
    )
}
```

## Development

```sh
cargo test -p ai-models-policy
cargo clippy -p ai-models-policy --all-targets --all-features -- -D warnings
```

Downstream tests can enable the `test-support` feature for the `unimock`
mocks `SleeperMock`, `RetryRandomMock`, and `FallbackPolicyMock`. Production
code uses `TokioSleeper` and `SystemRetryRandom`.

### Key Code

- `src/retrying.rs` - `RetryingModel`, which retries before the first event
- `src/retry_config.rs` - `RetryConfig` and `STANDARD_TRANSIENT_RETRY_DELAYS`
- `src/retry_random.rs` - `RetryRandom` and `SystemRetryRandom` for jitter
- `src/sleeper.rs` - `Sleeper` and `TokioSleeper`
- `src/fallback/` - `FallbackModel`, `FallbackPolicy`, the two presets, and
  the fallback stream with its attempt history
- `src/concurrency.rs` - `ConcurrencyLimitedModel`
- `src/pricing.rs` - `UsagePricingModel`, `ModelPricing`, and `price_usage`

### Related Docs

- [`../../docs/protocol/model-policies.md`](../../docs/protocol/model-policies.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/architecture.md`](../../docs/architecture.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
