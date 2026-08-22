# ai-models-deepseek

`ai-models-deepseek` is the DeepSeek V4 implementation of
`ai_interface::Model`. Depend on it when a composition root needs current
DeepSeek Pro or Flash models through the official Chat Completions endpoint
with internally accumulated SSE and explicit credentials.

## Responsibilities

- Implement DeepSeek V4 behind the shared `ai_interface::Model` trait.
- Own the six Pro/Flash catalog variants and their routing metadata.
- Validate provider model ids and normalized thinking levels before requests.
- Map shared plain-text conversations and custom tools to typed DeepSeek
  request/response DTOs.
- Preserve private reasoning and raw tool calls for exact continuation replay
  without exposing reasoning in normalized assistant text or logger copies.
- Translate transport, authentication, HTTP, payload, and local validation
  failures into the shared model error contract.
- Accumulate streamed text, reasoning, tool calls, finish state, and exact
  usage without changing the buffered `ModelResponse` interface.
- Emit ordered assistant and reasoning text through the opt-in public
  completion-event boundary.

## What This Crate Does

`DeepSeekModel` accepts an injected `json-http` client plus an explicit API key
or auth hook. It streams requests from
`https://api.deepseek.com/chat/completions` with `include_usage`, records both
catalog and upstream model ids, and supports high, max, and disabled thinking
configurations for `deepseek-v4-pro` and `deepseek-v4-flash`. Standard and parallel custom
function calls retain provider ids, order, raw JSON arguments, assistant
content, and reasoning across tool continuations. Portable output limits and
stops map in all modes. Sampling and forced tool choice map only when thinking
is disabled; thinking keeps native sampling and automatic tool semantics.
`RequiredOrAuto` forces required use without thinking and retains tools while
omitting `tool_choice` in thinking mode. Blank system prompts are omitted and
per-call timeouts replace the overall stream deadline.

Structured requests use provider JSON-object mode plus a schema-specific
system instruction, then validate naturally stopped output against the caller's
JSON Schema locally. Cache hits, cache misses, visible completion tokens, and
reasoning tokens map to non-overlapping usage buckets; pricing remains a
composition-root concern. Resource-limited completions and retryable HTTP,
transport, and auth failures become shared transient errors.

Unsupported normalized levels downgrade to the highest configured level not
above the request: low and medium resolve to disabled, while extra-high
resolves to high. Responses record the effective level.

Completion streams default to a 3,600-second overall deadline and a 120-second
idle timeout. A failure before any event is retryable; a failure after progress
is `ModelError::Interrupted` and is not blindly replayed.
`complete_with_events` emits nonempty `delta.reasoning_content` and
`delta.content` fragments in order; schema-constrained calls remain silent.

The provider is text-only: any non-empty typed `content_parts` input is rejected
before authentication or transport. Retired aliases, vision, beta
APIs, Anthropic-format access, custom endpoints, and ambient credentials are
outside this crate's contract. Credentialed whole-catalog checks live in the
workspace `xtask` suite rather than this crate's deterministic unit tests.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_deepseek::{DEEPSEEK_V4_PRO, DeepSeekModel, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model(api_key: String) -> DeepSeekModel {
    DeepSeekModel::new(Arc::new(ReqwestJsonHttpClient::new()), api_key)
}

fn default_catalog_id() -> &'static str {
    assert!(
        known_models()
            .iter()
            .any(|model| model.id == DEEPSEEK_V4_PRO)
    );
    DEEPSEEK_V4_PRO
}
```

Callers retrieve credentials and apply policy/runtime wrappers at the
composition root. Use `DeepSeekModel::with_catalog_auth` for non-default
catalog variants.

## Development

```sh
cargo test -p ai-models-deepseek --all-features
cargo clippy -p ai-models-deepseek --all-targets --all-features -- -D warnings
```

All tests use injected transports and explicit test auth, so no DeepSeek
credential or network access is required.

### Key Code

- `src/catalog.rs` - current DeepSeek V4 ids and routing metadata.
- `src/deepseek/client.rs` - construction, validation, dispatch, and HTTP
  classification.
- `src/deepseek/request.rs` - shared request and continuation mapping.
- `src/deepseek/request_types.rs` - serialized Chat Completions request DTOs.
- `src/deepseek/response.rs` - typed response normalization.
- `src/deepseek/stream.rs` - public delta emission, SSE accumulation, and
  progress-aware failures.

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [DeepSeek provider protocol](../../docs/protocol/deepseek-model-provider.md)
- [Live model API test protocol](../../docs/protocol/live-model-api-tests.md)
- [DeepSeek implementation plan](../../plans/add-deepseek-model-provider.md)
