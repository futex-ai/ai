# Agent Loop Protocol

Status: Approved, not implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

When implemented, this protocol replaces the turn-loop description in the
`ai-tool-calling` README. The output window rules stay in
[tool-output-management.md](tool-output-management.md).

This protocol defines turns and steps over a caller-owned conversation, and
agent events. It also defines parallel tool calls, outcomes, observers,
checkpoints, the final-answer schema, and when the loop offers the output
reader.

## Terms

- **Turn:** one call of `run`. It ends with an outcome or an error.
- **Step:** one model call and the tool calls of its response.
- **Completed call:** a tool call whose output or tool error the loop
  received.
- **Agent event:** one item of the event stream of a turn.
- **Outcome:** the result of a turn that ends without an error: completed,
  truncated, or capped.
- **Observer:** a callback that receives log copies during a turn.

## Agent and agent request

- An agent holds a `Model`, its tools, an observer, an output store, the
  output policy, and a tool-call limit.
- Building an agent fails when two tools declare the same name, or when a
  tool declares the reserved name `tool_output_read`.
- The caller can replace the tools or the output store between turns. A new
  tool set follows the same build rules.
- An agent request holds the system prompt, the call controls, an optional
  step limit, and an optional final-answer schema.

## Turns and steps

- `run(conversation, request)` runs one turn over the conversation of the
  caller and returns a stream of agent events.
- The caller owns the conversation. The loop reads it and appends to it. It
  never removes or changes a message.
- Decision: the caller owns the conversation and passes it to `run`. The
  rejected alternative was a conversation inside the runtime with mutation
  methods such as `replace_conversation` and `push_user_message`, which hid
  the messages that a turn added.
- The loop appends no user message. The caller appends the user message
  before it calls `run`.
- Each model call sends the system prompt, the conversation, the tool
  definitions, and the call controls of the request unchanged.
- When the step limit is reached before a model call, the turn ends as
  capped without that call.
- The loop runs the tool calls of the last allowed step before the turn ends
  as capped.
- A request without a step limit has no cap.
- The loop appends the assistant message of a step directly after the
  response checkpoint, before any tool call of the step starts.
- The loop appends no assistant message that has no parts. It appends no
  assistant message for a response that finishes with the reason filtered
  or other.
- The loop appends the tool result of a call when the call completes. A
  result waits only while an earlier call of the step still runs, so that
  the results keep call order among the completed calls, whatever the
  order in which the calls finish.
- Dropping the event stream cancels the turn and the running tool calls.
- A turn can stop with an error, such as a checkpoint error, or with a
  dropped event stream. The loop appends each waiting result before the
  error returns or the drop ends. The conversation then holds every
  completed tool result.
- The loop appends synchronously, so that a dropped event stream also keeps
  every completed tool result.
- The final step of a stopped turn can therefore be incomplete. A tool call
  that did not complete has no tool result.
- Before the next turn, the host closes each tool call that has no tool
  result. It appends a tool result for that call.
- Decision: a stopped turn keeps every completed tool result. The rejected
  alternative appended whole steps only, which lost completed tool actions.

## Agent events

| Event | Meaning |
| --- | --- |
| Model | A model event of the current step, unchanged. Fallback events are model events. |
| Response | The response of a step, after the checkpoints, before any tool call of the step starts. |
| Tool started | A tool call starts, with its call id and tool name. |
| Tool finished | A tool call completes, with its call id and its tool execution record or its tool error. |
| Step finished | The step is complete, with the messages that the step appended. |
| Finished | The turn ended with its outcome. This is the last event. |

- In each step, the model events come first, then the response event, then
  the tool events, then the step finished event.
- Tool events follow the order in which the calls start and finish. With
  parallel calls, they interleave.
- The tool execution record of a successful call holds the call id, the
  operation id, the tool name, the output id of a stored output, the raw
  output, and the model-visible output.
  [Tool output management](tool-output-management.md) defines the output
  fields and requires the record for current-run consumers.
- The step finished event carries the messages that the step appended, in
  conversation order. A host can therefore save the conversation step by
  step while the turn runs.
- The step finished event tells the host that a step is complete. When a
  turn stops before the step finished event of its last step, that step
  can be incomplete.
- After an error, the stream returns no further event.
- [Model stream](model-stream.md) defines the model events and the rule that
  a fallback event voids the events of the failed attempt.

## Outcomes

| Outcome | When | Carries |
| --- | --- | --- |
| Completed | A step finishes with the reason stop | The latest non-empty assistant text, the structured output, and the steps taken |
| Truncated | A step finishes with the reason truncated | The latest non-empty assistant text and the steps taken |
| Capped | The step limit is reached | The latest non-empty assistant text, the steps taken, and the step limit |

- A turn never reports a truncated answer as completed.
- A truncated step runs no tool call.
- Decision: truncation is its own outcome, so that every caller must decide
  what to do with a cut-off answer. The rejected alternatives were a finish
  reason on the completed outcome, which callers can ignore, and an
  automatic continuation.
- A step that finishes with the reason filtered ends the turn with an error
  that says the provider filtered the response.
- A step that finishes with the reason other ends the turn with an error
  that carries the raw finish value.
- The latest non-empty assistant text is the text of the latest step whose
  text is not blank.

## Tool calls

- The loop runs the tool calls of one response in parallel, with at most
  the tool-call limit of the agent running at a time. The default limit is
  four.
- The loop starts the calls in call order.
- The loop appends exactly one tool result for each completed call. The
  turns and steps section gives the order and the time of each append.
- A tool error becomes the tool result of its call, and the turn continues
  with the next step. Unknown tools, invalid arguments, and failed
  executions are tool errors.
- The content of an error result is
  `{"ok": false, "error": {"message": <error text>}}`.
- Each tool receives the operation id of its call, or the call id when the
  call has no operation id.
- A successful output becomes the result that
  [tool output management](tool-output-management.md) defines.
- A `TypedTool` declares a Rust argument type. Its definition carries the
  JSON Schema of that type as its input schema.
- Arguments that do not decode into the argument type fail with the invalid
  arguments tool error before the tool runs.
- The loop serializes the output of a `TypedTool` to JSON.
- A tool definition can carry an activity verb, and a tool can name the
  group of each of its tools. The loop gives both to observers.

## Output reader

- A model call offers `tool_output_read` only when the current output store
  holds a stored output.
- The loop runs a `tool_output_read` call also when the model call did not
  offer the reader.
- [Tool output management](tool-output-management.md) defines the reader,
  its envelopes, and its errors.

## Final-answer schema

- When the request has a final-answer schema, every model call of the turn
  sends it as the response schema.
- The completed outcome carries the structured output that matches the
  schema.
- When the final answer does not match the schema, the model call fails
  with the invalid output kind. The turn ends with that error, unless a
  fallback policy moved to another model.

## Observers

- An observer receives a log copy of each model call with its response or
  error and its latency.
- An observer receives each tool call with its tool group, its
  model-visible result or its error, and its latency.
- An observer receives each start and finish of a tool call with its
  activity verb, and the outcome of the turn.
- An observer never receives provider state or raw tool output.
- An observer error goes to `tracing` as a warning. The turn continues as if
  the callback succeeded.
- Decision: an observer error never fails a turn. The rejected alternative
  failed the turn, which can waste a model call or lose the record of a tool
  side effect.

## Checkpoints

- The loop calls the checkpoint of the caller at these points:
  - Before each model call.
  - After each response.
  - Before the tool calls of a step start.
  - After each tool call finishes.
- The response checkpoint can change the response before the tools run, for
  example to set operation ids.
- A checkpoint error ends the turn with that error. The loop starts no new
  tool call after the error, and it ends the turn when the running calls
  finish.

## Errors

- A model error ends the turn with that error.
- A checkpoint error, a filtered finish, an unknown finish, and an internal
  failure end the turn with an error.
- Tool errors and observer errors never end the turn.
