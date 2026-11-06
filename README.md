# ai

Standalone Rust workspace for AI contracts, provider adapters, model policies,
and agent turns. Hosts supply credentials, transport, tools, and conversation
storage. The libraries keep provider wire formats separate from policies and
agent orchestration.

## Features

- Eight chat providers: Anthropic, OpenAI, Google Gemini, DeepSeek, Kimi,
  Qwen, MiniMax, and xAI.
- One pull-based model event stream, with a shared fold for complete responses.
- Ordered conversation parts with opaque provider state for replay.
- Catalog model specs, portable effort controls, and adapters built through
  one registry.
- Image generation and editing, Google video generation for caller-named
  models, OpenAI transcription, and TypeSafe judgment.
- Retry, explicit fallback policies, concurrency limits, and usage pricing.
- Agent turns with parallel tools, bounded output, checkpoints, observers,
  and separate completed, truncated, and capped outcomes.
- MCP tool discovery and dispatch, with host-side OAuth support.
- Offline conformance recordings, request comparisons, and file snapshots.

## Crates

| Component | Crate | Role |
| --- | --- | --- |
| Contracts | [ai-interface](crates/ai-interface/README.md) | Model, tool, and modality traits, events and their fold, conversation parts, specs, effort, usage, and typed errors. |
| HTTP | [json-http](crates/json-http/README.md) | Injected JSON and multipart transport, SSE streams, response headers, deadlines, and test recording and replay. |
| Adapter helpers | [ai-models-core](crates/ai-models-core/README.md) | Capability checks, schema checks, error classification, synthetic call ids, and shared provider config. |
| Native adapters | [ai-models-anthropic](crates/ai-models-anthropic/README.md), [ai-models-openai](crates/ai-models-openai/README.md), [ai-models-google](crates/ai-models-google/README.md), [ai-models-typesafe](crates/ai-models-typesafe/README.md) | Native provider protocols and catalogs. OpenAI adds image and transcription adapters. Google adds image and video adapters. TypeSafe supplies judgment. |
| Chat Completions engine | [ai-models-compat](crates/ai-models-compat/README.md) | One engine with dialects for DeepSeek, Kimi, Qwen, MiniMax, and xAI. xAI also supports deferred completion. |
| Policies | [ai-models-policy](crates/ai-models-policy/README.md) | Provider-neutral retry, fallback, concurrency, and pricing wrappers around any model. |
| Registry | [ai-models-registry](crates/ai-models-registry/README.md) | Provider config, combined catalogs, adapter factories, and the composed stored-message converter. |
| Agent loop | [ai-tool-calling](crates/ai-tool-calling/README.md) | Turns over caller-owned conversations, streamed events, parallel tools, typed tools, and tool output windows. |
| MCP | [ai-mcp](crates/ai-mcp/README.md), [ai-mcp-oauth](crates/ai-mcp-oauth/README.md) | Streamable HTTP MCP client, tool adapter, OAuth discovery, PKCE, token refresh, and request authentication. |
| Test kit | [ai-testkit](crates/ai-testkit/README.md) | Synthetic recordings, model and agent conformance, request comparisons, snapshots, and catalog replay checks. |
| Automation | [xtask](xtask/README.md) | Checks, smoke tests, capability matrix generation, explicit recording, live checks, and review commands. |

The contracts name no provider. Adapters own provider wire formats and replay
state. Policies and the agent loop depend on the contracts. The registry
combines the providers that its Cargo features include. Product libraries do
not depend on the test kit.

## Usage

Resolve a `provider/model` text id with `ai_models_registry::catalog().find`.
Pass the selected `ModelSpec` and provider credentials to this example.
It builds a registry adapter, adds a concurrency policy, and runs an agent turn.

```rust
use std::{error::Error, sync::Arc};

use ai_interface::{ConversationMessage, ModelSpec, NoopLogger};
use ai_models_policy::ConcurrencyLimitedModel;
use ai_models_registry::{ProviderConfig, Registry};
use ai_tool_calling::{
    Agent, AgentEvent, AgentRequest, InMemoryToolOutputStore,
    ToolCallingRuntime, ToolOutputPolicy,
};
use json_http::ReqwestJsonHttpClient;

async fn run(spec: &ModelSpec, api_key: &str) -> Result<(), Box<dyn Error>> {
    let registry = Registry::new(Arc::new(ReqwestJsonHttpClient::new()))
        .with_provider(ProviderConfig::new(spec.provider.clone(), api_key));
    let model = Arc::new(ConcurrencyLimitedModel::new(
        registry.model(spec)?, spec.text_id(), 4,
    ));
    let agent = ToolCallingRuntime::new(
        model,
        Arc::new(NoopLogger),
        Vec::new(),
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )?;
    let mut conversation = vec![ConversationMessage::user("Say done.")];
    let request = AgentRequest {
        system_prompt: "Keep answers short.".to_owned(),
        max_steps: Some(4),
        ..Default::default()
    };
    let mut events = agent.run(&mut conversation, request);
    while let Some(event) = events.next().await? {
        if let AgentEvent::Finished(outcome) = event {
            println!("{outcome:?}");
        }
    }
    Ok(())
}
```

Pass host tools or MCP tool adapters in the third constructor argument to
allow tool calls. Use a fresh output store for each active run. The caller
keeps the conversation, including all assistant parts and their provider state.

`ProviderConfig` supplies a base URL, an API key or auth hook, extra headers,
and optional timeouts. Each provider has a registry Cargo feature.
The default features include all nine providers. Building an adapter sends
no request and reads no environment variable or secret store. The registry
adds no policy layer. The provider constructors remain available while the
constructor default-effort decision stays open.

Each spec lists its supported efforts and its default. Set
`controls.generation.effort` on a model or agent request to select effort.
The capability check lowers unsupported effort to a supported value and
never raises it. A spec without a default effort leaves the reasoning to the
provider. Such a call sends no thinking field and reports no effort. A
disabled request sends the explicit off value of the provider. It fails
before transport when the provider has none.
The [control matrix](docs/protocol/model-capabilities.md#control-matrix)
lists the limits and controls. Regenerate it after a spec change with
`cargo xtask capability-matrix`.

For direct model calls, `Model::stream` returns events and `Model::complete`
folds them into a response. Keep the complete response in history with
`ModelResponse::to_assistant_message()`. Use `log_copy()` for logs.
It removes provider state and reasoning text.

Stored messages write format version 2. Read stored messages through
`registry.converter().read(value)` to convert version 1 with each provider's
replay rules. Plain serde reads use the generic conversion without provider
ordering. No catalog lists video or transcription models. Build those
adapters from custom specs that name the provider model.

## Protocols

- [Architecture](docs/architecture.md) describes the components, their
  boundaries, and the flow of a model call and an agent turn.
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

## Developer Get Started

```sh
cargo metadata --format-version 1 --no-deps
cargo xtask check
```

The complete gate runs formatting, Clippy, workspace tests, the Rust file-length
lint, and the smoke test. Run focused tests during development:

```sh
cargo test -p ai-testkit --test conformance
cargo test -p ai-tool-calling
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo xtask rust-file-length-lint
cargo xtask smoke-test
```

The smoke test builds every provider adapter with placeholder credentials.
It exercises the in-memory agent without provider requests.
The conformance suite runs all six model scenarios for each chat adapter.
It also runs agent text turns and tool loops, and checks each catalog effort
that needs replay. All recordings remain synthetic.

Credentialed model, image, and judgment checks make billable calls.
They stay ignored during `cargo xtask check`. Their credential-free guards are:

```sh
cargo test --locked -p xtask --test live_models
cargo test --locked -p xtask --test live_images
cargo test --locked -p xtask --test live_judgments
```

The [development guide](docs/development.md) gives the billable commands,
workflow triggers, secrets, CI details, and downstream adoption steps.
After checks pass and a branch has been pushed, `cargo xtask review` runs the
local AI review.

## Key Code

- `crates/ai-interface/src/model_spec.rs` defines catalog specs and replay needs.
- `crates/ai-interface/src/model_response_fold/` folds events into ordered parts.
- `crates/ai-models-core/src/capabilities.rs` resolves request controls against specs.
- `crates/ai-models-compat/src/engine/` implements the shared Chat Completions flow.
- `crates/ai-models-registry/src/registry.rs` builds adapters from provider config.
- `crates/ai-models-policy/src/` holds the model wrappers.
- `crates/ai-tool-calling/src/agent.rs` defines agent requests and events.
- `crates/ai-tool-calling/src/turn/` runs model steps and tool calls.
- `crates/ai-testkit/tests/_tests_/` holds model, agent, and replay conformance tests.
- `crates/ai-testkit/recordings/` holds the sanitized synthetic exchanges.
- `xtask/` holds repository automation. Each crate README gives its other entry points.

## CI

GitHub Actions runs the same formatting, Clippy, tests, file-length lint, and
credential-free smoke checks as the local gate. Separate workflows run the
billable model, image, and judgment suites. The
[CI guide](docs/development.md#ci) describes their triggers and secrets.
