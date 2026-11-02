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
- Record each original content block as Anthropic provider state on its
  part, replay that state, and convert stored version 1 Anthropic items

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
  the `UnsupportedControl(ModelControl::ToolChoice)` error kind before
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
  failures after progress as the `Interrupted` error kind
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

Each content block of a response becomes one part, and the adapter attaches
the original block to that part as Anthropic provider state. A thinking or
redacted-thinking block goes on its reasoning part, a text block on its text
part, and a tool-use block on the end event of its call. This includes empty
thinking text with its signature, and redacted thinking data. When the
response has `thinking` or `redacted_thinking` blocks, an assistant message
with these parts replays every block exactly, with the original order and
message boundary. The adapter ignores the state of other providers. Own
state with an unknown version or a payload that does not decode fails before
transport with the `InvalidRequest` error kind.

Every failure carries shared error details with the adapter's provider and
configured model id. Errors retain `Retry-After` and the `request-id` response
header, including failures after a stream opens. Provider diagnostics redact
the actual credentials applied for that call. `with_clock` supplies an injected
wall clock for HTTP-date retry hints. Malformed generated tools or schema output
use `InvalidOutput`, while an invalid caller schema uses `InvalidRequest`.
Thinking stays separate from visible assistant text. Stream consumers receive
thinking text as `ReasoningDelta` events, and the folded response keeps it in
reasoning parts. A call with a response schema emits text deltas like any
other call, and its finish event carries the validated structured output.
Log copies drop the provider state and empty the reasoning text, while the
conversation keeps both.

`AnthropicLegacyConverter` converts a stored version 1
`anthropic_assistant_message` item. In a message with items from Anthropic
only, it gives one part for each block, in the original block order, with
that block as the state of the part. It keeps the
version 1 `content` as the message text, and it keeps every `tool_calls`
entry. A tool-use block without a `tool_calls` entry stays as state on an
empty reasoning part, so the converter invents no portable call. Text and
calls that no block holds come after the block parts. Their state holds no
block, so they add no native block to the request. An item without blocks
keeps its empty assistant message. Add the converter to the
`LegacyMessageConverter` of the host. In a message with items from several
providers, the composition keeps every full content item whole on one
reasoning part with its provider's id, at every item position. A read
without this converter also keeps the item whole. The adapter sends the
first whole Anthropic item as the entire content of that message, as
version 1 did. Portable text and calls then add no native blocks.

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
- `src/anthropic/request_messages.rs` - maps ordered parts to Messages
  content, and replays own block state
- `src/anthropic/replay.rs` - the private, versioned block state and its
  decoding
- `src/legacy.rs` - `AnthropicLegacyConverter`, the conversion of stored
  version 1 items
- `src/anthropic/response.rs` - typed terminal metadata and its validation
- `src/anthropic/stream/` - typed Messages SSE events, content block state,
  model event mapping, and stream failure mapping

### Related Docs

- [`../../docs/protocol/model-capabilities.md`](../../docs/protocol/model-capabilities.md)
- [`../../docs/protocol/provider-call-controls.md`](../../docs/protocol/provider-call-controls.md)
- [`../../docs/protocol/model-stream.md`](../../docs/protocol/model-stream.md)
- [`../../docs/protocol/conversation-parts.md`](../../docs/protocol/conversation-parts.md)
- [`../../docs/protocol/live-model-api-tests.md`](../../docs/protocol/live-model-api-tests.md)
- [Anthropic model overview](https://platform.claude.com/docs/en/about-claude/models/overview)
- [Anthropic thinking and replay](https://platform.claude.com/docs/en/build-with-claude/extended-thinking)
- [Anthropic streaming messages](https://platform.claude.com/docs/en/build-with-claude/streaming)
- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../json-http/README.md`](../json-http/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
