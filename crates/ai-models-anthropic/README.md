# ai-models-anthropic

`ai-models-anthropic` is the Anthropic-specific `ai-interface::Model` implementation for the workspace. Depend on it when you want to call Anthropic message models with explicit auth and shared runtime wrappers from neighboring crates.

## Responsibilities

- Implement the Anthropic model client behind `ai_interface::Model`
- Export the Anthropic `PROVIDER_ID`, one `ModelSpec` constant for each known
  model, the `CATALOG` slice, and the `CUSTOM_DEFAULTS` of unlisted models
- Map shared model/tool DTOs to the Anthropic messages API
- Map Anthropic Messages SSE events to model events in provider order: text,
  thinking, tool use, usage, and finish
- Map Anthropic stop reasons, usage, and failures to finish events, usage
  events, and typed model errors

## What This Crate Does

`AnthropicModel` accepts a `json-http` client, a `ModelSpec`, and explicit
auth input. Each call runs the shared capability check of `ai-models-core`
against the spec before transport. The adapter handles:

- Anthropic messages request serialization with `"stream": true`
- `Model::stream`: a text delta for each text fragment, a reasoning delta for
  each thinking fragment, and tool-call start, argument, and end events for
  each tool-use block, in provider order. One usage event and one finish
  event follow after `message_stop`. `Model::complete` folds these events.
- text block boundaries: a block that holds only whitespace adds no text, and
  the first text delta of each later block starts with a newline, so the
  folded text joins the blocks with newlines
- 3,600-second default overall completion deadline, 120-second stream idle
  deadline, and explicit per-call total-timeout overrides
- portable sampling outside thinking mode, bounded output limits, stop
  sequences, strict and required-with-automatic-fallback tool choice, per-call
  timeouts, and blank-system omission
- with thinking enabled, `Required` and `Function` tool choice return
  `ModelError::UnsupportedControl` for `ModelControl::ToolChoice` before
  transport. `RequiredOrAuto` sends `auto` and retains tool definitions.
  Without thinking, `RequiredOrAuto` sends `any`, as `Required` does. Every
  Anthropic spec holds this rule as `ForcedToolChoice::OnlyWithoutThinking`.
- output limits from `ModelSpec::max_output_tokens`: Anthropic specs require
  an output limit, so every request sends the caller's limit capped at the
  spec's maximum, or that maximum when no limit is given. Claude Sonnet 5,
  Opus 5, Fable 5, Sonnet 4.6, and Opus 4.7 use 128,000 tokens. Haiku 4.5
  uses 64,000 tokens. A custom spec from `CUSTOM_DEFAULTS` uses 64,000 tokens
  as both the default and the ceiling.
- Anthropic tool-use and tool-result content blocks
- shared text and image parts as Anthropic text and base64 image blocks, with
  typed rejection of shared video parts before transport
- Anthropic `stop_reason` normalization into `ai_interface::FinishReason`
- the folded response keeps parsed tool calls only when the normalized
  finish reason is `ToolCalls`
- Anthropic adaptive thinking mapping from the resolved request effort
- effort lowering to the highest effort that the spec lists at or below the
  request, with the resolved effort and the text id retained in responses
- validated structured-output requests via JSON-only final responses
- usage from `message_start` and `message_delta`, normalized into input,
  output, cached input, and cache-creation token counts; cache creation is
  folded into regular input usage
- typed classification of failures before stream progress as retryable and
  failures after progress as `ModelError::Interrupted`
- status, native stream error, transport, and structured-output validation
  failure mapping onto `ai_interface::ModelError`

This crate does not load config, read environment variables, or resolve
credentials on its own. It exports one `ModelSpec` constant for each catalog
model: `CLAUDE_SONNET_5`, `CLAUDE_OPUS_5`, `CLAUDE_FABLE_5`,
`CLAUDE_SONNET_4_6`, `CLAUDE_OPUS_4_7`, and `CLAUDE_HAIKU_4_5`. `CATALOG`
lists them in that order, and `known_models()` returns a copy of it.
`CLAUDE_SONNET_5` is the first catalog spec and the balanced default used by
workspace examples. The Claude 5 specs list low, medium, and high efforts.
Opus 5 also lists max. The Claude 5 specs default to the high effort. The
Sonnet 4.6, Opus 4.7, and Haiku 4.5 specs default to disabled thinking and
remain available for pinned deployments. The
[control matrix](../../docs/protocol/model-capabilities.md#control-matrix)
lists the efforts and limits of each spec.
`new`, `with_auth`, and `with_catalog_auth` all take a spec. Every Anthropic
spec fixes sampling while thinking is on, so adaptive-thinking requests omit
caller sampling controls as Anthropic requires. Build a model that the
catalog does not list from
`ModelSpec::custom(PROVIDER_ID, model_id, &CUSTOM_DEFAULTS)`. Its thinking
stays off unless the request sets an effort.
A request with the max effort on `CLAUDE_OPUS_5` sends provider model id
`claude-opus-5`, enables adaptive thinking, sets `output_config.effort` to
`max`, and requests omitted thinking display.
When a response has `thinking` or `redacted_thinking` blocks, the adapter keeps
its full ordered content in `ProviderConversationItem::AnthropicAssistantMessage`.
The finish event carries this item in its `provider_context`, and the folded
response keeps it.
This includes text, tool uses, empty thinking text with its signature, and
redacted thinking data. An assistant message with this context replays those
blocks exactly, with their original order and message boundary.
Thinking stays separate from visible assistant text. Stream consumers receive
thinking text as `ReasoningDelta` events. A call with a response schema
emits text deltas like any other call, and its finish event carries the
validated structured output. The tool runtime removes private replay context
from logger copies while retaining it in conversation history.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{Effort, ModelCallControls, ModelSpec};
use ai_models_anthropic::{
    AnthropicModel, CLAUDE_OPUS_5, CLAUDE_SONNET_5, CUSTOM_DEFAULTS, PROVIDER_ID, known_models,
};
use json_http::ReqwestJsonHttpClient;

fn build_model() -> AnthropicModel {
    AnthropicModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        CLAUDE_SONNET_5,
        "anthropic-demo",
    )
}

fn build_max_effort_model() -> (AnthropicModel, ModelCallControls) {
    let model = AnthropicModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        CLAUDE_OPUS_5,
        "anthropic-demo",
    );
    let mut controls = ModelCallControls::default();
    controls.generation.effort = Some(Effort::Max);
    (model, controls)
}

fn build_unlisted_model() -> AnthropicModel {
    AnthropicModel::new(
        Arc::new(ReqwestJsonHttpClient::new()),
        ModelSpec::custom(PROVIDER_ID, "claude-preview", &CUSTOM_DEFAULTS),
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

- `src/anthropic/mod.rs` - `Model::stream` implementation and request
  dispatch
- `src/catalog.rs` - Anthropic provider id, model spec constants, `CATALOG`,
  and custom defaults
- `src/anthropic/request.rs` - maps the resolved controls and the effort to
  the Messages request
- `src/anthropic/request_types.rs` - typed Messages request bodies
- `src/anthropic/response.rs` - typed terminal metadata and its validation
- `src/anthropic/stream/` - typed Messages SSE events, content block state,
  model event mapping, and stream failure mapping

### Related Docs

- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/live-model-api-tests.md`](../../docs/protocol/live-model-api-tests.md)
- [Anthropic model overview](https://platform.claude.com/docs/en/about-claude/models/overview)
- [Anthropic thinking and replay](https://platform.claude.com/docs/en/build-with-claude/extended-thinking)
- [Anthropic streaming messages](https://platform.claude.com/docs/en/build-with-claude/streaming)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
