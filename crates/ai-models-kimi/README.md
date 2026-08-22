# ai-models-kimi

`ai-models-kimi` is the Moonshot AI Kimi implementation of
`ai_interface::Model`. Depend on it when a composition root needs Kimi K3
through internally accumulated Chat Completions SSE with explicit credentials
and the workspace's shared model contracts.

## Responsibilities

- Implement Kimi K3 behind the shared `ai_interface::Model` trait.
- Own Kimi catalog ids and routing metadata.
- Map shared text, image, structured-output, and custom-tool requests to the
  Moonshot Chat Completions API.
- Preserve Kimi assistant reasoning and raw tool calls for exact continuation
  replay without exposing reasoning as assistant text.
- Translate transport, HTTP, provider-payload, and validation failures into
  shared typed model errors.
- Accumulate streamed content, reasoning, tools, and final usage while keeping
  `Model::complete` buffered for callers.
- Emit ordered assistant and reasoning fragments through the opt-in public
  completion-event boundary.

## What This Crate Does

`KimiModel` accepts an injected `json-http` client plus an explicit API key or
auth hook. It sends requests to `https://api.moonshot.ai/v1/chat/completions`
and supports Kimi K3 low, high, and max reasoning effort, base64 image inputs,
parallel custom function calls, non-strict provider JSON-schema generation
with local validation, cached-input usage normalization, and lossless
assistant replay across tool continuations.

Synchronous completions send `stream: true` and request in-stream usage. Kimi's
final usage, including its direct cached-token field, maps through the existing
response normalizer. Streams default to a 3,600-second overall deadline and a
120-second idle timeout; explicit call timeouts replace the overall deadline.
Failures after stream progress become `ModelError::Interrupted`.
`complete_with_events` exposes nonempty primary-choice content and reasoning
fragments while schema-constrained calls remain silent.

Portable output limits use `max_completion_tokens`; ordered stops and all
shared tool choices map to Kimi fields, with `RequiredOrAuto` using native
required semantics. K3 keeps temperature and top-p at its provider-fixed
values, blank system prompts are omitted, and provider-neutral
`PreferDeferred` falls back to the ordinary synchronous request.

The crate does not read environment variables, load deployment config, price
usage, or make credential-dependent calls during unit tests. Credentialed
whole-catalog checks live in the workspace `xtask` suite. K2.x and Moonshot V1
models, Partial Mode, video/file upload, dynamic or official tools, and
provider cache-key tuning are outside its initial contract. Shared video
content parts are rejected with a typed provider error before transport.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_kimi::{KIMI_K3, KimiModel, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model(api_key: String) -> KimiModel {
    KimiModel::new(Arc::new(ReqwestJsonHttpClient::new()), api_key)
}

fn default_catalog_id() -> &'static str {
    assert!(known_models().iter().any(|model| model.id == KIMI_K3));
    KIMI_K3
}
```

Callers retrieve the API key and inject it at the composition root. Use
`KimiModel::with_catalog_auth` when constructing the high- or low-effort
catalog variants. Medium downgrades to low and extra-high downgrades to high;
disabled remains a typed configuration error because K3 has no supported level
at or below it. Unsupported provider model ids are rejected before a request
is sent.

## Development

```sh
cargo test -p ai-models-kimi
cargo clippy -p ai-models-kimi --all-targets --all-features -- -D warnings
```

All tests use injected transports and explicit test auth, so no Moonshot
credentials or network access are required.

### Key Code

- `src/catalog.rs` - known Kimi K3 ids and routing metadata.
- `src/kimi/client.rs` - construction, configuration validation, and request
  dispatch.
- `src/kimi/request.rs` - shared request and continuation mapping.
- `src/kimi/request_types.rs` - serialized Chat Completions request DTOs.
- `src/kimi/response.rs` - response, tool-call, structured-output, replay, and
  usage normalization.
- `src/kimi/stream.rs` - SSE accumulation and progress-aware failures.

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../../docs/protocol/kimi-model-provider.md`](../../docs/protocol/kimi-model-provider.md)
- [`../../docs/protocol/live-model-api-tests.md`](../../docs/protocol/live-model-api-tests.md)
- [`../../plans/add-kimi-model-provider.md`](../../plans/add-kimi-model-provider.md)
