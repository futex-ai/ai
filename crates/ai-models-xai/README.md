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
- portable sampling, output limits, stops, tool choice, per-call deadlines,
  and blank-system omission
- provider-owned deferred submission and same-id polling that retries pending,
  rate-limited, transient transport, and server states without resubmitting an
  accepted completion
- xAI modern `tool_calls` and legacy `function_call` parsing, including
  deterministic request-scoped local ids for legacy calls and legacy
  continuation replay without sending synthetic `tool_call_id` fields
- xAI `finish_reason` normalization into `ai_interface::FinishReason`
- xAI tool-result continuation request serialization using `tool_call_id` and
  content without an unsupported tool-message `name` field
- isolation from replay context owned by other providers, including DeepSeek,
  Kimi, and MiniMax assistant reasoning state and raw tool calls
- terminal xAI finish reasons such as `length` are preserved before tool-call
  payloads are parsed, so partial tool-call arguments are not dispatched
- xAI OpenAI-compatible non-strict `response_format` JSON-schema mapping for
  structured outputs, with local response validation through shared helpers
- xAI `reasoning_effort` mapping from catalog `ThinkingLevel` for
  reasoning-capable catalog variants
- provider response usage extraction into normalized input, cached input,
  output, and reasoning token counts when xAI returns compatible usage details
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports `known_models()` and typed catalog id
constants for Grok 4.5. `GROK_4_5` is the first catalog entry and the default
used by workspace examples; its catalog metadata uses the provider's default
high reasoning effort. Low- and medium-thinking variants send provider model
id `grok-4.5` with the corresponding `reasoning_effort`. All existing Grok
4.20 entries remain available for pinned deployments.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_xai::{GROK_4_5, XaiModel, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> XaiModel {
    XaiModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GROK_4_5,
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

- `src/xai/client.rs` - `Model` implementation and request dispatch
- `src/xai/deferred.rs` - deferred submission, polling, and deadline handling
- `src/catalog.rs` - known xAI model ids and routing metadata
- `src/xai/request.rs` - xAI request DTO mapping
- `src/xai/request_types.rs` - xAI request serialization DTOs
- `src/xai/response.rs` - xAI response parsing

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [Grok 4.5 model details](https://docs.x.ai/developers/models/grok-4.5)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
