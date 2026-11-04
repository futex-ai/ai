# ai-models-core

`ai-models-core` contains reusable runtime wrappers and provider-agnostic helpers for `ai-interface` model implementations. Depend on it when you are building model providers or composing model execution policy without pulling in application-specific config or credential logic.

## Responsibilities

- Provide reusable wrappers around `Arc<dyn ai_interface::Model>`
- Preserve public completion-event delivery through retry, concurrency, and
  usage-pricing wrapper layers
- Keep retry, sleeping, and concurrency policy out of vendor crates
- Provide an injectable monotonic clock and async sleep boundary for provider
  operations that must poll within a total deadline
- Offer a boundary-neutral HTTP failure classifier and provider-agnostic
  JSON/error helpers shared by model crates
- Classify stream failures by provider-event progress
- Resolve every portable request control against its `ModelSpec` before
  transport, with one capability check that every chat adapter shares
- Provide provider-agnostic model usage pricing wrappers and integer cost
  calculation

## What This Crate Does

`ai-models-core` exposes wrappers such as `RetryingModel` and `ConcurrencyLimitedModel` so composition roots can assemble policy layers around provider clients. It also includes provider-facing helpers for common response/error handling, including the boundary-neutral `classify_http_status`, structured context-window overflow detection, tool-call JSON parsing, and deterministic local ids for provider tool calls that arrive without upstream ids. The classifier returns `None` below 400 and distinguishes rate-limited, transient, and terminal failures without depending on a modality-specific error type. HTTP 408, 409, 425, and 5xx responses are transient so retry wrappers can apply the configured schedule. Streaming helpers classify failures before any provider event as transient and failures after progress as `ModelError::Interrupted`.

The Chat Completions engine and its accumulator live in
[`ai-models-compat`](../ai-models-compat/README.md).

`resolve_controls(&request, &spec)` is the shared capability check. Every
chat adapter calls it before transport and sends exactly the returned
`ResolvedModelControls`: the optional effort, temperature, top-p, output
limit, stop sequences, tool choice, total timeout, and whether to use the
deferred lifecycle. The check does no I/O. Its rules are these:

- A request without an effort uses the default effort of the spec. The check
  lowers an effort that the spec does not list to the highest listed effort
  below it. It never raises an effort.
- When the request sets no effort and the spec has no default effort, the
  resolved effort is `None`. The adapter then sends no thinking field.
- Thinking counts as off only for the resolved `Effort::Disabled`. The
  sampling and tool-choice rules below count a `None` effort as thinking on.
- An output limit above the max output of the spec becomes that max output.
  When the spec requires an output limit, a request without one gets the max
  output.
- Temperature and top-p are left out while the `Sampling` rule of the spec
  fixes sampling.
- `RequiredOrAuto` becomes `Required` when the `ForcedToolChoice` rule of the
  spec allows it, and `Auto` otherwise. `ResolvedToolChoice` therefore has
  no fallback variant, and the check never removes a tool definition.
- The check fails with `ModelError::UnsupportedControl` for an effort with no
  listed effort at or below it, nonempty stop sequences that the spec does
  not support, a forced tool choice that the rule does not allow, and a
  required deferred lifecycle that the spec does not have.

Adapters keep their wire syntax. For example, DeepSeek sends the resolved
automatic tool choice by omitting `tool_choice` while thinking is on. Model
specs, efforts, routing tiers, and the combined catalog live in
`ai-interface`, and each provider crate owns its specs. This crate re-exports
`CostTier`, `IntelligenceScore`, `ModelFeature`, and `SpeedTier` from
`ai-interface`. `known_mock_models()` returns a copy of the mock catalog,
`ai_interface::MOCK_CATALOG`.

When callers request structured model responses, this crate also owns the
shared JSON parsing and JSON Schema validation helpers used by provider crates.
Synthetic tool-call scopes also hash provider replay context, including every
Anthropic block and Google part field, as well as DeepSeek and Kimi raw
assistant, reasoning, and tool-call fields, so retained
provider conversations remain distinct during deterministic id generation.

`UsagePricingModel` wraps any `ai_interface::Model` and applies a deployment
provided `ModelPricing` snapshot to normalized usage categories. It emits
`ModelUsageCostLine` values and sums known line costs in micro-USD; provider
crates keep parsing usage quantities but do not own mutable price tables.

The default retry schedule preserved by this crate is `100ms` then `250ms` for
transient model failures. `RetryingModel` does not replay interrupted
generations because doing so can duplicate provider work and billing. Its
event-observing path forwards only the successful attempt's events because a
retryable transient failure is classified before provider progress.

`ConcurrencyLimitedModel` holds its permit while events and the terminal
response are delivered. `UsagePricingModel` forwards events unchanged and
prices only the terminal response.

Long-running provider adapters can use `PollingRuntime` to couple monotonic
deadline measurement with async sleeping behind one testable trait. Production
code uses `TokioPollingRuntime`; deterministic provider tests enable the
`test-support` feature and inject `PollingRuntimeMock`.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{
    ConversationMessage, DynModel, MOCK_MODEL, MockModel, ModelRequest, ModelResult,
};
use ai_models_core::{
    ConcurrencyLimitedModel, ModelPricing, ResolvedModelControls, RetryingModel,
    UsagePricingModel, resolve_controls,
};

fn wrap_model() -> DynModel {
    let base: DynModel = Arc::new(MockModel::new("mock"));
    let retried: DynModel = Arc::new(RetryingModel::with_standard_transient_retry(base));
    let limited: DynModel = Arc::new(ConcurrencyLimitedModel::new(retried, "mock", 1));
    Arc::new(UsagePricingModel::new(limited, ModelPricing::free("mock")))
}

fn resolve_mock_controls() -> ModelResult<ResolvedModelControls> {
    let mut request = ModelRequest {
        system_prompt: "You are concise.".to_owned(),
        messages: vec![ConversationMessage::user("Say done")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.max_output_tokens = Some(1_000_000);

    // The mock spec lists no effort, has no default effort, and allows
    // 128,000 output tokens. The resolved effort is `None`, and the limit
    // becomes 128,000. A request that sets an effort fails with
    // `ModelError::UnsupportedControl`.
    resolve_controls(&request, &MOCK_MODEL)
}
```

## Development

```sh
cargo test -p ai-models-core
cargo clippy -p ai-models-core --all-targets --all-features -- -D warnings
```

### Key Code

- `src/capabilities.rs` - `resolve_controls`, `ResolvedModelControls`, and
  `ResolvedToolChoice`: the shared capability check before transport
- `src/catalog.rs` - `known_mock_models()`, which copies the mock catalog of
  `ai-interface`
- `src/retrying.rs` - transient retry wrapper and retry schedule
- `src/concurrency.rs` - per-model concurrency limiter wrapper
- `src/pricing.rs` - model usage pricing wrapper and integer cost calculator
- `src/http_status.rs` - boundary-neutral HTTP failure classification
- `src/errors.rs` - provider-agnostic JSON error mapping, parsing, and
  structured-output validation helpers
- `src/tool_call_identity/` - deterministic synthetic tool-call ids and
  hashing of normalized requests and provider replay state
- `src/sleeper.rs` - abstract sleeper boundary for retry testing
- `src/polling.rs` - monotonic clock and async sleeper boundary for
  deadline-aware provider polling

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../../docs/architecture.md`](../../docs/architecture.md)
- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
- [`../../docs/protocol/kimi-model-provider.md`](../../docs/protocol/kimi-model-provider.md)
- [`../../docs/protocol/deepseek-model-provider.md`](../../docs/protocol/deepseek-model-provider.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
