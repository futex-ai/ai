# ai-tool-calling

`ai-tool-calling` is an application-agnostic Rust crate for in-memory
tool-calling conversations. Depend on it when you need a reusable runtime that
keeps conversation state, drives model/tool rounds, bounds model-visible tool
output, and exposes explicit mutation APIs for embedding services.

## Responsibilities

- Own the in-memory `ToolCallingRuntime`, retained conversation state, and turn
  loop.
- Dispatch model-emitted tool calls through the generic `ai-interface::Tool`
  boundary.
- Own output windowing policy, output-store integration, degraded-window
  handling, and the intrinsic `tool_output_read` reader.
- Preserve provider replay context, operation ids, turn checkpoints, and public
  tool activity lifecycle logging.
- Keep durable storage, principal isolation, retention, and raw-output
  redaction policy in the embedding host.

## What This Crate Does

- Starts turns with `send(...)` or resumes from retained history with
  `resume(...)`.
- Calls an injected model with the active system prompt, conversation, and tool
  definitions.
- Always advertises the reserved intrinsic `tool_output_read` tool. Injecting a
  normal tool with that name fails during runtime construction or
  `replace_tools(...)`.
- Dispatches `tool_output_read` inside the runtime, reading byte windows
  directly from the injected `ToolOutputStore`. Offsets and lengths are bytes,
  not tokens.
- Keeps intrinsic read failures actionable and free of storage internals in
  model-visible conversation state while retaining typed error details for
  logger callbacks.
- Serializes each successful raw tool result once as compact UTF-8 JSON and
  applies `ToolOutputPolicy`:
  - At or below the inline limit, the model receives a complete `tool_output`
    envelope. The store is not touched.
  - Above the inline limit, the runtime stores the serialized output and returns
    the first UTF-8-safe `tool_output_window` with an opaque `output_id` and
    `next_offset`.
  - If storage cannot retain a successful result, the runtime returns a
    degraded `tool_output_window` with no id and a `remainder_unavailable`
    reason. The tool call is still treated as successful because side effects
    may already have happened.
- Appends only bounded envelope JSON to conversation tool messages and logger
  success payloads. Raw output remains available only through current-run
  `ToolExecutionRecord::raw_output`.
- Records `ToolExecutionRecord::output_id`,
  `ToolExecutionRecord::raw_output`, and
  `ToolExecutionRecord::model_visible_output` for successful tool executions.
  Intrinsic reads use the returned window as both raw and model-visible output.
- Appends exactly one tool-role message per provider tool-call id, including
  multi-call rounds where sibling calls fail or degrade.

Output store ownership is part of the host contract. Use a fresh
`InMemoryToolOutputStore` for each active run. Output ids expire when that
store is dropped or replaced, and a store must never be shared across runs that
belong to different principals. Hosts that reuse one `ToolCallingRuntime` for
successive runs should call `replace_output_store(...)` at the run boundary.
Durable output ids require a host-owned protected storage design with explicit
retention, access control, and deletion behavior; the in-memory store is not a
durable archive.

Hosts should apply redaction to raw tool output before the store write and
before envelope construction. Window content is an opaque byte range of the
serialized output, so it cannot be reliably redacted after pagination has
started.

Normal observability receives bounded envelopes only. After a run ends, raw
output is gone everywhere unless the host captured it from step execution
records or implemented a custom store under its own retention and redaction
policy.

Persisted history can be mixed-format. Conversations saved before adopting
output management may contain raw tool JSON, and `replace_conversation(...)`
accepts arbitrary message content. Consumers should not assume every tool-role
message is an envelope.

Avoid suspending a run mid-pagination when possible. If persisted window
envelopes become stale before replay, a host may rewrite them to a user-facing
unavailable-output explanation or rerun the original tool only when that is
safe. Size `max_steps` for pagination rounds because each `tool_output_read`
call consumes one model round.

Breaking changes from `0.2.x`:

- `ToolCallingRuntime::new(...)` now requires a `DynToolOutputStore` and a
  validated `ToolOutputPolicy`.
- `ToolCallLogResult::Success` carries `ToolOutputEnvelope`, not raw
  `serde_json::Value`.
- Conversation tool messages for successful calls contain compact envelope JSON
  instead of the raw tool JSON.
- `ToolExecutionRecord` exposes `output_id`, `raw_output`, and
  `model_visible_output`; downstream code that read the old raw `output` field
  should switch to `raw_output` for active-run inspection or
  `model_visible_output` for replay/log parity.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{ConversationMessage, MockModel, NoopLogger};
use ai_tool_calling::{
    InMemoryToolOutputStore, RunOutcome, ToolCallingRuntime, ToolOutputPolicy, Turn,
};

async fn run_prompt() -> ai_tool_calling::Result<RunOutcome> {
    let runtime = ToolCallingRuntime::new(
        "You are concise.",
        Arc::new(MockModel::new("mock-model")),
        Arc::new(NoopLogger),
        Vec::new(),
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )?;

    let mut turn = runtime.send(ConversationMessage::user("Say done"), Some(4));
    turn.run().await
}
```

For a reused runtime, swap to a fresh store before starting the next principal
or active run:

```rust
use std::sync::Arc;

use ai_tool_calling::{InMemoryToolOutputStore, ToolCallingRuntime};

fn prepare_next_run(runtime: &ToolCallingRuntime) {
    runtime.replace_output_store(Arc::new(InMemoryToolOutputStore::new()));
}
```

## Development

Downstream crates that want generated `Unimock` mock types for runtime-facing
traits should enable the `test-support` feature:

```toml
ai-tool-calling = { version = "0.3.0", features = ["test-support"] }
```

```sh
cargo test -p ai-tool-calling
cargo clippy -p ai-tool-calling --all-targets --all-features -- -D warnings
```

### Key Code

- `src/runtime.rs` - retained conversation state, runtime constructor,
  `replace_tools(...)`, and `replace_output_store(...)`.
- `src/policy.rs` - validated inline, read, per-output, and aggregate byte
  limits.
- `src/output_store/` - async `ToolOutputStore` trait, typed errors, request
  DTOs, and `InMemoryToolOutputStore`.
- `src/tool_output.rs` - envelope construction, store writes, and degraded
  window mapping.
- `src/intrinsic.rs` - reserved `tool_output_read` definition and model
  guidance.
- `src/turn/` - `step()`, `run()`, tool dispatch, execution records, and
  checkpoints.
- [`../ai-interface/README.md`](../ai-interface/README.md) - shared DTO and
  trait ownership.

### Related Docs

- [`../ai-interface/README.md`](../ai-interface/README.md)
- [`../../docs/protocol/tool-output-management.md`](../../docs/protocol/tool-output-management.md)
- [`../../plans/README.md`](../../plans/README.md)
