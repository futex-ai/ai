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
- Export the OpenAI `PROVIDER_ID`, one `ModelSpec` constant for each known
  model, the `CATALOG` slice, and the `CUSTOM_DEFAULTS` of unlisted models
- Map shared model/tool DTOs to the OpenAI Responses API
- Map OpenAI Responses SSE events to model events in provider order: output
  text, reasoning, function calls, usage, and finish
- Map shared audio transcription DTOs to `v1/audio/transcriptions`
- Map shared image DTOs to `v1/images/generations` and `v1/images/edits`
- Map shared video DTOs to asynchronous `v1/videos` jobs and downloads
- Map Responses terminal events and failures to finish events, usage events,
  and typed model errors

## What This Crate Does

`OpenAiModel` accepts a `json-http` client, a `ModelSpec`, and explicit auth
input. Each call runs the shared capability check of `ai-models-core`
against the spec before transport. The adapter handles:

- OpenAI Responses request serialization with `"stream": true`
- `Model::stream`: a text delta for each output-text or refusal fragment, a
  reasoning delta for each reasoning-summary or reasoning-text fragment, and
  tool-call start, argument, and end events for each function-call item, in
  provider order. Output text and refusal text join without a separator. One
  usage event and one finish event follow the terminal `response.completed`
  or `response.incomplete` event. `Model::complete` folds these events.
  Item completion adds only the suffix of terminal text that earlier deltas
  did not deliver. State-only events do not count as delivered text.
  Divergent terminal text adds nothing.
- calls with a response schema emit text deltas like any other call, and
  the finish event carries the validated structured output
- 3,600-second default overall completion deadline, 120-second stream idle
  deadline, and explicit per-call total-timeout overrides
- shared text and image parts as Responses input content, with typed rejection
  of shared video parts before transport
- portable temperature and top-p outside reasoning mode, output-limit and
  strict or required-with-automatic-fallback tool-choice mapping, per-call
  timeouts, blank-instruction omission, and typed rejection of unsupported
  stop sequences
- OpenAI tool-call parsing
- OpenAI terminal status normalization into `ai_interface::FinishReason`.
  The folded response keeps tool calls only for the `ToolCalls` finish
  reason, so incomplete Responses results cannot dispatch partial
  function-call output items
- OpenAI `text.format` JSON-schema mapping for structured outputs, using
  non-strict mode so callers can pass the broader shared schema contract
- OpenAI `reasoning.effort` mapping from the resolved request effort for
  reasoning-capable specs
- effort lowering to the highest effort that the spec lists at or below the
  request, with the resolved effort and the text id retained in responses
- stateless `store: false` generation calls using caller-owned conversation
  state
- `reasoning.encrypted_content` inclusion for reasoning models so encrypted
  reasoning items can be replayed across stateless tool-calling turns
- raw Responses function-call item retention so stateless tool continuations
  replay OpenAI's provider item id and original argument string instead of a
  normalized JSON render
- assistant message `phase` retention so stateless replay preserves tool
  preambles and final-answer phase metadata in the original output-item order
- phase-less assistant message markers when function-call replay state needs
  the original OpenAI output item order
- isolation from the provider state of every other provider, including
  DeepSeek, Kimi, and MiniMax assistant reasoning state and raw tool calls
- provider response usage extraction into normalized input, output, cached
  input, and reasoning token counts
- `response.incomplete` ends the attempt with its partial output: the
  truncated finish reason for `max_output_tokens`, the filtered finish reason
  for `content_filter`, and the other finish reason otherwise
- typed classification of completion failures before stream progress as
  retryable or provider errors and failures after progress as
  the `Interrupted` error kind
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`, with retryable `408`, `409`, `425`, and `5xx`
  transcription statuses mapped to transient provider failures

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports one `ModelSpec` constant for each catalog
model: `GPT_5_6_SOL`, `GPT_5_6_TERRA`, `GPT_5_6_LUNA`, `GPT_5_5`,
`GPT_5_4_MINI`, `GPT_5_4_NANO`, `GPT_IMAGE_2`, and `SORA_2`. `CATALOG` lists
them in that order, and `known_models()` returns a copy of it.
`GPT_5_6_SOL` is the first catalog spec and the default used by workspace
examples. Its default effort is medium, and it also lists low, high, extra
high, and max. Every effort sends provider model id `gpt-5.6-sol`.
Terra and Luna list low and medium efforts, with medium as their default. The
GPT-5.5 spec remains available for pinned deployments. Cost-optimized routing
uses GPT-5.4 Mini and Nano because OpenAI does not expose GPT-5.5 Mini or
Nano aliases. The
[control matrix](../../docs/protocol/model-capabilities.md#control-matrix)
lists the efforts and limits of each spec. Build a model that the catalog
does not list from
`ModelSpec::custom(PROVIDER_ID, model_id, &CUSTOM_DEFAULTS)`. Its default
effort is disabled.
OpenAI generation uses workspace-defined function tools with `strict: false` during
the Responses cutover. OpenAI built-in tools are intentionally not exposed by
this crate.
Each replay-sensitive Responses output item becomes OpenAI provider state on
its part. A `reasoning` item goes on its reasoning part with its id, summary,
and encrypted content. An assistant `message` item goes on its text part with
its `phase`. A `function_call` item goes on the end event of its call with
its item id, call id, name, and raw arguments. Runtimes keep every part of
the response on the assistant message, for example with
`ModelResponse::to_assistant_message()`. Later OpenAI requests then replay
the phased assistant message and the provider items, in their original
order, before the associated function-call outputs. The portable tool calls
remain the tool-dispatch contract, but own state is preferred for OpenAI
request replay when present. Own state with an unknown version or a payload
that does not decode fails before transport with the `InvalidRequest` error kind.

`OpenAiLegacyConverter` converts stored version 1 `openai_message`,
`openai_reasoning`, and `openai_function_call` items. It keeps their
`provider_context` order, and the text part takes the phase of the first
`openai_message` item only. Text and portable tool calls without an item come
after the parts of the items. When a native call item exists, an unmatched
portable call carries a private skip marker and adds nothing to OpenAI
requests. It keeps its position after the text. Native call items without a
portable call stay on empty reasoning parts and still reach OpenAI.
Add the converter to the `LegacyMessageConverter` of the host.

All OpenAI adapters attach shared error details with the configured model id.
Errors retain `Retry-After` and the `x-request-id` response header.
Late stream, decode, and download failures keep the relevant response metadata.
Each adapter offers `with_clock` for HTTP-date retry hints.
Provider diagnostics and typed media details redact the actual call credentials.
Malformed generated output uses `InvalidOutput`; invalid caller schemas use `InvalidRequest`.

`OpenAiAudioTranscriber` submits completed audio recordings to the OpenAI
transcription endpoint using `gpt-4o-mini-transcribe` or another caller-chosen
transcription model. It expects the caller to provide the API key and the
uploaded audio media type. It applies a 60-second request timeout and surfaces
retryable OpenAI transcription statuses as transient errors. Its direct reqwest
client, multipart request, and redirect behavior stay unchanged.

`OpenAiImageGenerator` accepts an injected `json-http` client and explicit
model id. The `GPT_IMAGE_2` spec identifies the current image catalog model,
and its `provider_model_id` is the id to pass. Requests
without source images use the JSON generation endpoint; requests with source
images use multipart edit requests. The injected client makes auth,
serialization, timeout, transport, and status behavior credential-free to
test. The adapter rejects empty decoded payloads, preserves any revised prompt,
normalizes usage, and keeps content-policy refusals distinct from retryable
provider failures.

`OpenAiVideoGenerator` submits text-to-video JSON or first-frame multipart
requests to `v1/videos`, polls queued jobs through `GET /v1/videos/{id}`, then
downloads the completed MP4 through the authenticated binary transport path.
The `SORA_2` spec identifies the 720p video catalog model, and its
`provider_model_id` is the id to pass. The adapter supports portable
landscape/portrait and four/eight-second controls, enforces one ten-minute total
deadline with an injected polling runtime, and keeps policy, timeout, rate
limit, transient, and terminal failures typed. This upstream API is currently
scheduled to shut down on September 24, 2026, so consumers should treat the
adapter as a time-bounded compatibility integration.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{AudioTranscriber, ImageGenerator, Model, ModelSpec, VideoGenerator};
use ai_models_openai::{
    CUSTOM_DEFAULTS, GPT_5_6_SOL, GPT_IMAGE_2, PROVIDER_ID, SORA_2, OpenAiAudioTranscriber,
    OpenAiImageGenerator, OpenAiModel, OpenAiVideoGenerator, known_models,
};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> OpenAiModel {
    OpenAiModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GPT_5_6_SOL,
        "sk-demo",
    )
}

fn build_unlisted_model() -> OpenAiModel {
    OpenAiModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        ModelSpec::custom(PROVIDER_ID, "gpt-preview", &CUSTOM_DEFAULTS),
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
        GPT_IMAGE_2.provider_model_id,
        "sk-demo",
    )
}

fn build_video_generator() -> OpenAiVideoGenerator {
    OpenAiVideoGenerator::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        SORA_2.provider_model_id,
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

- `src/openai/mod.rs` - `Model::stream` implementation and request dispatch
- `src/catalog.rs` - OpenAI provider id, model spec constants, `CATALOG`, and
  custom defaults
- `src/openai/request.rs` - maps the resolved controls and the effort to the
  Responses request
- `src/openai/request_types.rs` - OpenAI Responses request DTOs
- `src/openai/response/` - typed Responses output items and finish reasons
- `src/openai/replay_state.rs` - the private, versioned message, reasoning,
  and function-call state
- `src/openai/legacy.rs` - `OpenAiLegacyConverter`, the conversion of stored
  version 1 items
- `src/openai/stream/` - typed Responses SSE events, model event mapping,
  provider state on parts, and stream failure mapping
- `src/openai/_tests_/openai_event_tests.rs` - event order, schema events,
  and interruption coverage
- `src/openai/transcription.rs` - OpenAI audio transcription implementation
- `src/openai/image_generation/` - OpenAI image request, response, and error mapping
- `src/openai/video_generation/` - OpenAI asynchronous video submission,
  polling, download, request, response, and error mapping

### Related Docs

- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/conversation-parts.md`](../../docs/protocol/conversation-parts.md)
- [`../../docs/protocol/live-model-api-tests.md`](../../docs/protocol/live-model-api-tests.md)
- [OpenAI model catalog](https://developers.openai.com/api/docs/models)
- [OpenAI Responses streaming reference](https://developers.openai.com/api/reference/typescript/resources/beta/subresources/responses/methods/create)
- [`../../docs/protocol/image-generation.md`](../../docs/protocol/image-generation.md)
- [`../../docs/protocol/live-image-api-tests.md`](../../docs/protocol/live-image-api-tests.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
- [`../../docs/protocol/live-video-api-tests.md`](../../docs/protocol/live-video-api-tests.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
