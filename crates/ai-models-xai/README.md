# ai-models-xai

`ai-models-xai` is the xAI-specific `ai-interface::Model` implementation for the workspace. Depend on it when you want to call xAI chat-completions models with explicit auth and shared runtime wrappers from neighboring crates.

## Responsibilities

- Implement the xAI model client behind `ai_interface::Model`
- Export strongly typed known xAI model metadata for model routing
- Map shared model/tool DTOs to the xAI chat-completions API
- Parse xAI responses into shared response DTOs and typed model errors

## What This Crate Does

`XaiModel` accepts a `json-http` client plus explicit auth input and handles:

- xAI chat-completions request serialization
- xAI tool-call parsing
- xAI `finish_reason` normalization into `ai_interface::FinishReason`
- xAI OpenAI-compatible `response_format` JSON-schema mapping for structured outputs
- xAI `reasoning_effort` mapping from catalog `ThinkingLevel` for
  reasoning-capable catalog variants
- provider response usage extraction into normalized input and output token
  counts, with unsupported cached/reasoning categories left at zero
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`

This crate does not load config, read environment variables, or resolve credentials on its own.
It exports `known_models()` and typed catalog id constants
(`GROK_4_20_REASONING`, `GROK_4_20`, `GROK_4_20_THINKING_HIGH`,
`GROK_4_20_MINI`) so composition roots can validate deployment config and sort
routes without duplicating xAI model metadata. `GROK_4_20_THINKING_HIGH` sends
provider model id `grok-4.20` with `reasoning_effort: "high"`.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_xai::{GROK_4_20_REASONING, XaiModel, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> XaiModel {
    XaiModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GROK_4_20_REASONING,
        "xai-demo",
    )
}

fn known_model_count() -> usize {
    known_models().len()
}
```

## Development

```sh
cargo test -p ai-models-xai
cargo clippy -p ai-models-xai --all-targets --all-features -- -D warnings
```

### Key Code

- `src/xai/mod.rs` - `Model` implementation and request dispatch
- `src/catalog.rs` - known xAI model ids and routing metadata
- `src/xai/request.rs` - xAI request DTO mapping
- `src/xai/response.rs` - xAI response parsing

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
