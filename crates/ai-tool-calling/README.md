# ai-tool-calling

`ai-tool-calling` is an application-agnostic Rust crate for in-memory tool-calling
conversations. Depend on it when you need a reusable `send(...)->turn`
runtime that keeps conversation state, drives model/tool rounds, and exposes
explicit conversation mutation APIs.

## Responsibilities

- Own the in-memory conversation runtime and turn loop
- Keep tool dispatch generic and application-agnostic
- Expose logging hooks for successful and failed model, tool, and turn events
- Emit public-safe tool activity lifecycle callbacks before and after tool
  dispatch
- Build on the shared DTOs and traits from `ai-interface`

## What This Crate Does

- Stores retained conversation state for one runtime instance
- Starts new turns either by appending a caller message with `send(...)` or by
  resuming from an already-retained caller message with `resume(...)`
- Calls a resolved model with the active system prompt, conversation, and tool
  definitions
- Dispatches model-emitted tool calls by name
- Passes each dispatch through `ToolInvocation`, including the runtime
  operation id/idempotency key supplied by the embedding runtime or derived
  from the provider tool-call id
- Uses `ModelResponse::finish_reason` as the turn termination contract instead
  of inferring completion from whether the tool-call list is empty
- Retains `ModelResponse::provider_context` on assistant messages so
  provider-specific replay state, such as OpenAI stateless reasoning items,
  stays available for later model calls
- Records successful tool output and tool errors back into conversation state,
  always appending one tool-role message per emitted `tool_call_id` even when a
  sibling call in the same model response fails
- Emits logger callbacks for successful model calls plus failed model/tool
  operations, including debug details for operator-facing runtimes
- Resolves tool `activity_verb` metadata by tool name and reports it through
  `Logger::log_tool_activity` immediately before each tool call starts, then
  reports completion so callers can return public status to `Using`
- Resolves optional runtime tool-group metadata by tool name and includes it in
  `Logger::log_tool_call` entries for embedding runtimes that persist or render
  grouped tool activity
- Supports caller-provided turn checkpoints before model calls, after model
  responses, and between tool calls so embedding runtimes can fail closed at
  lifecycle boundaries without cancelling in-flight model or tool execution
- Supports a validated-response checkpoint hook before tool dispatch so
  embedding runtimes can persist assistant/tool-call boundaries after response
  contract validation and before side effects
- Keeps durable storage out of this crate; embedding services use response and
  tool checkpoints to persist assistant responses and tool-execution checkpoints
  in their own store
- Lets callers step one round at a time or run a turn to completion

`FinishReason::Stop` and `FinishReason::Truncated` complete a turn.
`FinishReason::ToolCalls` continues only when the response includes tool-call
payloads. Filtered, unknown, missing, or inconsistent finish reasons surface as
model errors so callers can distinguish provider truncation/filtering from a
clean assistant stop.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{ConversationMessage, ConversationRole, Model, NoopLogger, Tool};
use ai_tool_calling::{RunOutcome, ToolCallingRuntime};

fn user_message(content: &str) -> ConversationMessage {
    ConversationMessage {
        role: ConversationRole::User,
        content: content.to_owned(),
        name: None,
        tool_call_id: None,
        tool_calls: Vec::new(),
        provider_context: Vec::new(),
    }
}

fn build_runtime(
    model: Arc<dyn Model>,
    tools: Vec<Arc<dyn Tool>>,
) -> ai_tool_calling::Result<ToolCallingRuntime> {
    ToolCallingRuntime::new(
        "You are a helpful assistant.",
        model,
        Arc::new(NoopLogger),
        tools,
    )
}

async fn run_once(
    mut runtime: ToolCallingRuntime,
) -> ai_tool_calling::Result<RunOutcome> {
    runtime.push_assistant_message("Previous answer".to_owned());
    runtime.send(user_message("Check the workspace status"), Some(8)).run().await
}
```

## Development

Downstream crates that want the generated `Unimock` mock types for `Model`,
`Tool`, or `Logger` should enable the `test-support` feature:

```toml
ai-tool-calling = { version = "0.2.0", features = ["test-support"] }
```

```sh
cargo test -p ai-tool-calling
cargo clippy -p ai-tool-calling --all-targets --all-features -- -D warnings
```

### Key Code

- `src/runtime.rs` - retained conversation state and runtime mutation APIs
- `src/turn.rs` - `step()` and `run()` turn execution
- [`../ai-interface/README.md`](../ai-interface/README.md) - shared model, tool,
  logger, and conversation DTO ownership

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../../plans/README.md`](../../plans/README.md)
