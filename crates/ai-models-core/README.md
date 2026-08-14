# ai-models-core

`ai-models-core` contains reusable runtime wrappers and provider-agnostic helpers for `ai-interface` model implementations. Depend on it when you are building model providers or composing model execution policy without pulling in application-specific config or credential logic.

## Responsibilities

- Provide reusable wrappers around `Arc<dyn ai_interface::Model>`
- Keep retry, sleeping, and concurrency policy out of vendor crates
- Provide an injectable monotonic clock and async sleep boundary for provider
  operations that must poll within a total deadline
- Offer provider-agnostic JSON/error helper functions shared by model crates
- Provide shared known-model catalog metadata used by composition roots
- Provide provider-agnostic model usage pricing wrappers and integer cost
  calculation

## What This Crate Does

`ai-models-core` exposes wrappers such as `RetryingModel` and `ConcurrencyLimitedModel` so composition roots can assemble policy layers around provider clients. It also includes provider-facing helpers for common response/error handling, including HTTP status classification, structured context-window overflow detection, tool-call JSON parsing, and deterministic local ids for provider tool calls that arrive without upstream ids. HTTP 408, 409, 425, and 5xx model responses are classified as transient provider failures so retry wrappers can apply the configured schedule.

It also defines `KnownModelSpec`, `KnownModelCatalog`, coarse `SpeedTier` and
`CostTier` values, `ThinkingLevel`, and the 1-to-10 `IntelligenceScore` used
by worker-side model routing. Provider crates own their concrete catalog
entries. A known model spec has a unique workspace catalog id plus a separate
provider model id, so a provider catalog can expose multiple deployable
variants for the same upstream model.

`ThinkingLevel` is ordered from `Disabled` through `Low`, `Medium`, `High`,
`ExtraHigh`, and `Max`. The level is normalized routing metadata; each
provider crate owns the mapping to provider-native fields and must only expose
catalog variants that the provider/model supports.
`resolve_catalog_thinking_level` keeps an exact requested level when a matching
variant exists and otherwise returns the highest variant for the same provider
model that does not exceed the request. Provider constructors use this helper
to downgrade unsupported levels without silently increasing reasoning cost.
If no catalog level is at or below the request, the helper returns `None` so
the adapter can preserve an explicit custom mapping or return a typed error.

When callers request structured model responses, this crate also owns the
shared JSON parsing and JSON Schema validation helpers used by provider crates.
Synthetic tool-call scopes also hash provider replay context, including every
DeepSeek and Kimi raw assistant, reasoning, and tool-call field, so retained
provider conversations remain distinct during deterministic id generation.

`UsagePricingModel` wraps any `ai_interface::Model` and applies a deployment
provided `ModelPricing` snapshot to normalized usage categories. It emits
`ModelUsageCostLine` values and sums known line costs in micro-USD; provider
crates keep parsing usage quantities but do not own mutable price tables.

The default retry schedule preserved by this crate is `100ms` then `250ms` for transient model failures.

Long-running provider adapters can use `PollingRuntime` to couple monotonic
deadline measurement with async sleeping behind one testable trait. Production
code uses `TokioPollingRuntime`; deterministic provider tests enable the
`test-support` feature and inject `PollingRuntimeMock`.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{DynModel, MockModel};
use ai_models_core::{
    ConcurrencyLimitedModel, KnownModelCatalog, ModelPricing, RetryingModel,
    ThinkingLevel, UsagePricingModel, known_mock_models,
};

fn wrap_model() -> DynModel {
    let base: DynModel = Arc::new(MockModel::new("mock"));
    let retried: DynModel = Arc::new(RetryingModel::with_standard_transient_retry(base));
    let limited: DynModel = Arc::new(ConcurrencyLimitedModel::new(retried, "mock", 1));
    Arc::new(UsagePricingModel::new(limited, ModelPricing::free("mock")))
}

fn mock_catalog() -> KnownModelCatalog {
    KnownModelCatalog::new().with_models(known_mock_models())
}

fn mock_thinking_level() -> ThinkingLevel {
    ThinkingLevel::Disabled
}
```

## Development

```sh
cargo test -p ai-models-core
cargo clippy -p ai-models-core --all-targets --all-features -- -D warnings
```

### Key Code

- `src/retrying.rs` - transient retry wrapper and retry schedule
- `src/concurrency.rs` - per-model concurrency limiter wrapper
- `src/catalog.rs` - known-model metadata, catalog lookup, routing tiers, and
  safe thinking-level downgrade resolution
- `src/pricing.rs` - model usage pricing wrapper and integer cost calculator
- `src/errors.rs` - provider-agnostic status, JSON parsing, and structured-output validation helpers
- `src/tool_call_identity.rs` - deterministic synthetic tool-call id helpers
- `src/sleeper.rs` - abstract sleeper boundary for retry testing
- `src/polling.rs` - monotonic clock and async sleeper boundary for
  deadline-aware provider polling

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../../docs/protocol/kimi-model-provider.md`](../../docs/protocol/kimi-model-provider.md)
- [`../../docs/protocol/deepseek-model-provider.md`](../../docs/protocol/deepseek-model-provider.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
- [`../../plans/README.md`](../../plans/README.md)
