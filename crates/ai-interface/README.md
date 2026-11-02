# ai-interface

`ai-interface` is the shared contract crate for AI-facing runtime boundaries in
this workspace. Depend on it when you need the common conversation, model,
model event stream, model spec, effort, tool, audio transcription, image
generation, video generation, judgment, routing, logging, usage, or
model-visible tool output DTOs without taking a dependency on a stateful
runtime implementation.

## Responsibilities

- Own shared DTOs for conversations, model calls, tool calls, audio
  transcription, image generation/editing, video generation, typed judgment,
  routing, logging, and usage metering.
- Own the model spec contract: `ModelSpec`, `ProviderId`, `Effort`,
  `EffortSet`, the capability data types, `RoutingTiers`, `ModelCatalog`,
  and the spec of the built-in mock model. This crate defines only the
  `mock` provider id. Provider crates define their own provider ids and
  specs.
- Own conversation messages as ordered parts, and the opaque
  `ProviderState` that an adapter attaches to the parts that it produces.
  Only the adapter that wrote a state interprets its payload.
- Own the stored message format, its reader, and the provider-neutral
  composition of the version 1 converters that provider crates export.
- Own the model-visible tool output DTOs used by universal output management:
  opaque output ids, inline envelopes, window envelopes, read requests, and
  unavailable-remainder reasons.
- Own the generic `Model`, `Tool`, `ModelRouter`, `AudioTranscriber`,
  `ImageGenerator`, `VideoGenerator`, `JudgmentModel`, and `Logger` trait
  boundaries plus their dyn aliases.
- Own the one model entry point: `Model::stream`, its ordered `ModelEvent`
  values, and the shared fold that the provided `Model::complete` and other
  stream consumers use.
- Keep individual tools pagination-agnostic: `Tool::call()` and
  `Tool::call_with_invocation()` return raw `serde_json::Value`.
- Define logger success payloads in terms of bounded model-visible envelopes,
  leaving any full raw-output capture to the runtime that executed the tool.
- Provide built-in mocks for deterministic development and tests.

## What This Crate Does

- Defines `ConversationMessage`, `ConversationRole`, `ToolCall`, and
  `ToolDefinition`. A message has a role, ordered `parts`, and an optional
  participant `name`. `ConversationPart` has the `Text`, `Image`, `Video`,
  `ToolCall`, `ToolResult`, and `Reasoning` variants, and each variant has an
  optional `provider_state`. `ConversationPart::text`, `image`, `video`,
  `reasoning`, `tool_result`, and `from_tool_call` build parts without state.
  `ConversationMessage::new`, `user`, `user_with_parts`, `assistant`, and
  `tool` build messages. `text()` joins the text and tool-result content, and
  `tool_calls()` returns the portable calls, both in part order.
- Checks the role of each message before transport: a user message holds
  text, image, and video parts, an assistant message holds text, reasoning,
  and tool-call parts, and a tool message holds exactly one tool result.
  `ModelRequest::validate_messages(provider, model_id)` checks every message
  and fails with `ModelError::Provider`. `ModelFeature::Vision` and
  `ModelFeature::VideoInput` are the stable routing capabilities for image
  and video inputs; providers without a video wire contract reject video
  parts with a typed provider error.
- Defines `ProviderState { provider, model, version, payload }`. The payload
  is a JSON object that only the adapter of `provider` reads, and each
  adapter keeps its payload types private.
  `ProviderState::decode::<T>(provider, model_id, version)` returns `None` for
  the state of another provider, and fails with `ModelError::Provider` for an
  unsupported version or a payload that does not decode.
- Serializes `ConversationMessage` as stored format version 2:
  `format_version`, `role`, `parts`, and an optional `name`.
  `read_stored_message(value, &converters)` reads versions 1 and 2, and
  returns `StoredMessageError::UnsupportedFormatVersion` for another version.
  `LegacyMessageConverter::new(converters)` composes the
  `LegacyProviderConverter` of each provider crate, and its `read` and
  `convert` methods apply them. Plain `serde` deserialization reads version 1
  without a provider converter, so every old item stays whole as the
  provider state of its provider, in the generic part order. Use a composed
  converter, which supplies the conversion and the part order of each
  provider.
- Defines the version 1 input types without provider names:
  `LegacyConversationMessage`, and `LegacyProviderItem` with the original
  `type` tag and fields of an old item. A `LegacyProviderConverter` gives its
  `provider()` and its `order()`, `LegacyPartOrder::ReasoningFirst` by default
  or `Context`. Its `convert_item` returns `LegacyPartAttachment` values with a
  `LegacyPartTarget`, and its optional `convert_message` rebuilds the parts of
  a message whose item holds the exact block order.
  `compose_legacy_message` applies a list of converters.
- Defines `ProviderId`, the stable lowercase name of a provider, such as
  `deepseek`, `minimax`, or `qwen`. It serializes as that plain string. Each
  provider crate exports the constant of its provider. This crate defines
  only `MOCK_PROVIDER` (`mock`).
- Defines `ModelSpec`, the facts of one provider model: the provider id, the
  provider model id, the context window, the max output, whether every
  request needs an output limit, the supported efforts as a nonempty
  `EffortSet`, the default effort, the `ForcedToolChoice` and `Sampling`
  rules, stop-sequence and deferred-lifecycle support, the efforts whose tool
  continuations need replay state, the routing features, and optional
  `RoutingTiers` (intelligence, speed, and cost). Every field is public.
  `text_id()` returns the `provider/model` text id.
- Defines `CustomModelDefaults` and
  `ModelSpec::custom(provider, model_id, &defaults)` for a model that no
  catalog lists. Each provider crate exports the defaults of its provider. A
  custom spec starts with no features and no routing tiers.
- Defines `ModelCatalog`. `ModelCatalog::new().with_models(...)` combines the
  `CATALOG` slices of several providers in insertion order. `find(text_id)`
  splits the text id at its first slash and matches the provider id and the
  provider model id exactly, with case.
- Defines `Effort`, ordered from `Disabled` through `Low`, `Medium`, `High`,
  `ExtraHigh`, and `Max`. A request sets it in
  `ModelGenerationControls::effort`. A request without one uses the default
  effort of the spec. The shared capability check in `ai-models-core`
  resolves it against the spec.
- Defines `MOCK_MODEL`, `MOCK_CATALOG`, and `MOCK_CUSTOM_DEFAULTS` for local
  development and tests.
- Defines `ModelRequest`, `ModelResponse`, `FinishReason`,
  `StructuredOutputSchema`, model usage DTOs, and typed model/router errors.
  `ModelResponse::parts` holds the assistant parts in generated order.
  `text()` joins its text parts, `tool_calls()` returns its portable calls,
  and `to_assistant_message()` copies every part with its state into an
  assistant message. `log_copy()` on a request, a message, a part, or a
  response keeps every message and part, drops every provider state, and
  empties the reasoning text. `ModelResponse::effort()` returns the resolved
  effort. The typed `thinking_level` field holds it and serializes as
  `effort`.
  `catalog_model_id` records the text id of the spec.
  `ModelResponse::attempts` lists the failed fallback attempts before the
  response as `ModelAttempt` records. JSON leaves out an empty list.
  `ModelError::Interrupted` identifies a model stream that failed only after
  provider events had arrived, keeping it distinct from a safe-to-retry
  `TransientProvider` failure before any progress.
- Defines `ModelRouteRequest`, `ModelRequirement`, `ModelPreference`, and the
  `ModelRouter` trait. `ModelRequirement::ModelId` holds a text id, and
  `ModelRequirement::Provider` holds a `ProviderId`.
- Defines `Model::stream`, the one method that a model implements. It
  returns an error or a `DynModelEventStream`, a boxed `ModelEventStream`.
  Its async `next` returns the next `ModelEvent`, `None` after the finish
  event, or an error. After `None` or an error, every later call returns
  `None`. Dropping the stream cancels the call.
- Defines `ModelEvent`: `TextDelta`, `ReasoningDelta`, `ToolCallStart`,
  `ToolCallArgumentsDelta`, `ToolCallEnd`, `Usage`, `Finish`, and
  `Fallback`. Content events carry a zero-based `part_index`.
  `ToolCallStart` carries the call id, the tool name, and the optional
  runtime operation id. `TextDelta`, `ReasoningDelta`, and `ToolCallEnd`
  carry an optional `provider_state` for their part. A text or reasoning
  delta has empty text only when it carries state. `Finish` carries the
  finish reason, the provider, the provider model id, the text id, the
  resolved effort, and the structured output. Only a policy layer emits
  `Fallback`. It carries the error of a failed attempt, and a consumer
  discards every event of that attempt.
- Defines `ModelResponseFold` with `new`, `push`, and `finish`. The provided
  `Model::complete` folds every event of `stream` with it, and other stream
  consumers can use the same fold. The fold builds one part for each part
  position into `ModelResponse::parts`, in part position order. It keeps
  reasoning parts, keeps the tool-call parts only for the `ToolCalls` finish
  reason, and gives each part the last provider state that its events
  carried. It takes the usage and the finish fields from their events. A
  `Fallback` event removes the content of the failed attempt and adds a
  `ModelAttempt` to `attempts`. Events that break the order of the
  [model stream protocol](../../docs/protocol/model-stream.md) fail the fold
  with `ModelError::Internal`.
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

Provider state follows the same separation. Reasoning stays in reasoning
parts and never joins the response text or tool output. An adapter replays
its own state and ignores the state of every other provider. Log copies keep
every part but drop every provider state and every reasoning text, for every
provider.

## Quick Start

```rust
use std::collections::BTreeMap;

use ai_interface::{
    ConversationMessage, Effort, MOCK_CATALOG, MOCK_CUSTOM_DEFAULTS, MOCK_PROVIDER, MockModel,
    Model, ModelCatalog, ModelEvent, ModelRequest, ModelSpec,
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
    Ok(response.text())
}

async fn stream_mock_text() -> ai_interface::ModelResult<String> {
    let mut stream = MockModel::new("done").stream(&high_effort_request()).await?;
    // Parts can interleave, so collect each text part by its position.
    let mut text_parts = BTreeMap::<usize, String>::new();
    while let Some(event) = stream.next().await? {
        match event {
            ModelEvent::TextDelta { part_index, delta, .. } => {
                text_parts.entry(part_index).or_default().push_str(&delta);
            }
            ModelEvent::Fallback { .. } => text_parts.clear(),
            _ => {}
        }
    }
    Ok(text_parts.into_values().collect())
}
```

[`docs/examples.md`](docs/examples.md) has more examples: model calls and
model events, test models, image and video generation, judgment, tool output
envelopes, conversation parts, log copies, and stored messages.

## Development

```sh
cargo test -p ai-interface
cargo clippy -p ai-interface --all-targets --all-features -- -D warnings
```

The crate is intentionally dependency-light. `serde` and `serde_json` are used
for contract DTOs, while runtime policy, storage, pagination, and intrinsic
tool dispatch live in `ai-tool-calling`.

Downstream tests can enable the `test-support` feature for the `unimock`
mocks, such as `ModelMock`, `ModelEventStreamMock`, and
`LegacyProviderConverterMock`, and for the finite streams of
`test_support`. `model_event_stream` boxes a list of events.
`model_response_stream` turns a response fixture into the events that fold
back into it.

### Key Code

- `src/messages/` - `ConversationMessage`, `ConversationPart` and its
  helpers, `ProviderState`, the role check, log copies, the stored format
  reader, and the provider-neutral version 1 converter types and
  composition.
- `src/model.rs` - the `Model` trait with its provided `complete`,
  request/response DTOs, and finish reasons.
- `src/model_helpers.rs` - `ModelRequest::validate_messages` and `log_copy`,
  and the part accessors and `log_copy` of `ModelResponse`.
- `src/model_event.rs` - `ModelEvent`, the ordered events of a model stream.
- `src/model_stream.rs` - the `ModelEventStream` trait and
  `DynModelEventStream`.
- `src/model_response_fold/` - `ModelResponseFold`, the shared fold from
  events to a `ModelResponse`, and its event-order errors.
- `src/model_attempt.rs` - `ModelAttempt`, the serializable record of a
  failed fallback attempt.
- `src/model_error.rs` - `ModelError` and its constructors.
- `src/test_support.rs` - finite event streams for tests, behind
  `test-support`.
- `src/model_controls.rs` - portable generation and execution controls,
  including the request effort.
- `src/model_spec.rs` - `ModelSpec`, its text id, and `ModelSpec::custom`.
- `src/model_capabilities.rs` - `ForcedToolChoice`, `Sampling`, and
  `CustomModelDefaults`.
- `src/effort.rs` - `Effort` and the nonempty `EffortSet`.
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
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/conversation-parts.md`](../../docs/protocol/conversation-parts.md)
- [`../../docs/protocol/video-input.md`](../../docs/protocol/video-input.md)
- [`../../docs/protocol/model-policies.md`](../../docs/protocol/model-policies.md)
- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
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
