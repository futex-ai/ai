# ai-models-multi

`ai-models-multi` is the ordered fallback `ai-interface::Model` implementation for the workspace. Depend on it when you want to try a sequence of already-configured models in order and fall through on model-call failures until one succeeds.

## Responsibilities

- Combine multiple `Arc<dyn ai_interface::Model>` instances behind one model boundary
- Preserve ordered fallback semantics across wrapped models
- Forward the events of each model in order, and emit a fallback event for
  each failed model before the next model
- Keep fallback policy separate from provider and transport code

## What This Crate Does

`MultiModel` accepts a `Vec<Arc<dyn Model>>` and opens their streams in order. It falls through to the next model for any `ModelError`, including provider rejections, rate limits, context-window failures, transient failures after retry exhaustion, interrupted streams, and internal adapter errors. This applies when a stream fails to open and when an open stream fails. If every model fails, it returns the error of the last model: from `stream` when no stream opened, and from the stream's `next` otherwise. A `MultiModel` with no models fails every call with `ModelError::Internal`.

`MultiModel` passes each request to every lane unchanged, including its
effort and other controls. A provider adapter resolves the controls against
its own model spec, so lanes can resolve the same request to different
efforts. The response of the lane that succeeds records its own resolved
effort and text id. A lane whose spec cannot honor a control fails with
`ModelError::UnsupportedControl` before transport, and `MultiModel` then
falls through to the next lane, as it does for any other error.

An interrupted stream means the first provider had already begun generating.
Falling through can therefore bill both that partial generation and the next
model attempt. Composition roots that cannot accept that cost should avoid a
fallback lane or enforce their own policy before constructing `MultiModel`.

The stream of `MultiModel` forwards the events of each lane unchanged and in
order. Before the next lane starts, it emits one `ModelEvent::Fallback` with
the error of the failed lane. It does this for every failed lane, also when
the lane failed before its stream opened or before it emitted an event. A
consumer discards every event since the start of the stream or since the
previous fallback event. The last lane's failure is an error, not a fallback
event. The folded response lists each failed lane in
`ModelResponse::attempts`. The fallback events of a nested `MultiModel` pass
through unchanged.

The text, reasoning, and tool-call end events of a lane keep their provider
state unchanged. At a fallback event, the shared fold of `ai-interface`
discards the parts and the provider state of the failed attempt. The folded
response therefore carries only the provider state of the lane that
succeeds.

This crate expects retry and concurrency policy to be applied by wrappers before the models are inserted into the vector.
Composition roots can use this crate as the concrete model returned by
model-router resolution when a route contains more than one buildable model.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{DynModel, MockModel};
use ai_models_multi::MultiModel;

fn build_model() -> MultiModel {
    MultiModel::new(vec![
        Arc::new(MockModel::with_provider("openai", "gpt-5.6-sol")) as DynModel,
        Arc::new(MockModel::with_provider("anthropic", "claude-sonnet-5")) as DynModel,
    ])
}
```

## Development

```sh
cargo test -p ai-models-multi
cargo clippy -p ai-models-multi --all-targets --all-features -- -D warnings
```

### Key Code

- `src/lib.rs` - crate root that exports `MultiModel`
- `src/model.rs` - `MultiModel` and the ordered opening of the first stream
  that opens
- `src/stream.rs` - the fallback stream: lane changes during reads and one
  fallback event for each failed lane
- `src/_tests_/multi_tests.rs` - fallback behavior coverage
- `src/_tests_/multi_event_tests.rs` - event forwarding and fallback event
  coverage
- `src/_tests_/stream_fallback_tests.rs` - failed attempt reporting, the last
  error, and drop behavior of the fallback stream

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/model-policies.md`](../../docs/protocol/model-policies.md)
