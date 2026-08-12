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
- portable sampling outside thinking mode, bounded output limits, stop
  sequences, tool choice, per-call timeouts, and blank-system omission
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

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports `known_models()` and typed catalog id
constants for Claude Sonnet 5, Opus 5, and Fable 5. `CLAUDE_SONNET_5` is the
first catalog entry and the balanced default used by workspace examples.
`CLAUDE_OPUS_5_THINKING_MAX` sends provider model id `claude-opus-5`, enables
adaptive thinking, sets `output_config.effort` to `max`, and requests omitted
thinking display. All existing Sonnet 4.6, Opus 4.7, and Haiku 4.5 entries
remain available for pinned deployments.
Reasoning/thinking content blocks in provider responses are ignored and are not
surfaced as assistant text.

## Quick Start

```rust
use std::sync::Arc;

use ai_models_anthropic::{AnthropicModel, CLAUDE_SONNET_5, known_models};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> AnthropicModel {
    AnthropicModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        CLAUDE_SONNET_5,
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

- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [Anthropic model overview](https://platform.claude.com/docs/en/about-claude/models/overview)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../plans/README.md`](../../plans/README.md)
