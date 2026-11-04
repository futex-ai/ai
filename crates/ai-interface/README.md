# ai-interface

`ai-interface` is the shared contract crate for AI-facing runtime boundaries in
this workspace. Depend on it when you need the common conversation, model,
model spec, effort, tool, audio transcription, image generation, video
generation, judgment, routing, logging, usage, or model-visible tool output
DTOs without taking a dependency on a stateful runtime implementation.

## Responsibilities

- Own shared DTOs for conversations, model calls, tool calls, audio
  transcription, image generation/editing, video generation, typed judgment,
  routing, logging, and usage metering.
- Own the model spec contract: `ModelSpec`, `ProviderId`, `Effort`,
  `EffortSet`, the capability data types, `RoutingTiers`, `ModelCatalog`,
  and the spec of the built-in mock model. This crate defines only the
  `mock` provider id. Provider crates define their own provider ids and
  specs.
- Represent provider-owned replay state, including Anthropic, Google,
  DeepSeek, Kimi, MiniMax, and Qwen reasoning context that remains separate
  from visible assistant text.
- Own the model-visible tool output DTOs used by universal output management:
  opaque output ids, inline envelopes, window envelopes, read requests, and
  unavailable-remainder reasons.
- Own the generic `Model`, `Tool`, `ModelRouter`, `AudioTranscriber`,
  `ImageGenerator`, `VideoGenerator`, `JudgmentModel`, and `Logger` trait
  boundaries plus their dyn aliases.
- Define ordered assistant, reasoning, and fallback-restart completion events,
  an async event sink, and a default-compatible opt-in model entrypoint.
- Keep individual tools pagination-agnostic: `Tool::call()` and
  `Tool::call_with_invocation()` return raw `serde_json::Value`.
- Define logger success payloads in terms of bounded model-visible envelopes,
  leaving any full raw-output capture to the runtime that executed the tool.
- Provide built-in mocks for deterministic development and tests.

## What This Crate Does

- Defines `ConversationMessage`, `ConversationRole`, `ToolCall`, and
  `ToolDefinition`, including provider replay context on assistant messages.
  User messages carry ordered typed content parts: text, base64 images, and
  base64 videos. `ModelFeature::Vision` and `ModelFeature::VideoInput` are the
  stable routing capabilities for image and video inputs; providers without a
  video wire contract reject video parts with a typed provider error.
  DeepSeek replay items retain normalized assistant content, optional provider
  reasoning, and ordered raw tool calls so a later tool continuation can
  reproduce the original assistant message exactly. Kimi and Qwen have
  equivalent provider-owned replay shapes with nullable content.
- Defines `AnthropicAssistantMessage` replay items with the complete ordered
  text, thinking, redacted-thinking, and tool-use blocks from signed responses.
  `GoogleAssistantMessage` keeps every original model part when any part has a
  thought signature, including thought flags and optional raw function-call ids.
  Their serde tags are `anthropic_assistant_message` and
  `google_assistant_message`. Each adapter replays only its own context.
- Defines typed MiniMax reasoning details for lossless continuation replay
  and typed Qwen raw tool-call replay state.
- Defines `ProviderId`, the stable lowercase name of a provider, such as
  `deepseek`, `minimax`, or `qwen`. It serializes as that plain string. Each
  provider crate exports the constant of its provider. This crate defines
  only `MOCK_PROVIDER` (`mock`).
- Defines `ModelSpec`, the facts of one provider model: the provider id, the
  provider model id, the context window, the max output, whether every
  request needs an output limit, the supported efforts as an `EffortSet`
  that can be empty, the optional default effort, the `ForcedToolChoice`
  and `Sampling` rules, stop-sequence and deferred-lifecycle support, the
  efforts whose tool continuations need replay state, the routing features,
  and optional `RoutingTiers` (intelligence, speed, and cost). Every field
  is public. `text_id()` returns the `provider/model` text id. A
  `default_effort` of `None` means that the provider chooses the reasoning.
- Defines `CustomModelDefaults` and
  `ModelSpec::custom(provider, model_id, &defaults)` for a model that no
  catalog lists. Each provider crate exports the defaults of its provider. A
  custom spec starts with no features and no routing tiers.
- Defines `ModelCatalog`. `ModelCatalog::new().with_models(...)` combines the
  `CATALOG` slices of several providers in insertion order. `find(text_id)`
  splits the text id at its first slash and matches the provider id and the
  provider model id exactly, with case.
- Defines `Effort`, ordered from `Disabled` through `Low`, `Medium`, `High`,
  `ExtraHigh`, and `Max`. `Disabled` turns thinking off with an explicit
  provider off value. A request sets it in
  `ModelGenerationControls::effort`. A request without one uses the default
  effort of the spec. A spec without a default effort leaves the reasoning
  to the provider. The shared capability check in `ai-models-core`
  resolves it against the spec.
- Defines `MOCK_MODEL`, `MOCK_CATALOG`, and `MOCK_CUSTOM_DEFAULTS` for local
  development and tests.
- Defines `ModelRequest`, `ModelResponse`, `FinishReason`,
  `StructuredOutputSchema`, model usage DTOs, and typed model/router errors.
  `ModelResponse::effort()` returns the resolved effort, or `None` when the
  provider chose the reasoning. The typed `thinking_level` field holds it
  and serializes as `effort`.
  `catalog_model_id` records the text id of the spec.
  `ModelError::Interrupted` identifies a streaming completion that failed only
  after provider events had arrived, keeping it distinct from a safe-to-retry
  `TransientProvider` failure before any progress.
- Defines `ModelRouteRequest`, `ModelRequirement`, `ModelPreference`, and the
  `ModelRouter` trait. `ModelRequirement::ModelId` holds a text id, and
  `ModelRequirement::Provider` holds a `ProviderId`.
- Defines `ModelCompletionEvent` and `ModelCompletionEventSink` for callers
  that opt into `Model::complete_with_events`. The default method delegates to
  `complete` without events, while `NoopModelCompletionEventSink` lets concrete
  adapters share the observing path with buffered callers.
- Defines defaulted `ModelCallControls` for the request effort, portable
  sampling, output limits, ordered stops, strict or explicit
  required-with-automatic-fallback tool choice, total call timeout, and
  provider-neutral synchronous/prefer-deferred/require-deferred lifecycle
  intent. Explicit intent that cannot be honored returns a typed
  `UnsupportedControl` error that names the `ModelControl`, including
  `ModelControl::Effort`.
- Defines the one-image `ImageGenerator` boundary, normalized aspect/quality
  controls, edit input images, decoded output bytes, and typed policy/retry
  errors. Internal failures use the shared tracked `InternalError` contract.
  `ModelFeature::ImageGeneration` is the stable routing capability.
- Defines the one-video `VideoGenerator` boundary for text-to-video and one
  optional first-frame image, portable landscape/portrait 720p and four/eight
  second controls, downloaded MP4 bytes with metadata, and typed policy,
  timeout, and retry errors. `ModelFeature::VideoGeneration` is the stable
  routing capability.
- Defines the `JudgmentModel` boundary for evaluating text or JSON state
  against typed condition, choice, and score questions in one call. Local
  validation rejects empty state and malformed question maps before provider
  access. Score rubrics require a description for every ordered level, while
  condition and choice descriptions remain optional.
  `JudgmentResponse::validate_against` checks successful answer kinds,
  option and level membership, complete probability distributions, bounded
  numeric values, distribution sums, and internal consistency (the selected
  option is the most probable one and the expected score matches its
  distribution within documented tolerances) before callers consume them.
  The `typesafe` provider id and `ModelFeature::Judgment` are the stable
  routing identities.
- Defines `ToolInvocation`, which carries the runtime operation id used as a
  tool idempotency key alongside the model-visible tool name and JSON input.
- Defines `ToolOutputEnvelope` as the model-visible success payload for tools.
  Complete small outputs serialize as tagged `tool_output` envelopes. Larger or
  degraded outputs serialize as tagged `tool_output_window` envelopes with
  byte offsets, UTF-8 content, total byte counts, and either a next byte offset
  or an unavailable-remainder reason.
- Defines `ToolOutputId` as an opaque string newtype. Callers may store and
  echo the value, but must not parse or derive meaning from `toolout_...`
  strings.
- Defines `ToolOutputReadRequest`, the argument DTO used by runtimes that expose
  the intrinsic `tool_output_read` tool.
- Defines `TurnOutcomeLogEntry::outcome` with `TurnOutcome::Completed`,
  `Truncated`, and `Capped` so turn logs retain the reason a turn ended.
- Defines `ToolCallLogResult::Success` with a `ToolOutputEnvelope` payload.
  Normal logs and conversation state carry only bounded model-visible output.

The raw versus model-visible boundary is intentional. A `Tool` implementation
returns the raw JSON value it produced. Runtime crates can inspect that value in
current-run execution records, but they serialize only `ToolOutputEnvelope`
values into conversation tool messages and logger success entries.

Provider replay context follows the same separation. Anthropic, Google,
DeepSeek, Kimi, MiniMax, and Qwen reasoning content is retained as provider-owned context for
continuation requests, but it is never copied into normalized assistant text
or exposed as tool output. Other model providers ignore replay items they do
not own.

## Quick Start

```rust
use ai_interface::{
    ConversationMessage, Effort, MOCK_CATALOG, MOCK_CUSTOM_DEFAULTS, MOCK_PROVIDER, MockModel,
    Model, ModelCatalog, ModelRequest, ModelSpec,
};

fn find_mock_spec() -> Option<ModelSpec> {
    ModelCatalog::new()
        .with_models(MOCK_CATALOG.iter().cloned())
        .find("mock/mock")
        .cloned()
}

fn unlisted_mock_spec() -> ModelSpec {
    let mut spec = ModelSpec::custom(MOCK_PROVIDER, "mock-large", &MOCK_CUSTOM_DEFAULTS);
    spec.context_window_tokens = 1_000_000;
    spec
}

fn high_effort_request() -> ModelRequest {
    let mut request = ModelRequest {
        system_prompt: "You are concise.".to_owned(),
        messages: vec![ConversationMessage::user("Plan the release")],
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.effort = Some(Effort::High);
    request
}

async fn call_mock_model() -> ai_interface::ModelResult<String> {
    let response = MockModel::new("done").complete(&high_effort_request()).await?;
    Ok(response.assistant_message)
}
```

[`docs/examples.md`](docs/examples.md) has more examples: model calls with
completion events, image and video generation, judgment, tool output
envelopes, and provider replay context.

## Development

```sh
cargo test -p ai-interface
cargo clippy -p ai-interface --all-targets --all-features -- -D warnings
```

The crate is intentionally dependency-light. `serde` and `serde_json` are used
for contract DTOs, while runtime policy, storage, pagination, and intrinsic
tool dispatch live in `ai-tool-calling`.

### Key Code

- `src/messages/` - conversation DTOs, typed Anthropic blocks and Google parts,
  DeepSeek/Kimi/Qwen raw tool replay, MiniMax reasoning details, and provider
  replay context.
- `src/model.rs` - model trait, request/response DTOs, finish reasons, and
  typed model errors.
- `src/model_completion_events.rs` - public incremental events, sink trait,
  dynamic alias, and no-op sink.
- `src/model_controls.rs` - portable generation and execution controls,
  including the request effort.
- `src/model_spec.rs` - `ModelSpec`, its text id, and `ModelSpec::custom`.
- `src/model_capabilities.rs` - `ForcedToolChoice`, `Sampling`, and
  `CustomModelDefaults`.
- `src/effort.rs` - `Effort` and `EffortSet`, which can be empty.
- `src/provider_id.rs` - `ProviderId` and `MOCK_PROVIDER`.
- `src/model_catalog.rs` - `ModelCatalog` and its case-sensitive text-id
  lookup.
- `src/routing_tiers.rs` - intelligence, speed, and cost tiers.
- `src/mock_catalog.rs` - the mock model spec, catalog, and custom
  defaults.
- `src/router.rs` - model route request DTOs and router trait.
- `src/audio_transcriber.rs` - speech-to-text trait, request/response DTOs,
  and typed transcription errors.
- `src/image_generator.rs` - one-image generation/editing trait, normalized
  DTOs, and typed generation errors.
- `src/video_generator.rs` - one-video generation trait, portable controls,
  downloaded MP4 response DTO, and typed lifecycle errors.
- `src/judgment/` - judgment content, questions, answers, validation, errors,
  response DTOs, and the provider-agnostic trait.
- `src/tools.rs` - tool trait, tool DTOs, invocation context, and tool errors.
- `src/output/` - model-visible tool output ids, envelopes, reasons, and read
  request DTOs.
- `src/logger.rs` - logger trait, log payloads, `ToolCallLogResult`, and
  `NoopLogger`.
- `src/mock_model.rs`, `src/mock_audio_transcriber.rs`,
  `src/mock_image_generator.rs`, `src/mock_video_generator.rs`, and
  `src/mock_judgment_model.rs` - built-in mocks for tests and local
  development.

### Related Docs

- [`docs/examples.md`](docs/examples.md)
- [`../../docs/architecture.md`](../../docs/architecture.md)
- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`futex-ai/internal-error`](https://github.com/futex-ai/internal-error)
- [`../ai-tool-calling/README.md`](../ai-tool-calling/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../docs/protocol/minimax-model-provider.md`](../../docs/protocol/minimax-model-provider.md)
- [`../../docs/protocol/image-generation.md`](../../docs/protocol/image-generation.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
- [`../../docs/protocol/judgment-boundary.md`](../../docs/protocol/judgment-boundary.md)
- [`../../docs/protocol/typesafe-judgment-provider.md`](../../docs/protocol/typesafe-judgment-provider.md)
- [`../../docs/protocol/live-judgment-api-tests.md`](../../docs/protocol/live-judgment-api-tests.md)
- [`../../docs/protocol/tool-output-management.md`](../../docs/protocol/tool-output-management.md)
- [`../../docs/protocol/kimi-model-provider.md`](../../docs/protocol/kimi-model-provider.md)
- [`../../docs/protocol/deepseek-model-provider.md`](../../docs/protocol/deepseek-model-provider.md)
- [`../../docs/protocol/qwen-model-provider.md`](../../docs/protocol/qwen-model-provider.md)
