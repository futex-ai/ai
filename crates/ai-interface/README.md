# ai-interface

`ai-interface` is the shared contract crate for AI-facing runtime boundaries in
this workspace. Depend on it when you need the common conversation, model, tool,
audio transcription, routing, logging, usage, or model-visible tool output DTOs
without taking a dependency on a stateful runtime implementation.

## Responsibilities

- Own shared DTOs for conversations, model calls, tool calls, audio
  transcription, routing, logging, and usage metering.
- Own the model-visible tool output DTOs used by universal output management:
  opaque output ids, inline envelopes, window envelopes, read requests, and
  unavailable-remainder reasons.
- Own the generic `Model`, `Tool`, `ModelRouter`, `AudioTranscriber`, and
  `Logger` trait boundaries plus their dyn aliases.
- Keep individual tools pagination-agnostic: `Tool::call()` and
  `Tool::call_with_invocation()` return raw `serde_json::Value`.
- Define logger success payloads in terms of bounded model-visible envelopes,
  leaving any full raw-output capture to the runtime that executed the tool.
- Provide built-in mocks for deterministic development and tests.

## What This Crate Does

- Defines `ConversationMessage`, `ConversationRole`, `ToolCall`, and
  `ToolDefinition`, including provider replay context on assistant messages.
- Defines `ModelRequest`, `ModelResponse`, `FinishReason`,
  `StructuredOutputSchema`, model usage DTOs, and typed model/router errors.
- Defines `ToolInvocation`, which carries the runtime operation id used as a
  tool idempotency key alongside the model-visible tool name and JSON input.
- Defines `ToolOutputEnvelope` as the model-visible success payload for tools.
  Complete small outputs serialize as tagged `tool_output` envelopes. Larger or
  degraded outputs serialize as tagged `tool_output_window` envelopes with
  byte offsets, UTF-8 content, total byte counts, and either a next byte offset
  or an unavailable-remainder reason.
- Defines `ToolOutputId` as an opaque string newtype. Callers may store and
  echo the value, but must not parse or derive meaning from `toolout_...`
  strings.
- Defines `ToolOutputReadRequest`, the argument DTO used by runtimes that expose
  the intrinsic `tool_output_read` tool.
- Defines `ToolCallLogResult::Success` with a `ToolOutputEnvelope` payload.
  Normal logs and conversation state carry only bounded model-visible output.

The raw versus model-visible boundary is intentional. A `Tool` implementation
returns the raw JSON value it produced. Runtime crates can inspect that value in
current-run execution records, but they serialize only `ToolOutputEnvelope`
values into conversation tool messages and logger success entries.

## Quick Start

```rust
use ai_interface::{
    ConversationMessage, Model, ModelRequest, MockModel, ToolOutputEnvelope, ToolOutputId,
    ToolOutputReadRequest,
};
use serde_json::json;

async fn call_model() -> ai_interface::ModelResult<String> {
    let model = MockModel::new("done");
    let response = model
        .complete(&ModelRequest {
            system_prompt: "You are concise.".to_owned(),
            messages: vec![ConversationMessage::user("Summarize the status")],
            tools: Vec::new(),
            response_schema: None,
        })
        .await?;

    Ok(response.assistant_message)
}

fn serialize_inline_tool_output() -> serde_json::Result<String> {
    let raw_output = json!({ "matches": ["alpha"] });
    let total_bytes = serde_json::to_string(&raw_output)?.len();
    let envelope = ToolOutputEnvelope::inline("search", raw_output, total_bytes);

    serde_json::to_string(&envelope)
}

fn read_next_window(output_id: &str) -> ToolOutputReadRequest {
    ToolOutputReadRequest {
        output_id: ToolOutputId::from_opaque(output_id),
        offset: Some(20_000),
        length: None,
    }
}
```

## Development

```sh
cargo test -p ai-interface
cargo clippy -p ai-interface --all-targets --all-features -- -D warnings
```

The crate is intentionally dependency-light. `serde` and `serde_json` are used
for contract DTOs, while runtime policy, storage, pagination, and intrinsic
tool dispatch live in `ai-tool-calling`.

### Key Code

- `src/messages.rs` - conversation DTOs and provider replay context.
- `src/model.rs` - model trait, request/response DTOs, finish reasons, and
  typed model errors.
- `src/router.rs` - model route request DTOs and router trait.
- `src/audio_transcriber.rs` - speech-to-text trait, request/response DTOs,
  and typed transcription errors.
- `src/tools.rs` - tool trait, tool DTOs, invocation context, and tool errors.
- `src/output/` - model-visible tool output ids, envelopes, reasons, and read
  request DTOs.
- `src/logger.rs` - logger trait, log payloads, `ToolCallLogResult`, and
  `NoopLogger`.
- `src/mock_model.rs` and `src/mock_audio_transcriber.rs` - built-in mocks for
  tests and local development.

### Related Docs

- [`../ai-tool-calling/README.md`](../ai-tool-calling/README.md)
- [`../ai-models-core/README.md`](../ai-models-core/README.md)
- [`../../docs/protocol/tool-output-management.md`](../../docs/protocol/tool-output-management.md)
- [`../../plans/README.md`](../../plans/README.md)
