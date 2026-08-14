# ai-models-google

`ai-models-google` provides Google-specific chat and image implementations for
the shared AI interfaces. Depend on it when you want to call Gemini models with
explicit auth and shared runtime wrappers from neighboring crates.

## Responsibilities

- Implement the Google model client behind `ai_interface::Model`
- Implement the Google image client behind `ai_interface::ImageGenerator`
- Export strongly typed known Google model metadata for model routing
- Map shared model/tool DTOs to the Google `generateContent` API
- Map shared image generation and edit DTOs to the Google `generateContent` API
- Parse Google responses into shared response DTOs and typed model errors

## What This Crate Does

`GoogleModel` accepts a `json-http` client plus explicit auth input and handles:

- Gemini request serialization
- portable generation controls, including required-with-automatic-fallback
  function calling, per-call timeouts, and blank-system omission
- complete function schemas through `parametersJsonSchema`, kept separate
  from structured response schemas in `responseJsonSchema`
- function call and function response content parts
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
- Gemini 3 `generationConfig.thinkingConfig.thinkingLevel` mapping from catalog
  `ThinkingLevel`, while retaining Gemini 2.5 `thinkingBudget` mapping
- catalog-aware downgrade to the highest configured thinking level not above
  an unsupported request, with the effective level retained in responses
- provider response usage extraction into normalized input, output, cached
  input, and thinking token counts
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports `known_models()` and typed catalog id
constants for Gemini 3.6 Flash and Gemini 3.5 Flash-Lite.
`GEMINI_3_6_FLASH` is the first catalog entry and the default used by workspace
examples. Its high-thinking variant sends provider model id
`gemini-3.6-flash` with `thinkingLevel: "high"`. Gemini 3.5 Flash-Lite leaves
the thinking control unset so the provider uses its minimal default. Gemini
2.5 models are not advertised in the routing catalog because the API does not
make them available to new users. Callers can still construct an explicit
model id using the retained Gemini 2.5 constants, and the request mapper keeps
fixed `thinkingBudget` compatibility for those models. Response parts marked
as provider thoughts are ignored and are not surfaced as assistant text.

`GoogleImageGenerator` uses the current `v1` compatibility endpoint with an
injected `json-http` client and explicit API key or auth hook. It maps text and
ordered source images to Gemini content parts, requests exactly one final image
modality, and maps shared aspect preferences to Google's typed
`ImageResponseFormat.AspectRatio` enum values. The adapter intentionally does
not map shared quality preferences, because Gemini's resolution control is a
different contract. It skips interim thought images, rejects empty decoded
payloads, decodes the first final image, normalizes usage, and retains safety
messages in typed content-policy errors. `GEMINI_3_1_FLASH_IMAGE` identifies
the balanced image catalog entry.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::ImageGenerator;
use ai_models_google::{
    GEMINI_3_1_FLASH_IMAGE, GEMINI_3_6_FLASH, GoogleImageGenerator, GoogleModel,
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

fn known_model_count() -> usize {
    known_models().len()
}

fn build_image_generator() -> GoogleImageGenerator {
    GoogleImageGenerator::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GEMINI_3_1_FLASH_IMAGE,
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
- `src/catalog.rs` - known Google model ids and routing metadata
- `src/google/request.rs` - Google request DTO mapping
- `src/google/response.rs` - Google response parsing
- `src/google/image_generation/` - Google image request, response, and error mapping

### Related Docs

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [Latest Gemini models](https://ai.google.dev/gemini-api/docs/latest-model)
- [`../../docs/protocol/image-generation.md`](../../docs/protocol/image-generation.md)
- [`../../docs/protocol/live-image-api-tests.md`](../../docs/protocol/live-image-api-tests.md)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
