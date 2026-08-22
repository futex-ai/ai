# ai-models-minimax

`ai-models-minimax` is the MiniMax-specific `ai_interface::Model`
implementation for this workspace. Depend on it to call supported MiniMax
Chat Completions models with explicit credentials and shared runtime wrappers.

## Responsibilities

- Implement MiniMax text completion and modern function tools behind
  `ai_interface::Model`.
- Export typed metadata for current MiniMax agent models.
- Map shared messages to MiniMax's internally streamed Chat Completions API.
- Normalize finish reasons, usage, provider codes, HTTP failures, and
  transport failures into shared model contracts.
- Serialize M3 image and video inputs and validate requested structured output
  locally.
- Normalize MiniMax cumulative SSE snapshots and preserve complete reasoning
  details for continuation replay.
- Emit normalized assistant text and append-only reasoning content through the
  opt-in public completion-event boundary.

## What This Crate Does

`MiniMaxModel` accepts an injected `json-http` client and explicit API key or
auth hook. It sends requests to the international MiniMax endpoint, sets
`reasoning_split`, applies the selected thinking mode, and records both the
catalog id and upstream model id in normalized responses.
Requested thinking levels above a model's configured catalog ceiling
automatically downgrade to that ceiling, and responses record the effective
level. For M3, `Max`, `ExtraHigh`, and `High` therefore resolve to the catalog's
`Medium` adaptive mode.

Portable sampling and output limits map to the current OpenAI-compatible
fields. Every catalog model accepts `none` and `auto`. MiniMax-M3 also accepts
strict `Required`, as verified by Firna with a real tool; other catalog models
retain strict rejection. `RequiredOrAuto` maps to required for M3 and automatic
selection elsewhere while retaining tools. Named choices and stop sequences
return typed unsupported-control errors. Blank system prompts are omitted and
per-call timeouts reach the transport.

Completion requests send `stream: true` with final usage enabled. M3 emits
incremental visible-content deltas, while M2.x cumulative content is converted
into suffix deltas before shared accumulation. The last nonempty
`reasoning_details` snapshot is retained as the canonical replay state even
when MiniMax revises an earlier snapshot. A structurally complete stream may
end with `[DONE]` or clean EOF; EOF still fails when choices, a finish reason,
usage, or tool metadata are incomplete. Streams default to a 3,600-second
overall deadline and a 120-second idle timeout. Numeric `base_resp` failures
keep their provider classification before progress and become
`ModelError::Interrupted` after progress.

Modern `tools` and `tool_calls` retain MiniMax provider call ids across
assistant and tool-result messages. MiniMax `reasoning_content` and ordered
`reasoning_details` are stored as provider-owned replay context for subsequent
turns; private reasoning is never added to normalized assistant text. Tool
results always include `content`, including an empty string, while unavailable
empty assistant content is omitted. Legacy `function_call` messages are
neither sent nor accepted.

Cached input and reasoning tokens are separated from ordinary input/output
usage. HTTP-success `base_resp` failures are classified by MiniMax's numeric
provider codes, while HTTP failures use the shared status classifier.
Structured-output requests append raw-JSON and JSON Schema instructions to the
system prompt, then locally validate only naturally stopped responses; the
adapter does not claim native provider schema enforcement.

`complete_with_events` emits M3 incremental content and M2.x suffix-normalized
content. Append-only `reasoning_content` is emitted separately, while revisable
`reasoning_details` snapshots remain terminal replay context. Schema-constrained
calls remain silent.

Ordered shared text, image, and video parts are sent as Chat Completions
content parts, with base64 image and video bytes encoded as `data:` URLs in
`image_url` and `video_url` parts. The M3 catalog variants advertise vision
and video input; M2.7 variants advertise neither. Provider server tools,
regional endpoint selection, and legacy MiniMax models remain outside this
crate's boundary.

The crate exports `known_models()` and typed constants for `MiniMax-M3`,
`MiniMax-M3-thinking-disabled`, `MiniMax-M2.7`, and
`MiniMax-M2.7-highspeed`. It does not read configuration, inspect environment
variables, resolve secrets, or choose a region.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_minimax::{MINIMAX_M3, MiniMaxModel, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> MiniMaxModel {
    MiniMaxModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        MINIMAX_M3,
        "minimax-demo",
    )
}

fn known_model_count() -> usize {
    known_models().len()
}
```

## Development

```sh
cargo test -p ai-models-minimax --all-features
cargo clippy -p ai-models-minimax --all-targets --all-features -- -D warnings
```

### Key Code

- `src/catalog.rs` - supported model ids and routing metadata.
- `src/minimax/client.rs` - `Model` implementation, auth, and dispatch.
- `src/minimax/request.rs` - shared-to-MiniMax request mapping.
- `src/minimax/request_types.rs` - typed MiniMax request DTOs.
- `src/minimax/response.rs` - MiniMax response normalization.
- `src/minimax/stream.rs` and `stream_normalizer.rs` - public delta emission,
  SSE consumption, and cumulative snapshot normalization.

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
- [`../../docs/protocol/live-model-api-tests.md`](../../docs/protocol/live-model-api-tests.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [MiniMax provider protocol](../../docs/protocol/minimax-model-provider.md)
- [Implementation plans](../../plans/README.md)
