# ai-tool-calling

`ai-tool-calling` runs agent turns over conversations that the caller owns.
Use it to stream model events, run tools in parallel, and keep bounded tool
output in model context. Model providers and model policies remain separate.

## Responsibilities

- Run one turn from the caller's conversation and an `AgentRequest`.
- Forward model events and build responses with the shared model-response fold.
- Run tool calls in parallel with a default limit of four.
- Append assistant messages and completed tool results to the conversation.
- Give observers log copies and give checkpoints control over continuation.
- Bound tool output through an injected store and `ToolOutputPolicy`.

The host owns durable storage, principal isolation, retention, and raw-output
redaction. The host also closes unfinished tool calls before the next turn.

## What This Crate Does

`ToolCallingRuntime` implements `Agent`. It holds the model, tools, observer,
output store, output policy, and tool-call limit. It holds no conversation or
system prompt. Its constructor rejects duplicate tool names and the reserved
name `tool_output_read`. `replace_tools` validates a new catalog before it
replaces the old catalog. `replace_output_store` changes the store for future
turns. `with_tool_call_limit` accepts a nonzero limit.

`AgentRequest` supplies the system prompt, call controls, optional step limit,
and optional final-answer schema. Each model step receives the controls and
schema unchanged. A request without a step limit has no cap.

`Agent::run` borrows the caller's conversation and returns an
`AgentEventStream`. The stream emits model, response, tool-started,
tool-finished, step-finished, and finished events. Model events include the
original fallback events. A fallback voids the earlier attempt's events.
The shared `ModelResponseFold` builds the response, as `Model::complete` does.

The loop appends no user message. It appends the assistant message after the
response checkpoint and before tool execution. It appends no empty assistant
message and none for filtered or unknown finishes. Tools start in call order.
Normal result appends wait only for earlier calls that still run.
`StepFinished` carries the messages that the step appended, in conversation
order. `ToolFinished` carries the successful `ToolExecutionRecord` or tool error.

A stopped turn keeps every completed tool result. The loop publishes each
received output synchronously before it awaits output storage. Dropping the
stream flushes waiting results in call order and cancels running calls.
A dropped write keeps a bounded degraded result. A checkpoint error prevents
new calls and waits for running calls to finish. Calls that did not complete
have no result. The host must close those calls before another turn.

`RunOutcome` distinguishes three terminal states:

- `Completed` carries the latest nonblank assistant text, structured output,
  and step count.
- `Truncated` carries the latest nonblank assistant text and step count.
  The loop runs no tool call from a truncated response.
- `Capped` carries the latest nonblank assistant text, step count, and limit.
  The loop runs the tools of the last allowed step before it reports the cap.

Model errors end the turn. Filtered and unknown finishes become model errors.
A tool error becomes a tool-result message with `ok: false`, and the turn
continues. Each tool receives its operation id or its call id as an idempotency
key. The loop keeps the model's provider state for replay.

The observer uses `ai-interface::Logger`. It receives model-call log copies,
tool groups, bounded results or errors, activity verbs, call latencies, and
turn outcomes. Log copies remove provider state and reasoning text. Observer
errors produce tracing warnings and never fail a turn.
`run_with_checkpoints` accepts a `TurnCheckpoint` and a
`ModelResponseCheckpoint`. The response checkpoint can set operation ids
before the assistant message is appended.

`TypedTool` declares an argument type and a serializable output type.
`TypedToolAdapter` derives the input schema with `schemars`, decodes arguments
before execution, forwards the operation id, and serializes output to JSON.
Bad arguments return `ToolError::InvalidArguments` before the tool runs.

Successful ordinary output uses a bounded envelope:

- Small output stays inline and consumes no store budget.
- Large output is stored and returns its first UTF-8-safe byte window.
- Oversized output, exhausted budgets, and failed writes return degraded
  windows. The successful tool action remains recorded.

The loop offers `tool_output_read` only when the current store holds output.
It also dispatches a reader call that was not offered. Reads do not create
another stored output or wrap a window again.

`ToolExecutionRecord` exposes `output_id`, `raw_output`, and
`model_visible_output`. Only current-run consumers receive raw output through
`ToolFinished`. Conversation history and observers receive bounded output.
Apply redaction before output enters the store or its envelope.

Use a fresh `InMemoryToolOutputStore` for each active run. A reused agent can
replace the store between turns. Never share a store across principals.
Output ids expire with their store. Durable ids require host-owned access
control, retention, encryption, and cleanup.

The caller can supply old history that contains raw tool JSON. Do not assume
every tool message is an envelope. Read stored version 1 messages through the
composed message converter before the turn. Budget pagination steps in
`AgentRequest::max_steps`, because each reader call needs another model step.

## Quick Start

```rust
use std::sync::Arc;

use ai_interface::{ConversationMessage, MockModel, NoopLogger};
use ai_tool_calling::{
    Agent, AgentEvent, AgentRequest, InMemoryToolOutputStore,
    ToolCallingRuntime, ToolOutputPolicy,
};

async fn run_prompt() -> ai_tool_calling::Result<()> {
    let agent = ToolCallingRuntime::new(
        Arc::new(MockModel::new("mock-model")),
        Arc::new(NoopLogger),
        Vec::new(),
        Arc::new(InMemoryToolOutputStore::new()),
        ToolOutputPolicy::default(),
    )?;
    let mut conversation = vec![ConversationMessage::user("Say done")];
    let request = AgentRequest {
        system_prompt: "You are concise.".to_owned(),
        max_steps: Some(4),
        ..Default::default()
    };
    let mut events = agent.run(&mut conversation, request);
    while let Some(event) = events.next().await? {
        if let AgentEvent::Finished(outcome) = event {
            println!("{outcome:?}");
        }
    }
    Ok(())
}
```

## Development

Enable the `test-support` feature to use generated `unimock` types for the
output store and checkpoint contracts.

```sh
cargo test -p ai-tool-calling --all-features
cargo clippy -p ai-tool-calling --all-targets --all-features
cargo xtask smoke-test
```

### Key Code

- `src/agent.rs` defines `Agent`, requests, events, and the borrowed stream.
- `src/runtime.rs` builds agents and replaces tools or output stores.
- `src/turn/stream/` runs model steps, checkpoints, and outcomes.
- `src/turn/batch.rs` limits parallel tool calls and reports completion.
- `src/turn/pending.rs` keeps completed results through cancellation.
- `src/typed_tool.rs` derives schemas and adapts typed tool execution.
- `src/policy.rs` validates output size and store limits.
- `src/output_store/` defines the store contract and in-memory implementation.
- `src/tool_output.rs` prepares bounded output before an asynchronous write.
- `src/intrinsic.rs` defines the output reader and its model guidance.

### Related Docs

- [Agent loop](../../docs/protocol/agent-loop.md)
- [Tool output management](../../docs/protocol/tool-output-management.md)
- [Model stream](../../docs/protocol/model-stream.md)
- [Conversation parts](../../docs/protocol/conversation-parts.md)
- [Architecture](../../docs/architecture.md)
- [Shared contracts](../ai-interface/README.md)
