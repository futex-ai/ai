# ai-models-compat

`ai-models-compat` implements Chat Completions behind `ai_interface::Model`.
Depend on it when a composition root needs a compatible provider with explicit
credentials and an injected `json-http` client.

## Responsibilities

- Map conversations, tools, call controls, and structured output to the wire.
- Map SSE chunks to model events: text, reasoning, tool calls, usage, and
  finish.
- Record each dialect's replay data as provider state on the parts that it
  produces, replay own state, and keep each dialect's errors.
- Convert the stored version 1 replay items of each dialect.
- Export each dialect's provider id, model specs, catalog, custom defaults,
  and model constructors.
- Apply shared provider connection settings to streams and deferred requests.

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

Each model also offers `from_config(http, spec, config)` with a
`ai_models_core::ProviderConfig`. This constructor uses the default effort
of its spec. The existing constructors retain their defaults.
Every request uses the config's base URL and keeps its optional path prefix.
A trailing slash changes no endpoint. The default Qwen base URL includes
`/compatible-mode`. API keys use Bearer auth. Auth hooks run once for each
stream opening, deferred submission, or poll. Extra headers have lower
precedence than adapter and auth headers, without regard to name case.

Streams default to a 120-second idle timeout and a 3,600-second total timeout.
xAI deferred calls default to a 60-second total timeout and a 5-second poll
interval. Each poll keeps its 30-second HTTP cap. Config timeout values
replace the relevant defaults. A request's total timeout replaces the config
total timeout and keeps the idle timeout. Polls and sleeps stay within the
remaining deferred deadline. Construction reads no environment variable and
sends no request.

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
after progress return the `Interrupted` error kind.
Moved errors retain their existing message prefixes for response parity.

Errors carry precise shared kinds, adapter identity, Retry-After, and optional
provider request ids. Late stream and output failures retain their response
metadata. Auth hooks apply once per request and diagnostics redact captured
credentials. Each model offers `with_clock` for HTTP-date retry hints.
MiniMax uses `Trace-Id` first; other request-id names are optional aliases.

Before progress, a standard provider error event takes the kind of its known
code: rate-limit codes give `RateLimited`, context codes
`ContextLimitExceeded`, server and overload codes `Transient`, key and quota
codes `RejectedKey`, model codes `UnknownModel`, unsupported-input codes
`UnsupportedInput`, and invalid-output codes `InvalidOutput`. An unknown or
absent code gives `InvalidRequest`, and a provider error event after
progress gives `Interrupted`. MiniMax `base_resp` codes follow the
[MiniMax response protocol](../../docs/protocol/minimax-responses.md).

MiniMax retains M2.x snapshot recovery, M3 deltas, reasoning details, numeric
provider errors, and structurally validated clean EOF. It emits the inferred
M2.x text as one text delta after terminal validation.
xAI retains legacy function calls, deterministic call ids, final usage
replacement, and deferred submission and polling. A deferred call emits
every event of the call after the provider result arrives. Its existing
timeout classification remains `Transient`. Polling continues after production
transport failures through the shared transport predicate, with one submission.

DeepSeek keeps its `insufficient_system_resource` finish as `Transient`.

Each dialect records its replay data as provider state of its own provider
id on the part that produced it. The text part keeps the raw content, the
reasoning part keeps the raw reasoning, and each tool-call part keeps its raw
call. The provider doc of each dialect says which values it keeps. The
engine prefers own state to the portable parts, and it ignores the state of
every other provider. Own state with an unknown version or a payload that
does not decode fails before transport with the `InvalidRequest` error kind. A
dialect writes state with version 2, and it reads the version 1 state that
holds the fields of an old item.

Each dialect module exports a `LegacyConverter` for its stored version 1
items, such as `deepseek::LegacyConverter`. The DeepSeek, Kimi, Qwen, and
MiniMax converters put the item on a new reasoning part. The xAI converter
puts its legacy function call on the matching tool-call part. The parts come
in the order reasoning, text, then portable tool calls. Add each converter to
the `LegacyMessageConverter` of the host. MiniMax replays only the reasoning
and details of the first version 1 `minimax_assistant` item, also on plain
serde reads without a converter. Native version 2 reasoning parts retain
their separate accumulation.

A dialect supplies its endpoint, usage fields, replay rule, and wire settings.
Model wrappers hold the spec and delegate execution to the engine. Callers
supply credentials and apply the retry, fallback, concurrency, and pricing
layers of [`ai-models-policy`](../ai-models-policy/README.md) at the
composition root.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::ModelSpec;
use ai_models_compat::{
    deepseek::{self, DeepSeekModel},
    minimax::{MINIMAX_M3, MiniMaxModel},
};
use ai_models_core::ProviderConfig;
use json_http::{ReqwestJsonHttpClient, StaticHeaderAuth};

let deepseek_default = DeepSeekModel::new(Arc::new(ReqwestJsonHttpClient::new()), "api-key");
let deepseek_gateway = DeepSeekModel::from_config(
    Arc::new(ReqwestJsonHttpClient::new()),
    deepseek::DEEPSEEK_V4_FLASH,
    ProviderConfig::new(deepseek::PROVIDER_ID, "api-key")
        .with_base_url("https://gateway.example/provider"),
);
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
- `src/engine/client.rs` applies shared provider config and request timeout
  precedence before stream or deferred dispatch.
- `src/xai/deferred.rs` applies config to submission and every bounded poll.
- `src/engine/replay_state.rs` holds the private, versioned part state, and
  `src/engine/request_replay.rs` decodes own state before transport.
- `src/chat_completions/` owns the typed chunks, their indexed accumulation
  state, and their errors.
- The dialect folders own construction, specs, custom defaults, settings, and
  isolated quirks. Each `catalog.rs` holds the provider id, the specs, and
  the custom defaults of its dialect. Each `state.rs` decodes the version 1
  state of its dialect and holds its `LegacyConverter`.

### Related Docs

- [Architecture](../../docs/architecture.md)
- [Target dialect protocol](../../docs/protocol/chat-completions-dialects.md)
- [Model stream](../../docs/protocol/model-stream.md)
- [Conversation parts](../../docs/protocol/conversation-parts.md)
- [Model capabilities](../../docs/protocol/model-capabilities.md)
- [Model policies](../../docs/protocol/model-policies.md)
- [Provider controls](../../docs/protocol/provider-call-controls.md)
- [Provider configuration](../../docs/protocol/provider-registry.md)
- [DeepSeek](../../docs/protocol/deepseek-model-provider.md)
- [Kimi](../../docs/protocol/kimi-model-provider.md)
- [Qwen](../../docs/protocol/qwen-model-provider.md)
- [MiniMax](../../docs/protocol/minimax-model-provider.md) and its
  [responses](../../docs/protocol/minimax-responses.md)
- [Adapter helpers](../ai-models-core/README.md)
