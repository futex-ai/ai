# ai-models-anthropic

`ai-models-anthropic` is the Anthropic-specific `ai-interface::Model` implementation for the workspace. Depend on it when you want to call Anthropic message models with explicit auth and shared runtime wrappers from neighboring crates.

## Responsibilities

- Implement the Anthropic model client behind `ai_interface::Model`
- Export strongly typed known Anthropic model metadata for model routing
- Map shared model/tool DTOs to the Anthropic messages API
- Parse Anthropic responses into shared response DTOs and typed model errors

## What This Crate Does

`AnthropicModel` accepts a `json-http` client plus explicit auth input and handles:

- Anthropic messages request serialization
- Anthropic tool-use and tool-result content blocks
- Anthropic `stop_reason` normalization into `ai_interface::FinishReason`
- terminal Anthropic stop reasons suppress parsed tool calls unless the
  normalized finish reason is `ToolCalls`
- Anthropic adaptive thinking mapping from catalog `ThinkingLevel`
- validated structured-output requests via JSON-only final responses
- provider response usage extraction into normalized input, output, cached
  input, and cache-creation token counts; cache creation is folded into regular
  input usage
- status, transport, and structured-output validation failure mapping onto
  `ai_interface::ModelError`

This crate does not load config, read environment variables, or resolve credentials on its own.
It exports `known_models()` and typed catalog id constants
(`CLAUDE_SONNET_4_6`, `CLAUDE_OPUS_4_7`, `CLAUDE_OPUS_4_7_THINKING_MAX`,
`CLAUDE_HAIKU_4_5`) so composition roots can validate deployment config and
sort routes without duplicating Anthropic model metadata. The Opus max variant
sends provider model id `claude-opus-4-7`, enables adaptive thinking, sets
`output_config.effort` to `max`, and requests omitted thinking display.
Reasoning/thinking content blocks in provider responses are ignored and are not
surfaced as assistant text.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_anthropic::{AnthropicModel, CLAUDE_SONNET_4_6, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> AnthropicModel {
    AnthropicModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        CLAUDE_SONNET_4_6,
        "anthropic-demo",
    )
}

fn known_model_count() -> usize {
    known_models().len()
}
```

## Development

```sh
cargo test -p ai-models-anthropic
cargo clippy -p ai-models-anthropic --all-targets --all-features -- -D warnings
```

### Key Code

- `src/anthropic/mod.rs` - `Model` implementation and request dispatch
- `src/catalog.rs` - known Anthropic model ids and routing metadata
- `src/anthropic/request.rs` - Anthropic request DTO mapping
- `src/anthropic/response.rs` - Anthropic response parsing

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
