# ai

Standalone Rust workspace for AI interfaces, model-provider adapters,
provider-agnostic model policy, model fallback routing, JSON HTTP transport, and
in-memory tool-calling runtime behavior.

## Features

- Shared `ai-interface` contracts for conversations, one pull-based model
  stream of ordered text, reasoning, tool-call, usage, finish, and fallback
  events with a provided `complete` that folds the stream into one response,
  audio transcription, one-image generation and editing, one-video
  generation, tool calls, routing, logging, usage metering, and bounded
  model-visible tool output envelopes
- Conversations of ordered text, image, video, tool-call, tool-result, and
  reasoning parts. Each part can carry opaque provider state that only its
  adapter reads. Messages store as format version 2, and each provider crate
  converts its own replay items of the old format
- One `ModelSpec` constant for each provider model, with stable provider ids,
  `provider/model` text ids, a combined catalog lookup, and custom specs for
  unlisted models
- Typed provider-neutral generation controls, including a per-request effort,
  explicit required-with-automatic-fallback tool selection, per-call
  deadlines, and completion preferences. One shared capability check resolves
  every control against the model spec before transport, while every provider
  adapter owns its native wire mapping and XAI owns deferred submission and
  polling
- Provider adapters for Anthropic, DeepSeek, Google Gemini, Kimi, MiniMax,
  OpenAI, QwenCloud, and xAI models, including provider-specific tools,
  reasoning replay, vision where supported, Google and MiniMax inline video
  input, OpenAI and Gemini image generation, a Google Veo video adapter for
  caller-named models, model events mapped directly from each provider stream
  (also for calls with a response schema and for xAI's deferred path), structured
  output, usage normalization, effort lowering to the efforts of each model
  spec, and typed errors
- TypeSafe Jev judgment evaluation through a provider-neutral typed boundary
  for condition, choice, and score questions over shared text or JSON state
- One error core for every model, image, video, transcription, and judgment
  error: a precise kind that says whether another model can succeed, the
  provider, the model, a credential-free message, the provider's
  retry-after, and its request id
- Provider-agnostic policy layers in `ai-models-policy`: retry with jitter
  that honours retry-after and covers rate limits, ordered fallback with a
  required policy and a fallback event for each failed model, concurrency
  limits, and usage pricing
- A provider registry in `ai-models-registry` that builds model, image,
  video, transcription, and judgment adapters from a spec and one provider
  config for each provider: base URL, API key or auth hook, extra headers,
  and timeouts. It combines the catalogs and the stored-message converters
  of the providers that its Cargo features include
- Shared adapter helpers in `ai-models-core` for structured output
  validation and HTTP and stream failure classification
- Trait-backed buffered and incremental SSE HTTP client support through
  `json-http`, with connect, idle, and overall timeout controls, response
  headers on responses, streams, and failures, and typed request bodies
  serialized straight to bytes
- Agent turns through `ai-tool-calling` over caller-owned conversations,
  streamed model and tool events, parallel tools, checkpoints, observers,
  distinct completed/truncated/capped outcomes, typed tools with generated
  schemas, and bounded tool output with UTF-8-safe windows
- Streamable HTTP MCP tool discovery and dispatch through `ai-mcp`
- Host-side MCP OAuth discovery, PKCE, refresh, and request authentication
  through `ai-mcp-oauth`

## Protocols

- [Architecture](docs/architecture.md) (approved, not implemented) describes
  the target components, their boundaries, and the flow of a model call and
  an agent turn.
- [Model stream](docs/protocol/model-stream.md) and its
  [adapter page](docs/protocol/model-stream-adapters.md) define the one
  stream method, its events and their order, the fold, deadlines,
  interruption, the policy layers on the stream, the native adapter mapping,
  and the SSE transport.
- [Conversation parts](docs/protocol/conversation-parts.md) defines roles,
  parts, opaque provider state, replay rules, the stored format and its
  converter, and log copies.
- [Model capabilities](docs/protocol/model-capabilities.md) defines model
  specs, provider ids, text ids, custom specs, effort, the capability check,
  and the generated control matrix.
- [Chat Completions dialects](docs/protocol/chat-completions-dialects.md)
  defines the shared Chat Completions engine and the settings of its
  DeepSeek, Kimi, Qwen, MiniMax, and xAI dialects.
- [Provider registry](docs/protocol/provider-registry.md) defines provider
  config, base URLs, auth, headers, timeouts, the adapters that a spec
  builds, and the converter for stored conversations.
- [Model policies](docs/protocol/model-policies.md) defines the error core
  and kinds, retry, fallback policies, concurrency, and pricing.
- [Agent loop](docs/protocol/agent-loop.md)
  defines turns over a caller-owned conversation, agent events, parallel
  tools, outcomes, observers, and the output reader.
- [Provider call controls](docs/protocol/provider-call-controls.md) defines
  portable generation and execution intent, the error and fixed-value
  contract, blank system handling, full Google schemas, and XAI deferred
  completion.
- [Image generation](docs/protocol/image-generation.md) defines the shared
  one-image generation/editing boundary and the OpenAI and Google mappings.
- [Video generation](docs/protocol/video-generation.md) defines the shared
  one-video generation boundary, portable controls, and the asynchronous
  Google mapping.
- [Live image API tests](docs/protocol/live-image-api-tests.md) defines the
  implemented credentialed catalog coverage, low-cost probe, response
  validation, and CI secret boundary for image providers.
- [TypeSafe judgment provider](docs/protocol/typesafe-judgment-provider.md)
  and its [judgment boundary](docs/protocol/judgment-boundary.md) define the
  implemented provider-agnostic condition/choice/score judgment boundary and
  the TypeSafe Jev wire, validation, usage, and error mapping.
- [Live judgment API tests](docs/protocol/live-judgment-api-tests.md) defines
  the implemented credentialed judgment catalog coverage, probe, validation,
  and CI secret boundary.
- [Video input](docs/protocol/video-input.md) defines the shared video part,
  the Google and MiniMax mappings, and typed rejection elsewhere.
- [DeepSeek model provider](docs/protocol/deepseek-model-provider.md) and its
  [response page](docs/protocol/deepseek-responses.md) define the DeepSeek V4
  Pro/Flash catalog, text-only request boundary, thinking, replay,
  tool-calling, JSON-object, usage, and error contract.
- [Kimi model provider](docs/protocol/kimi-model-provider.md) and its
  [response page](docs/protocol/kimi-responses.md) define the implemented
  Kimi K3 catalog, request, replay, tool-calling, structured-output, usage,
  and error contract.
- [MiniMax model provider](docs/protocol/minimax-model-provider.md) and its
  [response page](docs/protocol/minimax-responses.md) define the provider
  identity, model specs, request/replay, response, usage, and
  error-normalization contract.
- [Qwen model provider](docs/protocol/qwen-model-provider.md) and its
  [response page](docs/protocol/qwen-responses.md) define the stable Qwen
  3.7 Max/Plus/Flash catalog, thinking, vision, replay, tool-calling,
  structured-output, usage, and error contract.
- [Live model API tests](docs/protocol/live-model-api-tests.md) define the
  credentialed provider/catalog coverage, assertions, CI schedule, and secret
  boundary.
- [Tool output management](docs/protocol/tool-output-management.md) defines the
  universal output-id, bounded-envelope, pagination, and raw-output isolation
  contract for tool calls.
- [AI MCP client and tool adapter](docs/protocol/ai-mcp.md) defines streamable
  HTTP transport, session lifecycle, protocol mapping, and tool adaptation.
- [Host-side MCP OAuth](docs/protocol/mcp-oauth.md) defines discovery,
  registration, PKCE, token lifecycle, and request authentication.

## Interfaces

The workspace is library-first. Consumers depend on the crate matching the
boundary they need:

- `ai-interface`: shared DTOs, traits, mocks, error contracts, the model
  event stream and its fold, conversation parts and provider state, the
  stored message reader, model specs, provider ids, efforts, the combined
  model catalog, and model-visible tool output envelopes
- `ai-models-core`: the shared capability check, the shared
  `ProviderConfig`, error classification with response metadata, and
  provider helper logic
- `ai-models-anthropic`: Anthropic model adapter
- `ai-models-compat`: one Chat Completions engine with public `deepseek`,
  `kimi`, `qwen`, `minimax`, and `xai` dialect modules and their model specs
- `ai-models-google`: Google Gemini chat/image adapters and a Veo video
  adapter with no catalog model
- `ai-models-openai`: OpenAI model, transcription, and image adapters
- `ai-models-typesafe`: TypeSafe Jev judgment adapter and its model specs
- `ai-models-policy`: retry, fallback, concurrency, and pricing layers
- `ai-models-registry`: `Registry`, which holds one `ProviderConfig` for each
  provider and builds model, image, video, transcription, and judgment
  adapters from specs, with the combined catalog, the composed
  stored-message converter, and typed `BuildError` values
- `ai-tool-calling`: `Agent::run` over caller-owned conversations, agent
  events, parallel tools, checkpoints, safe observers, `TypedToolAdapter`,
  output policy, and the conditional `tool_output_read` reader
- `ai-mcp`: MCP 2025-06-18/2025-03-26 streamable HTTP client and
  `ai-interface::Tool` adapter
- `ai-mcp-oauth`: host-side OAuth companion with injected browser, secure
  storage, issuer-selection, clock, randomness, and transport boundaries
- `json-http`: typed JSON and multipart HTTP client boundary
- `xtask`: repository automation invoked with `cargo xtask ...`

### Provider registry

`Registry::new` takes the injected HTTP client. `with_provider` and `insert`
store one `ProviderConfig` for each provider: its id, an optional base URL,
an API key or auth hook, extra headers, and optional idle, total, and poll
timeouts. A new config replaces the earlier config of its provider. `model`,
`image`, `video`, `transcription`, and `judgment` build an adapter from a
spec, with the client and the config of its provider. A timeout that the
config does not set keeps the adapter default. Building sends no request and
reads no environment variable or secret store.

- Every provider except TypeSafe builds models. OpenAI and Google build
  image adapters, Google builds video adapters, OpenAI builds transcription
  adapters, and TypeSafe builds judgment adapters.
- A build fails with a `BuildError` when no Cargo feature includes the
  provider, when the provider has no config, or when it has no adapter of
  that kind.
- Each provider has one Cargo feature with the name of its provider id. The
  default features include all nine. The catalog, the builders, and the
  converter cover only the included providers.
- The registry adds no policy layer. Compose the layers of
  `ai-models-policy` around the built model.
- The per-provider constructors stay until the open constructor
  default-effort decision is answered. Prefer the registry in new code.

No catalog lists a video model or a transcription model. Build these
adapters from a custom spec that names the provider model:

```rust
use ai_interface::{DynVideoGenerator, ModelSpec};
use ai_models_registry::{BuildResult, Registry};

fn veo(registry: &Registry, model_id: String) -> BuildResult<DynVideoGenerator> {
    let spec = ModelSpec::custom(
        ai_models_google::PROVIDER_ID,
        model_id,
        &ai_models_google::CUSTOM_DEFAULTS,
    );
    registry.video(&spec)
}
```

### Model specs and effort

Each provider crate exports one `ModelSpec` constant for each model, a
`CATALOG` slice, its `PROVIDER_ID`, and the `CUSTOM_DEFAULTS` of models that
its catalog does not list. `ModelSpec::custom` builds the spec of such a
model from those defaults. `ai_models_registry::catalog()` and
`Registry::catalog` combine the catalogs of the included providers into one
`ModelCatalog`, in alphabetical provider order. A host resolves
`provider/model` text ids when it loads its configuration, and the registry
builds each adapter from a spec. A request sets its effort in
`controls.generation.effort`. The shared capability check lowers an effort
that the spec does not list to the highest listed effort below it, and never
raises an effort. A request without an effort uses the default effort of the
spec. A spec without a default effort leaves the reasoning to the provider.
Such a call sends no thinking field and reports no effort. A disabled
request sends the explicit off value of the provider. It fails before
transport when the provider has none.

```rust
use std::{sync::Arc, time::Duration};

use ai_interface::{DynModel, Effort, ModelCallControls, ProviderId};
use ai_models_registry::{BuildResult, ProviderConfig, Registry};
use json_http::ReqwestJsonHttpClient;

fn registry(api_key: &str) -> Registry {
    let anthropic = ProviderConfig::new(ProviderId::new("anthropic"), api_key)
        .with_base_url("https://gateway.example.com/anthropic")
        .with_total_timeout(Duration::from_secs(600));
    Registry::new(Arc::new(ReqwestJsonHttpClient::new())).with_provider(anthropic)
}

fn build(registry: &Registry) -> Option<BuildResult<DynModel>> {
    let spec = registry.catalog().find("anthropic/claude-opus-5")?.clone();
    Some(registry.model(&spec))
}

fn max_effort() -> ModelCallControls {
    let mut controls = ModelCallControls::default();
    controls.generation.effort = Some(Effort::Max);
    controls
}
```

The [control matrix](docs/protocol/model-capabilities.md#control-matrix)
lists the efforts, limits, and control rules of every spec. After a change
to a spec or to custom defaults, regenerate it with
`cargo xtask capability-matrix`.

### Conversations and stored messages

A message has a role and ordered parts. A part can carry provider state.
Only the adapter of that provider reads it, and every other adapter ignores
it. Keep every part of a response in the conversation, for example with
`ModelResponse::to_assistant_message()`, so that the next request replays
it. Log only `log_copy()` of a request, a message, or a response. A log copy
keeps every part, drops every provider state, and empties the reasoning
text.

`ConversationMessage` writes stored format version 2. Each provider crate
exports the converter of its own version 1 replay items.
`Registry::converter` composes the converters of the included providers.
Read every stored message through that composition:

```rust
use ai_interface::{ConversationMessage, StoredMessageError};
use ai_models_registry::Registry;
use serde_json::Value;

fn load(registry: &Registry, stored: Value) -> Result<ConversationMessage, StoredMessageError> {
    // Reads version 2, and converts version 1 with the provider converters.
    registry.converter().read(stored)
}

fn save(message: &ConversationMessage) -> serde_json::Result<Value> {
    serde_json::to_value(message)
}
```

Plain `serde` deserialization also reads version 1, but it applies no
provider conversion. It keeps each old item whole as provider state, in the
generic part order. Use the composition, which supplies the conversion and
the part order of each provider. The composition keeps the items of a
provider that the registry does not include as provider state. A host
without the registry can compose `LegacyMessageConverter::new` from the
converters that the provider crates export.

## Developer Get Started

```sh
cargo metadata --format-version 1 --no-deps
cargo xtask check
```

Targeted checks:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace --all-features
cargo xtask rust-file-length-lint --all
cargo xtask smoke-test
```

The smoke test constructs every provider adapter with placeholder credentials
and exercises the in-memory tool runtime. It performs no provider requests and
does not require credentials or network access.

Credentialed live checks for models, images, and judgments make billable
calls. Those tests stay ignored, so `cargo xtask check` never runs
them. Run their credential-free guards with:

```sh
cargo test --locked -p xtask --test live_models
cargo test --locked -p xtask --test live_images
cargo test --locked -p xtask --test live_judgments
```

[`docs/development.md`](docs/development.md) gives the billable commands, the
live workflows, their secrets, the CI details, and the steps that the
downstream host Firna takes to adopt the model stream.

Run local AI review after checks pass and the branch has been pushed:

```sh
cargo xtask review
```

## Key Code

- `Cargo.toml`: workspace membership and shared internal crate dependencies
- `crates/ai-interface`: shared AI contracts, including call controls,
  `src/model_spec.rs`, `src/effort.rs`, `src/model_catalog.rs`,
  `src/model_event.rs`, `src/model_stream.rs`, the
  `src/model_response_fold/` fold, the `src/messages/` conversation parts,
  provider state, and stored format, and `src/output/` envelope DTOs
- `crates/ai-models-core`: provider-agnostic adapter helpers, including the
  shared capability check in `src/capabilities.rs`, the per-call error
  context in `src/error_context.rs`, and the shared `ProviderConfig` in
  `src/provider_config.rs`
- `crates/ai-models-policy`: retry, fallback, concurrency, and pricing
  layers
- `crates/ai-models-registry`: the `Registry` builders, the feature-selected
  combined catalog and converter, and `BuildError`
- `crates/ai-models-compat`: shared request mapping, streaming, response,
  usage, and errors, with DeepSeek, Kimi, Qwen, MiniMax, and xAI dialects
- `crates/ai-models-typesafe`: TypeSafe Jev catalog, judgment request/response
  mapping, validation, usage, authentication, and provider-error translation
- `crates/ai-models-*`: concrete provider adapters. Each
  provider crate keeps its provider id, model specs, and custom defaults in
  a `catalog.rs`
- `crates/ai-tool-calling`: `src/agent.rs` defines requests and events,
  `src/runtime.rs` builds agents, `src/turn/` runs streamed turns and parallel
  tools, and `src/typed_tool.rs` adapts typed tools. `src/policy.rs` and
  `src/output_store/` bound model-visible output
- `crates/ai-mcp`: streamable HTTP MCP protocol client, authorization
  challenges, and tool adapter
- `crates/ai-mcp-oauth`: protected-resource discovery, public-client
  registration, PKCE authorization, token lifecycle, and MCP auth hook
- `crates/json-http`: HTTP client abstraction used by provider crates
- `xtask/`: local automation for checks, smoke tests, file-length lint, the
  capability matrix, stream recording, and review
- `docs/protocol/tool-output-management.md`: normative universal tool output
  management contract
- `docs/protocol/image-generation.md`: normative shared image generation and
  provider mapping contract
- `docs/protocol/live-image-api-tests.md`: implemented credentialed image-provider
  catalog and CI verification contract
- `docs/protocol/video-generation.md`: normative shared video generation and
  asynchronous provider mapping contract
- `docs/protocol/typesafe-judgment-provider.md`: normative shared judgment
  boundary and TypeSafe Jev provider contract
- `docs/protocol/live-judgment-api-tests.md`: implemented credentialed judgment
  catalog and CI verification contract
- `docs/protocol/model-capabilities.md`: implemented model spec, effort,
  capability check, and generated control matrix contract
- `docs/protocol/provider-call-controls.md`: normative model-call control and
  provider wire-compatibility contract
- `docs/protocol/model-stream.md`: implemented model stream contract: the
  stream method, events, fold, deadlines, interruption, SSE transport, and
  the policy layers on the stream
- `docs/protocol/conversation-parts.md`: implemented conversation parts,
  provider state, stored format, version 1 converter, and log copy contract
- `docs/protocol/model-policies.md`: implemented error core, error kinds,
  retry, fallback, concurrency, and pricing contract
- `docs/protocol/deepseek-model-provider.md`: normative DeepSeek V4 provider
  contract
- `docs/protocol/kimi-model-provider.md`: normative Kimi K3 provider contract
- `docs/protocol/minimax-model-provider.md`: normative MiniMax adapter contract
- `docs/protocol/qwen-model-provider.md`: normative Qwen 3.7 provider contract
- `docs/protocol/`: other normative contracts for shared runtime behavior

## CI

GitHub Actions runs the same Rust verification expected locally on pull requests
and pushes to `main`: formatting, Clippy, tests, Rust file-length lint,
credential-free smoke tests, and `cargo xtask check`. Separate workflows run
the billable live model, image, and judgment suites. The
[CI section of the development guide](docs/development.md#ci) describes each
workflow, its triggers, and its secrets.
