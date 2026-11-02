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
- Consume Gemini `streamGenerateContent` SSE response fragments
- Emit public assistant and thought text deltas in provider order
- Map shared image generation and edit DTOs to the Google `generateContent` API
- Map shared video DTOs to the Google long-running prediction API
- Parse Google responses into shared response DTOs and typed model errors

## What This Crate Does

`GoogleModel` accepts a `json-http` client, a `ModelSpec`, and explicit auth
input. Each call runs the shared capability check of `ai-models-core`
against the spec before transport. The adapter handles:

- Gemini request serialization through `streamGenerateContent?alt=sse`
- opt-in `complete_with_events` delivery for candidate text and thought parts,
  including normalized newlines between retained assistant parts
- 3,600-second default overall completion deadline, 120-second stream idle
  deadline, and explicit per-call total-timeout overrides
- portable generation controls, including required-with-automatic-fallback
  function calling, per-call timeouts, and blank-system omission
- complete function schemas through `parametersJsonSchema`, kept separate
  from structured response schemas in `responseJsonSchema`
- function call and function response content parts
- full ordered model content in `ProviderConversationItem::GoogleAssistantMessage`
  when any part has a `thoughtSignature`. Replay preserves the original text,
  thought flags, function-call ids and arguments, and signatures. A missing
  provider call id stays absent in replay and tool results even when runtime
  dispatch needs a synthetic id. Unsigned calls keep their runtime id on both
  the call and its result. Signed parts, including empty text and signature-only
  parts, stay separate in replay data. Signatures do not change answer text or
  public deltas. Text continues across chunks, and separate text parts in one chunk
  keep their newline. The tool runtime removes this private context from log
  copies.
- inline `inlineData` parts for shared base64 image and video user content
- Gemini `finishReason` normalization into `ai_interface::FinishReason`,
  including deriving normal tool-call completion from parsed `functionCall`
  parts and preserving prompt-level safety blocks as filtered responses even
  when Gemini returns no candidates
- terminal Gemini finish reasons suppress parsed tool calls unless the
  normalized finish reason is `ToolCalls`
- Gemini function-call parsing that treats omitted `args` as `{}` for no-arg
  tools
- Gemini function-call parsing assigns deterministic request-scoped local ids
  and operation ids when Gemini omits `functionCall.id`, preventing distinct
  no-id calls from sharing runtime idempotency keys
- Gemini `generationConfig.responseJsonSchema` mapping for structured outputs
- Gemini 3 `generationConfig.thinkingConfig.thinkingLevel` mapping from the
  resolved request effort, while retaining Gemini 2.5 `thinkingBudget`
  mapping
- effort lowering to the highest effort that the spec lists at or below the
  request, with the resolved effort and the text id retained in responses
- provider response usage extraction into normalized input, output, cached
  input, and thinking token counts
- ordered accumulation of cross-chunk text, complete function-call parts,
  thinking parts, final finish reason, and final usage metadata before the
  existing buffered response parser runs
- typed classification of failures before stream progress as retryable or
  provider errors and failures after progress as `ModelError::Interrupted`
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
as provider thoughts are not surfaced as assistant text, but event-observing
callers receive them as reasoning deltas. Schema-constrained calls suppress all
public deltas in version one.

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

- `src/google/mod.rs` - `Model` implementation and request dispatch
- `src/catalog.rs` - Google provider id, model spec constants, `CATALOG`,
  custom defaults, and the retained Gemini 2.5 provider model ids
- `src/google/request.rs` - Google request and continuation mapping
- `src/google/request_types.rs` - typed Gemini request bodies
- `src/google/response.rs` - Google response parsing
- `src/google/stream/` - public fragment emission, terminal accumulation, and
  stream failure mapping
- `src/google/image_generation/` - Google image request, response, and error mapping
- `src/google/video_generation/` - Google long-running video submission,
  polling, download, request, response, and error mapping

### Related Docs

- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-completion-streaming.md`](../../docs/protocol/model-completion-streaming.md)
- [`../../docs/protocol/model-completion-events.md`](../../docs/protocol/model-completion-events.md)
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
