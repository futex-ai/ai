# ai

Standalone Rust workspace for AI interfaces, model-provider adapters,
provider-agnostic model policy, model fallback routing, JSON HTTP transport, and
in-memory tool-calling runtime behavior.

## Features

- Shared `ai-interface` contracts for conversations, buffered model calls,
  opt-in ordered assistant/reasoning/restart completion events, audio
  transcription, one-image generation and editing, one-video generation, tool
  calls, routing, logging, usage metering, and bounded model-visible tool output
  envelopes
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
  caller-named models, internally streamed completions and public incremental events for
  every synchronous provider path (with xAI's deferred path remaining buffered
  and event-silent), structured output, usage normalization, effort lowering
  to the efforts of each model spec, and typed errors
- TypeSafe Jev judgment evaluation through a provider-neutral typed boundary
  for condition, choice, and score questions over shared text or JSON state
- Provider-agnostic wrappers for retry, concurrency, structured output
  validation, usage pricing, streaming failure classification, and
  chat-completions delta accumulation
- Ordered fallback model composition through `ai-models-multi`
- Trait-backed buffered and incremental SSE HTTP client support through
  `json-http`, with connect, idle, and overall timeout controls
- In-memory tool-calling runtime through `ai-tool-calling`, including
  universal tool output management with inline envelopes, stored output ids,
  UTF-8-safe windows, and degraded-window fallbacks
- Streamable HTTP MCP tool discovery and dispatch through `ai-mcp`
- Host-side MCP OAuth discovery, PKCE, refresh, and request authentication
  through `ai-mcp-oauth`

## Protocols

- [Architecture](docs/architecture.md) (approved, not implemented) describes
  the target components, their boundaries, and the flow of a model call and
  an agent turn.
- [Model stream](docs/protocol/model-stream.md) (approved, not implemented)
  defines the one stream method, its events and their order, the fold,
  deadlines, interruption, and the policy layers on the stream.
- [Conversation parts](docs/protocol/conversation-parts.md) (approved, not
  implemented) defines roles, parts, opaque provider state, replay rules, the
  stored format and its converter, and log copies.
- [Model capabilities](docs/protocol/model-capabilities.md) defines model
  specs, provider ids, text ids, custom specs, effort, the capability check,
  and the generated control matrix.
- [Chat Completions dialects](docs/protocol/chat-completions-dialects.md)
  (approved, not implemented) defines the shared Chat Completions engine and
  the settings of its DeepSeek, Kimi, Qwen, MiniMax, and xAI dialects.
- [Provider registry](docs/protocol/provider-registry.md) (approved, not
  implemented) defines provider config, base URLs, auth, headers, timeouts,
  the adapters that a spec builds, and the converter for stored
  conversations.
- [Model policies](docs/protocol/model-policies.md) (approved, not
  implemented) defines the error core and kinds, retry, fallback policies,
  concurrency, and pricing.
- [Agent loop](docs/protocol/agent-loop.md) (approved, not implemented)
  defines turns over a caller-owned conversation, agent events, parallel
  tools, outcomes, observers, and the output reader.
- [Model completion events](docs/protocol/model-completion-events.md) defines
  the opt-in public assistant, reasoning, and fallback-restart event boundary,
  terminal parity, wrapper semantics, and downstream adoption contract.
- [Model completion streaming](docs/protocol/model-completion-streaming.md)
  defines internal SSE accumulation, idle and overall deadlines, interruption
  retry policy, provider mappings, response parity, and the Firna revision-bump
  contract.
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
- [Video input](docs/protocol/video-input.md) defines the shared video content
  part, the Google and MiniMax mappings, and typed rejection elsewhere.
- [DeepSeek model provider](docs/protocol/deepseek-model-provider.md) defines
  the DeepSeek V4 Pro/Flash catalog, text-only request boundary, thinking,
  replay, tool-calling, JSON-object, usage, and error contract.
- [Kimi model provider](docs/protocol/kimi-model-provider.md) defines the
  implemented Kimi K3 catalog, request, replay, tool-calling,
  structured-output, usage, and error contract.
- [MiniMax model provider](docs/protocol/minimax-model-provider.md) and its
  [response page](docs/protocol/minimax-responses.md) define the provider
  identity, model specs, request/replay, response, usage, and
  error-normalization contract.
- [Qwen model provider](docs/protocol/qwen-model-provider.md) defines the stable
  Qwen 3.7 Max/Plus/Flash catalog, thinking, vision, replay, tool-calling,
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

- `ai-interface`: shared DTOs, traits, mocks, error contracts, model specs,
  provider ids, efforts, the combined model catalog, and model-visible tool
  output envelopes
- `ai-models-core`: reusable model wrappers, the shared capability check, and
  provider helper logic
- `ai-models-anthropic`: Anthropic model adapter
- `ai-models-compat`: one Chat Completions engine with public `deepseek`,
  `kimi`, `qwen`, `minimax`, and `xai` dialect modules and their model specs
- `ai-models-google`: Google Gemini chat/image adapters and a Veo video
  adapter with no catalog model
- `ai-models-openai`: OpenAI model, transcription, and image adapters
- `ai-models-typesafe`: TypeSafe Jev judgment adapter and its model specs
- `ai-models-multi`: ordered fallback model adapter
- `ai-tool-calling`: in-memory tool-calling runtime with output policy, output
  store integration, and the intrinsic `tool_output_read` reader
- `ai-mcp`: MCP 2025-06-18/2025-03-26 streamable HTTP client and
  `ai-interface::Tool` adapter
- `ai-mcp-oauth`: host-side OAuth companion with injected browser, secure
  storage, issuer-selection, clock, randomness, and transport boundaries
- `json-http`: typed JSON and multipart HTTP client boundary
- `xtask`: repository automation invoked with `cargo xtask ...`

### Model specs and effort

Each provider crate exports one `ModelSpec` constant for each model, a
`CATALOG` slice, its `PROVIDER_ID`, and the `CUSTOM_DEFAULTS` of models that
its catalog does not list. `ModelSpec::custom` builds the spec of such a
model from those defaults. A host combines catalogs into a `ModelCatalog`,
resolves `provider/model` text ids when it loads its configuration, and
builds each adapter from a spec. A request sets its effort in
`controls.generation.effort`. The shared capability check lowers an effort
that the spec does not list to the highest listed effort below it, and never
raises an effort. A request without an effort uses the default effort of the
spec.

```rust
use std::sync::Arc;

use ai_interface::{Effort, ModelCallControls, ModelCatalog};
use ai_models_anthropic::AnthropicModel;
use json_http::ReqwestJsonHttpClient;

fn catalog() -> ModelCatalog {
    ModelCatalog::new()
        .with_models(ai_models_anthropic::CATALOG.iter().cloned())
        .with_models(ai_models_openai::CATALOG.iter().cloned())
}

fn build(catalog: &ModelCatalog, api_key: &str) -> Option<AnthropicModel> {
    let spec = catalog.find("anthropic/claude-opus-5")?.clone();
    Some(AnthropicModel::new(Arc::new(ReqwestJsonHttpClient::new()), spec, api_key))
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

Credentialed live checks for models, images, judgments, and videos make
billable calls. Those tests stay ignored, so `cargo xtask check` never runs
them. Run their credential-free guards with:

```sh
cargo test --locked -p xtask --test live_models
cargo test --locked -p xtask --test live_images
cargo test --locked -p xtask --test live_judgments
```

[`docs/development.md`](docs/development.md) gives the billable commands, the
live workflows, their secrets, and the CI details.

Run local AI review after checks pass and the branch has been pushed:

```sh
cargo xtask review
```

## Key Code

- `Cargo.toml`: workspace membership and shared internal crate dependencies
- `crates/ai-interface`: shared AI contracts, including call controls,
  `src/model_spec.rs`, `src/effort.rs`, `src/model_catalog.rs`,
  `src/model_completion_events.rs`, and `src/output/` envelope DTOs
- `crates/ai-models-core`: provider-agnostic model wrappers and helpers,
  including the shared capability check in `src/capabilities.rs`
- `crates/ai-models-compat`: shared request mapping, streaming, response,
  usage, and errors, with DeepSeek, Kimi, Qwen, MiniMax, and xAI dialects
- `crates/ai-models-typesafe`: TypeSafe Jev catalog, judgment request/response
  mapping, validation, usage, authentication, and provider-error translation
- `crates/ai-models-*`: concrete provider and fallback adapters. Each
  provider crate keeps its provider id, model specs, and custom defaults in
  a `catalog.rs`
- `crates/ai-tool-calling`: in-memory tool-calling runtime, including
  `src/policy.rs`, `src/output_store/`, and the intrinsic output reader
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
- `docs/protocol/model-completion-streaming.md`: approved internal SSE,
  timeout, interruption, parity, and downstream migration contract
- `docs/protocol/model-completion-events.md`: approved public incremental
  event, wrapper, provider, parity, and Firna adoption contract
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
