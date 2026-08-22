# ai-models-openai

`ai-models-openai` is the OpenAI-specific `ai-interface` implementation crate
for the workspace. Depend on it when you want to call OpenAI Responses
generation models, image or video generation, image editing, or the audio
transcription endpoint with explicit auth and shared runtime wrappers from
neighboring crates.

## Responsibilities

- Implement the OpenAI model client behind `ai_interface::Model`
- Implement the OpenAI speech-to-text client behind
  `ai_interface::AudioTranscriber`
- Implement the OpenAI image client behind `ai_interface::ImageGenerator`
- Implement the OpenAI video client behind `ai_interface::VideoGenerator`
- Export strongly typed known OpenAI model metadata for model routing
- Map shared model/tool DTOs to the OpenAI Responses API
- Consume OpenAI Responses SSE events and retain the complete terminal response
- Map shared audio transcription DTOs to `v1/audio/transcriptions`
- Map shared image DTOs to `v1/images/generations` and `v1/images/edits`
- Map shared video DTOs to asynchronous `v1/videos` jobs and downloads
- Parse OpenAI responses into shared response DTOs and typed model errors

## What This Crate Does

`OpenAiModel` accepts a `json-http` client plus explicit auth input and handles:

- OpenAI Responses request serialization with internal SSE consumption
- 3,600-second default overall completion deadline, 120-second stream idle
  deadline, and explicit per-call total-timeout overrides
- shared text and image parts as Responses input content, with typed rejection
  of shared video parts before transport
- portable temperature and top-p outside reasoning mode, output-limit and
  strict or required-with-automatic-fallback tool-choice mapping, per-call
  timeouts, blank-instruction omission, and typed rejection of unsupported
  stop sequences
- OpenAI tool-call parsing
- OpenAI `finish_reason` normalization into `ai_interface::FinishReason`
  before exposing tool calls, so incomplete or failed Responses results cannot
  dispatch partial function-call output items
- OpenAI `text.format` JSON-schema mapping for structured outputs, using
  non-strict mode so callers can pass the broader shared schema contract
- OpenAI `reasoning.effort` mapping from catalog `ThinkingLevel` for
  reasoning-capable catalog variants
- catalog-aware downgrade to the highest configured effort not above an
  unsupported request, with the effective effort retained in responses
- stateless `store: false` generation calls using caller-owned conversation
  state
- `reasoning.encrypted_content` inclusion for reasoning models so encrypted
  reasoning items can be replayed across stateless tool-calling turns
- raw Responses function-call item retention so stateless tool continuations
  replay OpenAI's provider item id and original argument string instead of a
  normalized JSON render
- assistant message `phase` retention so stateless replay preserves tool
  preambles and final-answer phase metadata in the original output-item order
- phase-less assistant message markers when function-call replay context needs
  the original OpenAI output item order
- isolation from replay context owned by other providers, including DeepSeek,
  Kimi, and MiniMax assistant reasoning state and raw tool calls
- provider response usage extraction into normalized input, output, cached
  input, and reasoning token counts
- terminal `response.completed` and `response.incomplete` objects routed
  through the buffered response parser, preserving truncated partial results
- typed classification of completion failures before stream progress as
  retryable or provider errors and failures after progress as
  `ModelError::Interrupted`
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`, with retryable `408`, `409`, `425`, and `5xx`
  transcription statuses mapped to transient provider failures

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports `known_models()` and typed catalog id
constants for GPT-5.6 Sol, Terra, and Luna. `GPT_5_6_SOL` is the first catalog
entry and the default used by workspace examples. Sol also has explicit low,
high, extra-high, and max-thinking variants; each sends provider model id
`gpt-5.6-sol`. All existing GPT-5.5 constants and catalog entries remain
available for pinned deployments. Cost-optimized routing uses GPT-5.4 Mini and
Nano because OpenAI does not expose GPT-5.5 Mini or Nano aliases.
OpenAI generation uses workspace-defined function tools with `strict: false` during
the Responses cutover. OpenAI built-in tools are intentionally not exposed by
this crate.
When OpenAI returns Responses assistant message `phase`, `reasoning`, or
`function_call` output items, this crate stores those replay-sensitive items in
`ModelResponse::provider_context`; runtimes should keep that context on the
assistant message so later OpenAI requests can replay the phased assistant
message and provider items before the associated function-call outputs. The
normalized `ToolCall` list remains the tool-dispatch contract, but the raw
provider context is preferred for OpenAI request replay when present.

`OpenAiAudioTranscriber` submits completed audio recordings to the OpenAI
transcription endpoint using `gpt-4o-mini-transcribe` or another caller-chosen
transcription model. It expects the caller to provide the API key and the
uploaded audio media type. It applies a 60-second request timeout and surfaces
retryable OpenAI transcription statuses as transient errors.

`OpenAiImageGenerator` accepts an injected `json-http` client and explicit
model id; `GPT_IMAGE_2` identifies the current image catalog entry. Requests
without source images use the JSON generation endpoint; requests with source
images use multipart edit requests. The injected client makes auth,
serialization, timeout, transport, and status behavior credential-free to
test. The adapter rejects empty decoded payloads, preserves any revised prompt,
normalizes usage, and keeps content-policy refusals distinct from retryable
provider failures.

`OpenAiVideoGenerator` submits text-to-video JSON or first-frame multipart
requests to `v1/videos`, polls queued jobs through `GET /v1/videos/{id}`, then
downloads the completed MP4 through the authenticated binary transport path.
`SORA_2` identifies the 720p video catalog entry. The adapter supports portable
landscape/portrait and four/eight-second controls, enforces one ten-minute total
deadline with an injected polling runtime, and keeps policy, timeout, rate
limit, transient, and terminal failures typed. This upstream API is currently
scheduled to shut down on September 24, 2026, so consumers should treat the
adapter as a time-bounded compatibility integration.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{AudioTranscriber, ImageGenerator, Model, VideoGenerator};
use ai_models_openai::{
    GPT_5_6_SOL, GPT_IMAGE_2, SORA_2, OpenAiAudioTranscriber, OpenAiImageGenerator,
    OpenAiModel, OpenAiVideoGenerator, known_models,
};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> OpenAiModel {
    OpenAiModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GPT_5_6_SOL,
        "sk-demo",
    )
}

fn known_model_count() -> usize {
    known_models().len()
}

fn build_transcriber() -> OpenAiAudioTranscriber {
    OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "sk-demo")
}

fn build_image_generator() -> OpenAiImageGenerator {
    OpenAiImageGenerator::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GPT_IMAGE_2,
        "sk-demo",
    )
}

fn build_video_generator() -> OpenAiVideoGenerator {
    OpenAiVideoGenerator::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        SORA_2,
        "sk-demo",
    )
}
```

## Development

```sh
cargo test -p ai-models-openai
cargo clippy -p ai-models-openai --all-targets --all-features -- -D warnings
```

### Key Code

- `src/openai/mod.rs` - `Model` implementation and request dispatch
- `src/catalog.rs` - known OpenAI model ids and routing metadata
- `src/openai/request.rs` - OpenAI Responses request mapping
- `src/openai/request_types.rs` - OpenAI Responses request DTOs
- `src/openai/response/mod.rs` - OpenAI Responses response parsing
- `src/openai/stream/` - terminal event handling and stream failure mapping
- `src/openai/transcription.rs` - OpenAI audio transcription implementation
- `src/openai/image_generation/` - OpenAI image request, response, and error mapping
- `src/openai/video_generation/` - OpenAI asynchronous video submission,
  polling, download, request, response, and error mapping

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [OpenAI model catalog](https://developers.openai.com/api/docs/models)
- [OpenAI Responses streaming reference](https://developers.openai.com/api/reference/typescript/resources/beta/subresources/responses/methods/create)
- [`../../docs/protocol/image-generation.md`](../../docs/protocol/image-generation.md)
- [`../../docs/protocol/live-image-api-tests.md`](../../docs/protocol/live-image-api-tests.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
- [`../../docs/protocol/live-video-api-tests.md`](../../docs/protocol/live-video-api-tests.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
