# Model Completion Events Protocol

Status: approved by Cal on 2026-08-22 via the public incremental completion
events handoff.

## Purpose

Expose ordered, provider-neutral text progress while one model completion is
running, without changing the existing buffered `Model::complete` contract.
Consumers that opt in can render live assistant or reasoning text and still
receive the same terminal `ModelResponse` used by buffered callers.

This protocol builds on the provider SSE and timeout behavior defined by
[model completion streaming](model-completion-streaming.md). It exposes
normalized events, never raw SSE frames or provider event names.

## Public Boundary

`ai-interface` owns these public concepts:

- `ModelCompletionEvent`, a non-exhaustive typed enum with
  `AssistantTextDelta { delta }`, `ReasoningTextDelta { delta }`, and
  `AttemptRestarted` variants;
- `ModelCompletionEventSink`, an async, infallible, unimock-able boundary whose
  `emit` method accepts one owned event;
- `NoopModelCompletionEventSink`, for implementations that share the observing
  path with buffered callers; and
- `Model::complete_with_events(&ModelRequest, &dyn ModelCompletionEventSink)`,
  which returns the existing `ModelResult<ModelResponse>`.

`complete_with_events` has a default implementation that delegates to
`complete` and emits nothing. Existing third-party `Model` implementations
therefore continue to compile and retain buffered behavior until they opt in.
`complete` remains required and unchanged.

An implementation awaits each sink callback before emitting the next event.
Callbacks are never concurrent, and their order matches provider generation
order. A sink cannot fail the model call; implementations that need fallible
or slow delivery should enqueue promptly and own that delivery policy outside
the model boundary.

The terminal response is returned only after every event for the successful
attempt has been emitted. There is no terminal event because
`ModelResponse` remains the authoritative completion, tool-call, finish,
provider-context, and usage result.

## Event Semantics

`AssistantTextDelta` contains newly generated user-visible assistant text.
`ReasoningTextDelta` contains newly generated thinking, reasoning, or reasoning
summary text that the provider makes available. Reasoning is a distinct event
even when the provider retains equivalent text only as replay context in the
terminal response; consumers decide whether to display it.

Empty fragments must not be emitted. Text is forwarded exactly as supplied by
the provider after any provider-required cumulative-snapshot normalization.
Implementations must not combine, rewrite, or trim independently delivered
fragments merely to make events more readable.

`AttemptRestarted` means all assistant and reasoning text observed for the
current attempt must be discarded before applying later deltas. It carries no
provider details because it describes the logical completion rather than a
provider transport.

For every successful plain-text completion, concatenating
`AssistantTextDelta::delta` values after the most recent
`AttemptRestarted` must equal `ModelResponse::assistant_message`. This parity
rule applies through a consumer holding only `DynModel`, including every
wrapper stack. Tool-call-only responses may emit no assistant text.

Schema-constrained requests with `ModelRequest::response_schema` set emit no
events in version one. Suppression avoids exposing incomplete JSON as though it
were validated output. The terminal structured response is unchanged.

## Errors, Retry, And Fallback

Timeout and error classification remain defined by the internal streaming
protocol. A provider failure after stream progress can return
`ModelError::Interrupted` after already-emitted public events; no terminal
response or parity guarantee exists for that failed attempt.

`RetryingModel` calls `complete_with_events` for every attempt and retries only
`TransientProvider`. Existing progress classification guarantees that a
transient failure occurred before any provider event was consumed, so a failed
retry attempt emits no public event. Retry does not emit `AttemptRestarted`.

`MultiModel` forwards each lane's events in order. After a lane fails, it emits
one `AttemptRestarted` immediately before starting the next lane only when an
assistant or reasoning delta has been forwarded since the most recent restart.
A failure before public text, or failure of the final configured lane, emits no
restart. Nested fallback markers reset the outer lane's public-text tracking.

`ConcurrencyLimitedModel` holds its permit for the complete event-observing
call and passes the sink through unchanged. `UsagePricingModel` also passes the
sink through and prices only the returned terminal response. Plain-text calls
through every wrapper therefore retain streaming behavior.

## Provider Mapping

| Provider | Assistant events | Reasoning events | Terminal behavior |
| --- | --- | --- | --- |
| Anthropic | `text_delta` text | `thinking_delta` text | Existing `message_stop` accumulation |
| OpenAI Responses | `response.output_text.delta` | Exposed reasoning-summary text deltas | Existing completed or incomplete response object |
| Google | Non-thought candidate text fragments | Candidate text fragments marked as thought | Existing merged terminal candidate or prompt block |
| DeepSeek | `delta.content` | `delta.reasoning_content` | Existing shared chat-completions accumulation |
| Kimi | `delta.content` | `delta.reasoning_content` | Existing shared chat-completions accumulation |
| QwenCloud | `delta.content` | `delta.reasoning_content` | Existing shared chat-completions accumulation |
| MiniMax | Normalized append-only content fragments | Append-only reasoning fragments when exposed | Existing shared accumulation and validated EOF rules |
| xAI synchronous | `delta.content` | `delta.reasoning_content` when exposed | Existing shared chat-completions accumulation |

Provider adapters emit from the native events they already parse and preserve
their current terminal accumulators and response mappers. Tool-call argument
deltas, usage deltas, raw error events, keepalives, and transport metadata are
not public completion events in version one.

xAI deferred submit-and-poll execution remains buffered and emits no events.
Synchronous xAI execution follows the table above. Completion mode selection,
idle and overall deadlines, and interrupted-stream classification do not
change.

## Verification

Deterministic provider tests must record events from representative streams
and assert their exact order. Every provider requires a plain-text parity test
that compares concatenated assistant deltas with the returned
`assistant_message`. Reasoning-capable fixtures must assert separate reasoning
events. Structured-output fixtures must assert suppression.

Wrapper tests must cover default no-event compatibility, retry without leaked
events, fallback before and after public text, nested restart tracking,
concurrency pass-through, and pricing pass-through. Credentialed live tests
must observe at least one assistant delta and assert parity for every provider's
synchronous streaming path. The xAI deferred probe instead asserts a normal
terminal result with no events.

## Downstream Firna Adoption

Firna is not changed in this workspace. After pinning a revision containing
this protocol it must:

1. implement `ModelCompletionEventSink` at its platform event boundary;
2. call `complete_with_events` for root burst completions and forward assistant
   deltas, reasoning deltas, and restart markers in order;
3. discard already-rendered attempt text when it receives
   `AttemptRestarted`; and
4. retain `complete` for callers that do not opt into live events.

The transport, Google URL matching, interrupted-error, and timeout adoption
steps in the internal streaming protocol remain prerequisites. The default
method keeps Firna compiling before it adopts the new entrypoint, and callers
that continue using `complete` observe no behavior change.

## Non-Goals

- Tool-call argument or usage deltas.
- Raw SSE frames, provider event names, or transport details.
- A changed timeout contract or xAI deferred lifecycle.
- Background execution, persisted streams, or stream reattachment.
