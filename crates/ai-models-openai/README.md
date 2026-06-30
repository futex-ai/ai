# ai-models-openai

`ai-models-openai` is the OpenAI-specific `ai-interface` implementation crate
for the workspace. Depend on it when you want to call OpenAI Responses
generation models or the audio transcription endpoint with explicit auth and shared
runtime wrappers from neighboring crates.

## Responsibilities

- Implement the OpenAI model client behind `ai_interface::Model`
- Implement the OpenAI speech-to-text client behind
  `ai_interface::AudioTranscriber`
- Export strongly typed known OpenAI model metadata for model routing
- Map shared model/tool DTOs to the OpenAI Responses API
- Map shared audio transcription DTOs to `v1/audio/transcriptions`
- Parse OpenAI responses into shared response DTOs and typed model errors

## What This Crate Does

`OpenAiModel` accepts a `json-http` client plus explicit auth input and handles:

- OpenAI Responses request serialization
- OpenAI tool-call parsing
- OpenAI `finish_reason` normalization into `ai_interface::FinishReason`
  before exposing tool calls, so incomplete or failed Responses results cannot
  dispatch partial function-call output items
- OpenAI `text.format` JSON-schema mapping for structured outputs, using
  non-strict mode so callers can pass the broader shared schema contract
- OpenAI `reasoning.effort` mapping from catalog `ThinkingLevel` for
  reasoning-capable catalog variants
- stateless `store: false` generation calls using caller-owned conversation
  state
- `reasoning.encrypted_content` inclusion for reasoning models so encrypted
  reasoning items can be replayed across stateless tool-calling turns
- raw Responses function-call item retention so stateless tool continuations
  replay OpenAI's provider item id and original argument string instead of a
  normalized JSON render
- provider response usage extraction into normalized input, output, cached
  input, and reasoning token counts
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`, with retryable `408`, `409`, `425`, and `5xx`
  transcription statuses mapped to transient provider failures

This crate does not load config, read environment variables, or resolve credentials on its own.
It exports `known_models()` and typed catalog id constants (`GPT_5_5`,
`GPT_5_5_THINKING_LOW`, `GPT_5_5_THINKING_HIGH`,
`GPT_5_5_THINKING_EXTRA_HIGH`, `GPT_5_5_MINI`, `GPT_5_5_NANO`) so
composition roots can validate deployment config and sort routes without
duplicating OpenAI model metadata. The `gpt-5.5` thinking variants all send
provider model id `gpt-5.5`; `gpt-5.5` supports up to `ExtraHigh`, so this
catalog does not define a max-thinking OpenAI variant.
OpenAI generation uses workspace-defined function tools with `strict: false` during
the Responses cutover. OpenAI built-in tools are intentionally not exposed by
this crate.
When OpenAI returns Responses `reasoning` or `function_call` output items, this
crate stores those replay-sensitive items in `ModelResponse::provider_context`;
runtimes should keep that context on the assistant message so later OpenAI
requests can replay the provider items before the associated function-call
outputs. The normalized `ToolCall` list remains the tool-dispatch contract, but
the raw provider context is preferred for OpenAI request replay when present.

`OpenAiAudioTranscriber` submits completed audio recordings to the OpenAI
transcription endpoint using `gpt-4o-mini-transcribe` or another caller-chosen
transcription model. It expects the caller to provide the API key and the
uploaded audio media type. It applies a 60-second request timeout and surfaces
retryable OpenAI transcription statuses as transient errors.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{AudioTranscriber, Model};
use ai_models_openai::{GPT_5_5, OpenAiAudioTranscriber, OpenAiModel, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> OpenAiModel {
    OpenAiModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        GPT_5_5,
        "sk-demo",
    )
}

fn known_model_count() -> usize {
    known_models().len()
}

fn build_transcriber() -> OpenAiAudioTranscriber {
    OpenAiAudioTranscriber::new("gpt-4o-mini-transcribe", "sk-demo")
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
- `src/openai/request.rs` - OpenAI Responses request DTO mapping
- `src/openai/response.rs` - OpenAI Responses response parsing
- `src/openai/transcription.rs` - OpenAI audio transcription implementation

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
