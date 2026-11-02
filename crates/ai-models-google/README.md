# ai-models-google

`ai-models-google` provides Google-specific chat, image, and video
implementations for the shared AI interfaces. Depend on it when you want to
call Gemini or Veo models with explicit auth and shared runtime wrappers from
neighboring crates.

## Responsibilities

- Implement the Google model client behind `ai_interface::Model`
- Implement the Google image client behind `ai_interface::ImageGenerator`
- Implement the Google video client behind `ai_interface::VideoGenerator`
- Export the Google `PROVIDER_ID`, one `ModelSpec` constant for each known
  model, the `CATALOG` slice, and the `CUSTOM_DEFAULTS` of unlisted models
- Map shared model/tool DTOs to the Google `generateContent` API
- Map Gemini `streamGenerateContent` SSE chunks to model events in provider
  order: text, thought text, function calls, usage, and finish
- Map shared image generation and edit DTOs to the Google `generateContent` API
- Map shared video DTOs to the Google long-running prediction API
- Map Gemini finish reasons, usage, and failures to finish events, usage
  events, and typed model errors
- Record the original Gemini parts as Google provider state on the parts
  that they produce, replay that state, and convert stored version 1 Google
  items

## What This Crate Does

`GoogleModel` accepts a `json-http` client, a `ModelSpec`, and explicit auth
input. Each call runs the shared capability check of `ai-models-core`
against the spec before transport. The adapter handles:

- Gemini request serialization through `streamGenerateContent?alt=sse`
- `Model::stream`: candidate-zero text parts as text deltas, thought parts as
  reasoning deltas, and each function-call part as one tool-call start, one
  argument delta, and one end event, in provider order. The first text part
  of a chunk continues the last text part of the previous chunk when both
  are thoughts or both are not. Separate text parts of one chunk are
  separate blocks, a block that holds only whitespace adds no text, and each
  later block starts with a newline. One usage event and one finish event
  follow the end of the stream. `Model::complete` folds these events.
- 3,600-second default overall completion deadline, 120-second stream idle
  deadline, and explicit per-call total-timeout overrides
- portable generation controls, including required-with-automatic-fallback
  function calling, per-call timeouts, and blank-system omission
- complete function schemas through `parametersJsonSchema`, kept separate
  from structured response schemas in `responseJsonSchema`
- function call and function response content parts
- the original Gemini parts as Google provider state on the parts that they
  produce, when any part has a `thoughtSignature`. Each text, thought, and
  function-call part then carries its original parts, including empty text
  and signature-only parts. Replay sends every original part in its original
  order and preserves the original text, thought flags, function-call ids
  and arguments, and signatures. Every function call carries its original
  call and its runtime call, also without a signature. In a signed message,
  a missing provider call id stays absent in replay and tool results even
  when runtime dispatch needs a synthetic id. Unsigned calls keep their
  runtime id on both the call and its result. Signatures do not change
  answer text or text deltas. Own state with an unknown version or a payload
  that does not decode fails before transport with the `InvalidRequest` error kind.
  Log copies drop this state.
- `GoogleLegacyProviderConverter` for a stored version 1
  `google_assistant_message` item. In a message with items from Google only,
  it gives one part for each original part, in the original order, with
  that part as the state of the new part. A
  function call takes precedence over the other fields of its part. A
  thought, and a part with neither text nor a function call, give a
  reasoning part. The portable text and calls still come from the version 1
  `content` and `tool_calls`. A function call without a `tool_calls` entry
  stays as state on an empty reasoning part, so the converter invents no
  portable call. Text and calls that no original part holds come after the
  parts of the item. Their state holds no part, so they add no native Gemini
  part to the request. An item without parts keeps its assistant message.
  Add the converter to the `LegacyMessageConverter` of the host. In a message
  with items from several providers, the composition keeps every full
  content item whole on one reasoning part with its provider's id, at every
  item position. A read without this converter also keeps the item whole.
  The adapter sends the first whole Google item as the entire content of
  that message, as version 1 did. Portable text and calls then add no native
  parts.
- inline `inlineData` parts for shared base64 image and video user content
- Gemini `finishReason` normalization into `ai_interface::FinishReason`,
  including deriving normal tool-call completion from parsed `functionCall`
  parts and preserving prompt-level safety blocks as filtered responses even
  when Gemini returns no candidates
- the folded response keeps parsed tool calls only when the normalized
  finish reason is `ToolCalls`
- Gemini function-call parsing that treats omitted `args` as `{}` for no-arg
  tools
- Gemini function-call parsing assigns deterministic request-scoped local ids
  and operation ids when Gemini omits `functionCall.id`, preventing distinct
  no-id calls from sharing runtime idempotency keys. The tool-call start
  event carries both ids.
- Gemini `generationConfig.responseJsonSchema` mapping for structured outputs
- Gemini 3 `generationConfig.thinkingConfig.thinkingLevel` mapping from the
  resolved request effort, while retaining Gemini 2.5 `thinkingBudget`
  mapping
- effort lowering to the highest effort that the spec lists at or below the
  request, with the resolved effort and the text id retained in responses
- provider response usage extraction into normalized input, output, cached
  input, and thinking token counts
- the last finish reason and the last usage metadata of the stream as those
  of the call. The stream must end after a finish reason or a prompt block
- typed classification of failures before stream progress as retryable or
  provider errors and failures after progress as the `Interrupted` error kind
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports one `ModelSpec` constant for each catalog
model: `GEMINI_3_6_FLASH`, `GEMINI_3_5_FLASH_LITE`,
`GEMINI_3_1_FLASH_IMAGE`, and `VEO_3_1_GENERATE_PREVIEW`. `CATALOG` lists
them in that order, and `known_models()` returns a copy of it.
`GEMINI_3_6_FLASH` is the first catalog spec and the default used by
workspace examples. It lists low, medium, and high efforts. Its default
effort is medium. A request with the high
effort sends provider model id `gemini-3.6-flash` with
`thinkingLevel: "high"`. Gemini 3.5 Flash-Lite lists only the disabled
effort and leaves the thinking control unset so the provider uses its
minimal default. The
[control matrix](../../docs/protocol/model-capabilities.md#control-matrix)
lists the efforts and limits of each spec. Gemini 2.5 models are not in the
catalog because the API does not make them available to new users. Callers
can still build them from the retained `GEMINI_2_5_PRO`, `GEMINI_2_5_FLASH`,
and `GEMINI_2_5_FLASH_LITE` provider model ids with
`ModelSpec::custom(PROVIDER_ID, GEMINI_2_5_PRO, &CUSTOM_DEFAULTS)`. The
request mapper keeps fixed `thinkingBudget` compatibility for those models.
A custom spec starts with the disabled effort as its default. Response parts marked
as provider thoughts are not surfaced as assistant text, but stream consumers
receive them as reasoning deltas. A call with a response schema emits text
deltas like any other call, and its finish event carries the validated
structured output.

Each adapter attaches shared error details with its configured model id.
Adapters inspect `x-goog-request-id`, `x-request-id`, and `request-id` as optional aliases.
Errors inside successful HTTP responses retain those response headers.
Late stream and output failures keep the relevant metadata and safe original sources.
`with_clock` supplies a wall clock for HTTP-date retry hints.
Provider diagnostics and typed media details redact the credentials applied for that call.

`GoogleImageGenerator` uses the current `v1` compatibility endpoint with an
injected `json-http` client and explicit API key or auth hook. It maps text and
ordered source images to Gemini content parts, requests exactly one final image
modality, and maps shared aspect preferences to Google's typed
`ImageResponseFormat.AspectRatio` enum values. The adapter intentionally does
not map shared quality preferences, because Gemini's resolution control is a
different contract. It skips interim thought images, rejects empty decoded
payloads, decodes the first final image, normalizes usage, and retains safety
messages in typed content-policy errors. The `GEMINI_3_1_FLASH_IMAGE` spec
identifies the balanced image catalog model, and its `provider_model_id` is
the id to pass.

`GoogleVideoGenerator` submits text or one inline first frame to Veo's
`predictLongRunning` endpoint, polls the returned operation, and downloads the
first generated MP4 with the same API-key auth. The
`VEO_3_1_GENERATE_PREVIEW` spec identifies the video catalog model, and its
`provider_model_id` is the id to pass. The adapter maps portable
landscape/portrait, four/eight-second, and 720p controls, enforces one
ten-minute total deadline through an injected polling runtime, rejects download
URIs outside the configured API origin, and keeps policy, timeout, rate-limit,
transient, and terminal failures typed.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{ImageGenerator, ModelSpec, VideoGenerator};
use ai_models_google::{
    CUSTOM_DEFAULTS, GEMINI_2_5_PRO, GEMINI_3_1_FLASH_IMAGE, GEMINI_3_6_FLASH, PROVIDER_ID,
    VEO_3_1_GENERATE_PREVIEW, GoogleImageGenerator, GoogleModel, GoogleVideoGenerator,
    known_models,
};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> GoogleModel {
    GoogleModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GEMINI_3_6_FLASH,
        "google-demo",
    )
}

fn build_gemini_2_5_model() -> GoogleModel {
    GoogleModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        ModelSpec::custom(PROVIDER_ID, GEMINI_2_5_PRO, &CUSTOM_DEFAULTS),
        "google-demo",
    )
}

fn known_model_count() -> usize {
    known_models().len()
}

fn build_image_generator() -> GoogleImageGenerator {
    GoogleImageGenerator::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GEMINI_3_1_FLASH_IMAGE.provider_model_id,
        "google-demo",
    )
}

fn build_video_generator() -> GoogleVideoGenerator {
    GoogleVideoGenerator::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        VEO_3_1_GENERATE_PREVIEW.provider_model_id,
        "google-demo",
    )
}
```

## Development

```sh
cargo test -p ai-models-google
cargo clippy -p ai-models-google --all-targets --all-features -- -D warnings
```

### Key Code

- `src/google/mod.rs` - `Model::stream` implementation and request dispatch
- `src/catalog.rs` - Google provider id, model spec constants, `CATALOG`,
  custom defaults, and the retained Gemini 2.5 provider model ids
- `src/google/request.rs` - Google request mapping
- `src/google/contents.rs` - maps ordered parts to Gemini contents, and
  replays own state
- `src/google/state.rs` - the private, versioned Gemini part state and its
  decoding
- `src/google/legacy.rs` - `GoogleLegacyProviderConverter`, the conversion of
  stored version 1 items
- `src/google/request_types.rs` - typed Gemini request bodies
- `src/google/response.rs` - typed terminal metadata and output checks
- `src/google/stream/` - typed Gemini chunks, model event mapping, replay
  parts, and stream failure mapping
- `src/google/image_generation/` - Google image request, response, and error mapping
- `src/google/video_generation/` - Google long-running video submission,
  polling, download, request, response, and error mapping

### Related Docs

- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/conversation-parts.md`](../../docs/protocol/conversation-parts.md)
- [`../../docs/protocol/live-model-api-tests.md`](../../docs/protocol/live-model-api-tests.md)
- [Latest Gemini models](https://ai.google.dev/gemini-api/docs/latest-model)
- [Gemini thought signatures](https://ai.google.dev/gemini-api/docs/generate-content/thought-signatures)
- [Gemini generate-content API](https://ai.google.dev/api/generate-content)
- [`../../docs/protocol/image-generation.md`](../../docs/protocol/image-generation.md)
- [`../../docs/protocol/live-image-api-tests.md`](../../docs/protocol/live-image-api-tests.md)
- [`../../docs/protocol/video-generation.md`](../../docs/protocol/video-generation.md)
- [`../../docs/protocol/live-video-api-tests.md`](../../docs/protocol/live-video-api-tests.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
