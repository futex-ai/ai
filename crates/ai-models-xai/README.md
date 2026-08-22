# ai-models-xai

`ai-models-xai` is the xAI-specific `ai-interface::Model` implementation for the workspace. Depend on it when you want to call xAI chat-completions models with explicit auth and shared runtime wrappers from neighboring crates.

## Responsibilities

- Implement the xAI model client behind `ai_interface::Model`
- Export strongly typed known xAI model metadata for model routing
- Map shared model/tool DTOs to the xAI chat-completions API
- Parse xAI responses into shared response DTOs and typed model errors
- Stream synchronous completions internally while retaining buffered deferred
  submission and polling
- Emit synchronous assistant and reasoning fragments through the opt-in public
  completion-event boundary while deferred calls remain silent

## What This Crate Does

`XaiModel` accepts a `json-http` client plus explicit auth input and handles:

- xAI chat-completions request serialization
- shared text and image parts as chat-completions content parts, with typed
  rejection of shared video parts before transport
- portable sampling, output limits, stops, strict or
  required-with-automatic-fallback tool choice, per-call deadlines, and
  blank-system omission
- provider-owned deferred submission and same-id polling that retries pending,
  rate-limited, transient transport, and server states without resubmitting an
  accepted completion
- synchronous SSE accumulation for visible content, modern tool calls, legacy
  function calls, final usage, and `[DONE]`; streams use a 3,600-second overall
  deadline and 120-second idle timeout unless the caller tightens the deadline
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
- xAI `reasoning_effort` mapping from catalog `ThinkingLevel` for Grok 4.5;
  fixed-reasoning Grok 4.20 requests omit the unsupported parameter
- catalog-aware downgrade to the highest configured reasoning effort not above
  an unsupported request, with the effective effort retained in responses
- provider response usage extraction into normalized input, cached input,
  output, and reasoning token counts when xAI returns compatible usage details
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`, including non-retryable interruption after
  stream progress

Synchronous `complete_with_events` calls expose nonempty primary-choice
`content` and `reasoning_content` fragments in order. Schema-constrained and
deferred submit-and-poll calls emit no events in version one.

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports `known_models()` and typed catalog id
constants for Grok 4.5. `GROK_4_5` is the first catalog entry and the default
used by workspace examples; its catalog metadata uses the provider's default
high reasoning effort. Low- and medium-thinking variants send provider model
id `grok-4.5` with the corresponding `reasoning_effort`. The catalog retains
the supported Grok 4.20 reasoning and general-purpose aliases with their
one-million-token context window; nonexistent Mini and configurable-thinking
aliases are not advertised.

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
- `src/xai/stream.rs` - synchronous SSE accumulation and failure classification

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
- [Grok 4.5 model details](https://docs.x.ai/developers/models/grok-4.5)
- [xAI model catalog](https://docs.x.ai/developers/models)
- [Grok 4.20 model details](https://docs.x.ai/developers/models/grok-4.20)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
