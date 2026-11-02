# ai-models-compat

`ai-models-compat` implements Chat Completions behind `ai_interface::Model`.
Depend on it when a composition root needs a compatible provider with explicit
credentials and an injected `json-http` client.

## Responsibilities

- Map conversations, tools, call controls, and structured output to the wire.
- Map SSE chunks to model events: text, reasoning, tool calls, usage, and
  finish.
- Preserve each dialect's replay data and errors.
- Export each dialect's provider id, model specs, catalog, custom defaults,
  and model constructors.

## What This Crate Does

One engine serves five provider dialects. The public modules are `deepseek`,
`kimi`, `qwen`, `minimax`, and `xai`. Each module exports its model, its
`PROVIDER_ID`, one `ModelSpec` constant for each catalog model, `CATALOG`,
`CUSTOM_DEFAULTS`, and `known_models()`, which returns a copy of `CATALOG`.

`MiniMaxModel` and `XaiModel` take a spec in `new`, `with_auth`, and
`with_catalog_auth`. `DeepSeekModel`, `KimiModel`, and `QwenModel` build the
default spec of their provider in `new` and `with_auth`: DeepSeek V4 Pro,
Kimi K3, and Qwen3.7 Plus. Their `with_catalog_auth` takes a catalog or
custom spec. A model that a catalog does not list uses
`ModelSpec::custom(PROVIDER_ID, model_id, &CUSTOM_DEFAULTS)` of its dialect.

Each request carries its effort. The engine runs the shared capability check
of `ai-models-core` against the spec before transport, so efforts, output
limits, sampling, stops, tool choice, and the deferred lifecycle follow the
spec data. The dialect then maps the resolved controls to its wire fields.
For example, DeepSeek omits `tool_choice` for the resolved automatic choice
while thinking is on. Responses record the resolved effort and the text id
of the spec.

The engine implements `Model::stream`. Only choice index 0 produces events.
`delta.content` gives text deltas, and the reasoning field of the dialect
gives reasoning deltas. When one chunk has both, the reasoning delta comes
first. Indexed `delta.tool_calls` entries give tool-call start, argument,
and end events. One usage event and one finish event follow the terminal
event of the stream. `Model::complete` folds these events. A call with a
response schema emits text deltas like any other call. Stream failures
after progress return `ModelError::Interrupted`.
Moved errors retain their existing message prefixes for response parity.

MiniMax retains M2.x snapshot recovery, M3 deltas, reasoning details, numeric
provider errors, and structurally validated clean EOF. It emits the inferred
M2.x text as one text delta after terminal validation.
xAI retains legacy function calls, deterministic call ids, final usage
replacement, and deferred submission and polling. A deferred call emits
every event of the call after the provider result arrives. Its existing
timeout classification remains unchanged.

A dialect supplies its endpoint, usage fields, replay rule, and wire settings.
Model wrappers hold the spec and delegate execution to the engine. Callers
supply credentials and apply retry, concurrency, and pricing wrappers at the
composition root.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::ModelSpec;
use ai_models_compat::{
    deepseek::{self, DeepSeekModel},
    minimax::{MINIMAX_M3, MiniMaxModel},
};
use json_http::{ReqwestJsonHttpClient, StaticHeaderAuth};

let deepseek_default = DeepSeekModel::new(Arc::new(ReqwestJsonHttpClient::new()), "api-key");
let deepseek_custom = DeepSeekModel::with_catalog_auth(
    Arc::new(ReqwestJsonHttpClient::new()),
    ModelSpec::custom(
        deepseek::PROVIDER_ID,
        "deepseek-v4-preview",
        &deepseek::CUSTOM_DEFAULTS,
    ),
    Arc::new(StaticHeaderAuth::bearer_token("api-key")),
);
let minimax = MiniMaxModel::new(Arc::new(ReqwestJsonHttpClient::new()), MINIMAX_M3, "api-key");
```

## Development

```sh
cargo build -p ai-models-compat --all-targets --all-features
cargo test -p ai-models-compat --all-features
cargo clippy -p ai-models-compat --all-targets --all-features
```

To add a dialect, add its provider id, specs, catalog, custom defaults, and
public constructor wrapper. Supply its settings and isolated protocol quirks
to the engine. Reuse the shared request,
stream, response, validation, and usage code. Put dialect tests in its
`_tests_` folder. Add recordings and conformance coverage in `ai-testkit`.

### Key Code

- `src/engine/` owns request mapping, dispatch, SSE consumption, model event
  mapping, the deferred event stream, and response validation.
- `src/chat_completions/` owns the typed chunks, their indexed accumulation
  state, and their errors.
- The dialect folders own construction, specs, custom defaults, settings, and
  isolated quirks. Each `catalog.rs` holds the provider id, the specs, and
  the custom defaults of its dialect.

### Related Docs

- [Architecture](../../docs/architecture.md)
- [Target dialect protocol](../../docs/protocol/chat-completions-dialects.md)
- [Model stream](../../docs/protocol/model-stream.md)
- [Model capabilities](../../docs/protocol/model-capabilities.md)
- [Provider controls](../../docs/protocol/provider-call-controls.md)
- [DeepSeek](../../docs/protocol/deepseek-model-provider.md)
- [Kimi](../../docs/protocol/kimi-model-provider.md)
- [Qwen](../../docs/protocol/qwen-model-provider.md)
- [MiniMax](../../docs/protocol/minimax-model-provider.md) and its
  [responses](../../docs/protocol/minimax-responses.md)
- [Adapter helpers](../ai-models-core/README.md)
