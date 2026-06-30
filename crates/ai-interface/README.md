# ai-interface

`ai-interface` is the shared contract crate for AI-facing runtime boundaries.
Depend on it when you need the common conversation DTOs, tool, model, audio
transcription, logging hooks, or built-in mocks without pulling in the stateful
tool-calling runtime from `ai-tool-calling`.

## Responsibilities

- Own shared conversation, tool, model, audio transcription, and logger DTOs
- Own the generic `Tool`, `Model`, `ModelRouter`, `AudioTranscriber`, and
  `Logger` trait boundaries
- Provide the typed model and tool error contracts used across AI crates
- Carry public-safe tool activity metadata and lifecycle log DTOs without
  making that metadata model-visible
- Provide built-in mock model and audio transcriber implementations for
  development and tests

## What This Crate Does

- Defines `ConversationMessage`, `ConversationRole`, `ToolCall`, and
  `ToolDefinition`
- Defines provider replay context carried on assistant messages and model
  responses so provider adapters can preserve opaque state needed by stateless
  APIs without making it part of user-visible assistant text
- Defines `ConversationContentPart::Image` for provider request payloads; image
  parts are model-call input, not a tool-result transport or durable storage
  contract
- Defines `ModelRequest`, `ModelResponse`, `ModelUsage`, `FinishReason`,
  `StructuredOutputSchema`, route request DTOs, and typed model/router errors
- Carries normalized model usage categories and priced usage lines for
  metering without making provider crates own pricing policy
- Defines the generic `Tool`, `Model`, `ModelRouter`, `AudioTranscriber`, and
  `Logger` traits plus dyn aliases
- Defines `ToolInvocation`, the generic tool-dispatch context that carries the
  runtime operation id/idempotency key alongside the model-visible tool name
  and JSON input
- Lets tools attach an optional one-word `activity_verb`; provider adapters
  skip that field when serializing model-visible tool definitions
- Lets tool adapters report the runtime tool group that exposed a tool so
  logging and persistence can label dynamic app tools without hard-coded names
- Defines `ToolActivityLogEntry` so runtimes can report public tool activity
  separately from full tool-call input/output logs
- Exposes `NoopLogger`, a simple `MockModel`, and `MockAudioTranscriber` for
  deterministic local development and tests

## Quick Start

```rust
use ai_interface::{ConversationMessage, Model, ModelRequest, MockModel};

async fn demo() -> ai_interface::ModelResult<String> {
    let model = MockModel::new("mock");
    let response = model
        .complete(&ModelRequest {
            system_prompt: "You are concise.".to_owned(),
            messages: vec![ConversationMessage::user("Summarize the build failure")],
            tools: Vec::new(),
            response_schema: None,
        })
        .await?;
    Ok(response.assistant_message)
}
```

Server-side speech-to-text callers use `AudioTranscriber`:

```rust
use ai_interface::{AudioTranscriber, AudioTranscriptionRequest, MockAudioTranscriber};

async fn transcribe_demo() -> ai_interface::TranscriptionResult<String> {
    let transcriber = MockAudioTranscriber::new("ship it");
    let response = transcriber
        .transcribe(&AudioTranscriptionRequest {
            audio: b"audio".to_vec(),
            filename: "voice.m4a".to_owned(),
            content_type: "audio/mp4".to_owned(),
        })
        .await?;
    Ok(response.text)
}
```

Structured responses are optional at the shared boundary. Callers can attach a
`StructuredOutputSchema` to `ModelRequest::response_schema` and read validated
JSON back from `ModelResponse::structured_output` when the provider returns a
normal `Stop` response. Provider adapters preserve non-success finish reasons
such as `Filtered` or `Truncated` without attempting schema validation.

`ModelResponse::finish_reason` carries the provider stop signal normalized to
`FinishReason`: `Stop` for natural completion, `ToolCalls` for model-requested
tool dispatch, `Truncated` for token or context limits, `Filtered` for
provider policy filtering/refusal, and `Other(raw)` for unrecognized provider
values.
`ModelResponse::model_id` is the concrete provider model id used for the
upstream call. Provider adapters may also populate
`ModelResponse::catalog_model_id` and `ModelResponse::thinking_level` so
runtime logs can distinguish catalog variants that target the same provider
model with different thinking settings.
`ModelUsage` includes provider-normalized, non-overlapping input, output, cached
input, and reasoning token counts. Runtime pricing wrappers can attach
`ModelUsageCostLine` rows with unit kind, quantity, price snapshot, rate
version, measurement state, and micro-USD cost for the usage metering ledger.
Provider request failures that clearly indicate the input exceeded the model
context window use `ModelError::ContextLimitExceeded` so callers can compact
retained history and retry safely.

Provider adapters can populate `ModelResponse::provider_context` with opaque
or provider-specific replay items. Conversation runtimes should retain those
items on the corresponding assistant message and pass them back in later model
requests; callers should not render them as assistant text or tool output.
OpenAI Responses adapters use this context for reasoning items and raw
function-call items whose provider item ids and original argument strings must
be replayed exactly during stateless tool-calling continuations.

Model routing is expressed separately from `ModelRequest`. Callers ask a
`ModelRouter` to resolve a `ModelRouteRequest` containing hard requirements
and ordered preferences; the resolved `DynModel` then executes normal model
requests. `ModelRouteRequest::default()` means deployment-priority ordering.
Multimodal callers should populate `ConversationMessage::content_parts` with
text and image parts only for the model call that needs pixels. Runtime crates
remain responsible for redacting or stripping those image parts before
persisting public conversation history, logs, or summaries.
`ModelRequirement::ModelId` pins routing to a configured catalog model id while
still letting the router apply credential validation and provider construction.

## Development

```sh
cargo test -p ai-interface
cargo clippy -p ai-interface --all-targets --all-features -- -D warnings
```

### Key Code

- `src/messages.rs` - conversation DTOs and convenience constructors
- `src/model.rs` - model trait, request/response DTOs, and typed model errors
- `src/router.rs` - model route request DTOs and router trait
- `src/audio_transcriber.rs` - speech-to-text trait, request/response DTOs,
  and typed transcription errors
- `src/tools.rs` - tool trait, tool DTOs, and tool errors
- `src/logger.rs` - logger trait, log payloads, and `NoopLogger`
- `src/mock_model.rs` - built-in mock model for development and tests
- `src/mock_audio_transcriber.rs` - built-in mock transcriber for development
  and tests

### Related Docs

- [`../ai-tool-calling/README.md`](../ai-tool-calling/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
