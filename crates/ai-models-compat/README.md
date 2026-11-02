# ai-models-compat

`ai-models-compat` implements Chat Completions behind `ai_interface::Model`.
Depend on it when a composition root needs a compatible provider with explicit
credentials and an injected `json-http` client.

## Responsibilities

- Map conversations, tools, call controls, and structured output to the wire.
- Accumulate SSE content, reasoning, tool calls, finish state, and final usage.
- Preserve each dialect's replay data, errors, and completion events.
- Export provider catalogs and the existing model constructors.

## What This Crate Does

One engine serves provider dialects. The public `deepseek`, `kimi`, `qwen`, `minimax`, and `xai` modules
export their models, configuration errors, catalog constants, and `known_models()`.
The engine keeps the current buffered `complete` and opt-in
`complete_with_events` methods. Schema calls suppress public text events.
Stream failures after progress return `ModelError::Interrupted`.
MiniMax retains M2.x snapshot recovery, M3 deltas, reasoning details, numeric
provider errors, and structurally validated clean EOF.
xAI retains legacy function calls, deterministic call ids, final usage
replacement, and buffered deferred submission and polling. Deferred calls
suppress events. Their existing timeout classification remains unchanged.

A dialect supplies its endpoint and provider settings. Model wrappers validate
construction and delegate execution to the engine. Callers supply credentials
and apply retry, concurrency, and pricing wrappers at the composition root.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_compat::deepseek::DeepSeekModel;
use json_http::ReqwestJsonHttpClient;

let model = DeepSeekModel::new(Arc::new(ReqwestJsonHttpClient::new()), "api-key");
```

## Development

```sh
cargo build -p ai-models-compat --all-targets --all-features
cargo test -p ai-models-compat --all-features
cargo clippy -p ai-models-compat --all-targets --all-features
```

To add a dialect, add its catalog and public constructor wrapper. Supply its
settings and isolated protocol quirks to the engine. Reuse the shared request,
stream, response, validation, and usage code. Put dialect tests in its
`_tests_` folder. Add recordings and conformance coverage in `ai-testkit`.

### Key Code

- `src/engine/` owns request mapping, dispatch, SSE consumption, and responses.
- `src/chat_completions/` owns the typed chunk accumulator and its errors.
- The dialect folders own construction, catalogs, settings, and isolated quirks.

### Related Docs

- [Architecture](../../docs/architecture.md)
- [Target dialect protocol](../../docs/protocol/chat-completions-dialects.md)
- [Current streaming contract](../../docs/protocol/model-completion-streaming.md)
- [Provider controls](../../docs/protocol/provider-call-controls.md)
- [DeepSeek](../../docs/protocol/deepseek-model-provider.md)
- [Adapter helpers](../ai-models-core/README.md)
