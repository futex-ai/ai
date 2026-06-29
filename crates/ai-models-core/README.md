# ai-models-core

`ai-models-core` contains reusable runtime wrappers and provider-agnostic helpers for `ai-interface` model implementations. Depend on it when you are building model providers or composing model execution policy without pulling in application-specific config or credential logic.

## Responsibilities

- Provide reusable wrappers around `Arc<dyn ai_interface::Model>`
- Keep retry, sleeping, and concurrency policy out of vendor crates
- Offer provider-agnostic JSON/error helper functions shared by model crates
- Provide shared known-model catalog metadata used by composition roots
- Provide provider-agnostic model usage pricing wrappers and integer cost
  calculation

## What This Crate Does

`ai-models-core` exposes wrappers such as `RetryingModel` and `ConcurrencyLimitedModel` so composition roots can assemble policy layers around provider clients. It also includes provider-facing helpers for common response/error handling, including HTTP status classification, structured context-window overflow detection, and tool-call JSON parsing. HTTP 408, 409, 425, and 5xx model responses are classified as transient provider failures so retry wrappers can apply the configured schedule.

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

When callers request structured model responses, this crate also owns the
shared JSON parsing and JSON Schema validation helpers used by provider crates.

`UsagePricingModel` wraps any `ai_interface::Model` and applies a deployment
provided `ModelPricing` snapshot to normalized usage categories. It emits
`ModelUsageCostLine` values and sums known line costs in micro-USD; provider
crates keep parsing usage quantities but do not own mutable price tables.

The default retry schedule preserved by this crate is `100ms` then `250ms` for transient model failures.

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
- `src/catalog.rs` - known-model metadata, catalog lookup, and routing tiers
- `src/pricing.rs` - model usage pricing wrapper and integer cost calculator
- `src/errors.rs` - provider-agnostic status, JSON parsing, and structured-output validation helpers
- `src/sleeper.rs` - abstract sleeper boundary for retry testing

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../../plans/README.md`](../../plans/README.md)
