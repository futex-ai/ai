# ai-models-google

`ai-models-google` is the Google-specific `ai-interface::Model` implementation for the workspace. Depend on it when you want to call Gemini models with explicit auth and shared runtime wrappers from neighboring crates.

## Responsibilities

- Implement the Google model client behind `ai_interface::Model`
- Export strongly typed known Google model metadata for model routing
- Map shared model/tool DTOs to the Google `generateContent` API
- Parse Google responses into shared response DTOs and typed model errors

## What This Crate Does

`GoogleModel` accepts a `json-http` client plus explicit auth input and handles:

- Gemini request serialization
- function call and function response content parts
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
the thinking control unset so the provider uses its minimal default. All
existing Gemini 2.5 entries remain available and continue to use fixed
`thinkingBudget` values. Response parts marked as provider thoughts are
ignored and are not surfaced as assistant text.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_google::{GEMINI_3_6_FLASH, GoogleModel, known_models};
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

### Related Docs

- [Latest Gemini models](https://ai.google.dev/gemini-api/docs/latest-model)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
