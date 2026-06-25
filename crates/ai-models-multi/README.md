# ai-models-multi

`ai-models-multi` is the ordered fallback `ai-interface::Model` implementation for the workspace. Depend on it when you want to try a sequence of already-configured models in order and fall through on model-call failures until one succeeds.

## Responsibilities

- Combine multiple `Arc<dyn ai_interface::Model>` instances behind one model boundary
- Preserve ordered fallback semantics across wrapped models
- Keep fallback policy separate from provider and transport code

## What This Crate Does

`MultiModel` accepts a `Vec<Arc<dyn Model>>` and calls them in order. It falls through to the next model for any `ModelError`, including provider rejections, rate limits, context-window failures, transient failures after retry exhaustion, and internal adapter errors. If every model fails, it returns the last model error.

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
        Arc::new(MockModel::with_provider("openai", "gpt-5.5")) as DynModel,
        Arc::new(MockModel::with_provider("anthropic", "claude-sonnet-4-6")) as DynModel,
    ])
}
```

## Development

```sh
cargo test -p ai-models-multi
cargo clippy -p ai-models-multi --all-targets --all-features -- -D warnings
```

### Key Code

- `src/lib.rs` - ordered fallback model implementation
- `src/_tests_/multi_tests.rs` - fallback behavior coverage

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
