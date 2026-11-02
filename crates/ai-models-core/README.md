# ai-models-core

`ai-models-core` contains reusable runtime wrappers and provider-agnostic helpers for `ai-interface` model implementations. Depend on it when you are building model providers or composing model execution policy without pulling in application-specific config or credential logic.

## Responsibilities

- Provide reusable wrappers around `Arc<dyn ai_interface::Model>`
- Keep the event order of the model stream through the retry, concurrency,
  and usage-pricing wrappers
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

`ai-models-core` exposes wrappers such as `RetryingModel` and
`ConcurrencyLimitedModel` for composition around provider clients.
`classify_http_status` returns the precise shared `ErrorKind` for a failed
HTTP status, and `None` below 400. `json_http_error_core` adds the HTTP 400
context-limit override and response metadata for every modality.
Stream helpers distinguish transient failures before provider progress from
interrupted failures after progress. The original HTTP source remains available.

Error and output helpers require an `ErrorContext`. Each call supplies its
adapter identity, request-id header names, and an injected `Clock`.
Production callers use `SystemClock`. Tests can use `ClockMock` through
`test-support`. Seconds and invalid retry headers do not read the clock.
`parse_retry_after` also accepts an explicit `SystemTime` for pure parsing.

Keep one response-free context for each logical call. Wrap the actual auth
hook with `capture_auth` to capture its transmitted credentials once.
Use `add_secret` for explicit keys outside that hook.
`for_response` creates a clone for one response and retains its headers.
The context evaluates absolute retry dates when it applies an error.
Each new HTTP phase starts from the response-free context.
Late stream, provider, and output failures use the clone for that stream.
`request_error` uses only the headers of the actual failed request.
`apply` and `apply_model_error` keep the kind and source while sanitizing
messages and request ids. `apply_judgment_error` also sanitizes question ids
and provider-controlled answer labels.
Auth-hook failures use a fixed diagnostic. They do not copy unknown hook text.

The Chat Completions engine and its accumulator live in
[`ai-models-compat`](../ai-models-compat/README.md).

`resolve_controls(&request, &spec)` is the shared capability check. Every
chat adapter calls it before transport and sends exactly the returned
`ResolvedModelControls`: the effort, temperature, top-p, output limit, stop
sequences, tool choice, total timeout, and whether to use the deferred
lifecycle. The check does no I/O. Its rules are these:

- A request without an effort uses the default effort of the spec. The check
  lowers an effort that the spec does not list to the highest listed effort
  below it. It never raises an effort.
- An output limit above the max output of the spec becomes that max output.
  When the spec requires an output limit, a request without one gets the max
  output.
- Temperature and top-p are left out while the `Sampling` rule of the spec
  fixes sampling.
- `RequiredOrAuto` becomes `Required` when the `ForcedToolChoice` rule of the
  spec allows it, and `Auto` otherwise. `ResolvedToolChoice` therefore has
  no fallback variant, and the check never removes a tool definition.
- The check fails with `ErrorKind::UnsupportedControl` for an effort with no
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
Synthetic tool-call scopes hash the portable projection of each message that
earlier releases hashed. The old layout is an optional text part before the
tool calls, a user message with at most one text part or with media parts,
or one tool result. A message in the old layout without provider state
therefore keeps its earlier ids. The hash also covers each provider state
with its part position and its opaque payload, and every part in order when
a message does not have the old layout. Retained provider conversations
therefore remain distinct during deterministic id generation.

Each wrapper is a `Model` whose stream forwards the events of the inner
stream in order. `UsagePricingModel` wraps any `ai_interface::Model` and
applies a deployment provided `ModelPricing` snapshot to the normalized usage
categories of the `Usage` event. It writes `ModelUsageCostLine` values and
sums known line costs in micro-USD; provider crates keep parsing usage
quantities but do not own mutable price tables. It forwards every other
event unchanged, so `complete` returns the priced usage.

The default retry schedule preserved by this crate is `100ms` then `250ms` for
transient model failures. `RetryingModel` retries only
`ErrorKind::Transient`. It opens the inner stream again when the
open fails with that error, or when the stream fails with it before the
stream returned its first event. After the first event it passes every
error on, so a consumer never sees an event of a retried attempt.
`RetryingModel` never retries `ErrorKind::Interrupted`, because a repeated
generation can duplicate provider work and billing.

`ConcurrencyLimitedModel` holds its permit from the call of `stream` until
its stream returns `None`, returns an error, or is dropped.

Long-running provider adapters can use `PollingRuntime` to couple monotonic
deadline measurement with async sleeping behind one testable trait. Production
code uses `TokioPollingRuntime`; deterministic provider tests enable the
`test-support` feature and inject `PollingRuntimeMock`.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{
    ConversationMessage, DynModel, Effort, MOCK_MODEL, MockModel, ModelRequest, ModelResult,
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
    request.controls.generation.effort = Some(Effort::High);
    request.controls.generation.max_output_tokens = Some(1_000_000);

    // The mock spec lists only `Effort::Disabled` and allows 128,000 output
    // tokens, so the effort lowers to disabled and the limit becomes 128,000.
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
- `src/retrying.rs` - transient retry wrapper on the stream and its retry
  schedule
- `src/concurrency.rs` - per-model concurrency limiter that holds a permit for
  the life of a stream
- `src/pricing.rs` - usage-event pricing wrapper and integer cost calculator
- `src/http_status.rs` - boundary-neutral HTTP failure classification
- `src/errors.rs` - provider-agnostic HTTP and provider-stream failure mapping
- `src/error_context.rs` and `src/capturing_auth.rs` - per-call error
  identity, safe diagnostics, response facts, and actual auth-hook capture
- `src/error_metadata.rs` and `src/clock.rs` - optional retry and request-id
  metadata with an injected wall clock
- `src/structured_output.rs` - caller-schema and generated-output checks
- `src/tool_call_identity/` - deterministic synthetic tool-call ids, and
  hashing of the portable message projection, ordered parts, and opaque
  provider state
- `src/sleeper.rs` - abstract sleeper boundary for retry testing
- `src/test_support.rs` - behind `test-support`: in-memory Chat Completions
  SSE fixtures, and `collect_model_events`, which records the events of a
  model stream as JSON and folds them, keeping the events before a failure
- `src/polling.rs` - monotonic clock and async sleeper boundary for
  deadline-aware provider polling

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../../docs/architecture.md`](../../docs/architecture.md)
- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/model-policies.md`](../../docs/protocol/model-policies.md)
- [`../../docs/protocol/kimi-model-provider.md`](../../docs/protocol/kimi-model-provider.md)
- [`../../docs/protocol/deepseek-model-provider.md`](../../docs/protocol/deepseek-model-provider.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
