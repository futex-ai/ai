# Tool Output Management Protocol

## Purpose

Every successful tool call must produce a bounded, identifiable result without
requiring individual tools to implement output pagination. Tool implementations
continue to return normal JSON through `ToolResult<Value>`; the shared
tool-calling runtime serializes, stores, envelopes, and pages those values.

This mechanism is the final model-context safety boundary. Tools should still
use provider or source pagination when it avoids fetching unnecessary data.

## Status

This contract is planned and is not implemented on the current branch. Until
the [Universal Tool Output Management plan](../../plans/universal-tool-output-management.md)
is complete, `ai-tool-calling` continues to put the raw JSON value directly
into retained tool messages.

## Ownership

- `ai-interface` owns the model-visible output envelope, output identifier,
  read request, and window DTOs.
- `ai-tool-calling` owns output policy, store integration, automatic output
  wrapping, the in-memory store, and the intrinsic `tool_output_read` tool.
- The embedding runtime owns the lifetime of the injected output store and any
  persistence, encryption, authorization, or redaction applied by a custom
  store.
- Individual `Tool` implementations return their schema-bound JSON and do not
  generate output ids, choose windows, or implement `tool_output_read`.

## Terminology

- **Raw output:** the `serde_json::Value` returned by the tool implementation.
- **Serialized output:** the compact UTF-8 JSON representation produced from
  the raw output.
- **Model-visible output:** the inline or window envelope appended as the tool
  message and sent to log callbacks.
- **Store scope:** the lifetime in which an output id can be resolved. The
  default in-memory scope is the lifetime of its store instance.

## Output Policy

The default policy is:

| Limit | Default |
| --- | ---: |
| Inline model-visible output | 20,000 bytes |
| Maximum bytes requested by one read | 20,000 bytes |
| Maximum serialized size of one stored output | 1,048,576 bytes |
| Maximum serialized output stored per runtime | 16,777,216 bytes |

All four limits are configurable when the runtime is composed. Configuration
must reject zero values, an inline or read limit larger than the per-output
limit, and a per-output limit larger than the aggregate limit.

All limits are serialized UTF-8 byte counts, not token counts; multibyte
content yields fewer characters per window.

Only windowed outputs are stored. Inline-sized outputs are never written to
the store and never consume the aggregate budget, because the model already
holds their complete value. The aggregate budget counts the full serialized
size of every stored output, and a write that would exceed the per-output or
aggregate limit fails atomically. V1 does not evict stored outputs because an
earlier id may still be in model context; after aggregate exhaustion, later
large outputs degrade as described below.

## Successful Tool Output Flow

For every successful ordinary tool call, the runtime must:

1. Keep the raw `Value` available to current-run internal consumers.
2. Serialize it once as compact UTF-8 JSON and measure its byte length.
3. Return an inline envelope, without storing, when the serialized output is
   at most the inline limit.
4. Otherwise reserve its bytes against the aggregate budget and write the
   complete serialized output to the injected `ToolOutputStore`, receiving an
   opaque `ToolOutputId`.
5. Build the first UTF-8-safe window carrying that id, or a degraded window
   when the output cannot be stored.
6. Append exactly one model-visible envelope per provider tool-call id to
   conversation state and send only that envelope to normal tool-call logging.

An output id uses the `toolout_` prefix followed by a lowercase hyphenated
UUIDv7. The Rust API represents it with a newtype rather than a raw string.
Callers must treat the complete value as opaque and must not derive meaning
from its timestamp or other bits.

### Inline Envelope

Serialized output at most the inline limit is included complete. Inline
envelopes carry no output id and cannot be read back through
`tool_output_read`:

```json
{
  "type": "tool_output",
  "tool_name": "memory_read",
  "output": { "entries": [] },
  "total_bytes": 14,
  "truncated": false
}
```

### Window Envelope

Larger output is represented by a substring of its serialized JSON:

```json
{
  "type": "tool_output_window",
  "output_id": "toolout_018f...",
  "tool_name": "slack_search_messages",
  "offset": 0,
  "content": "{\"messages\":[...",
  "returned_bytes": 20000,
  "total_bytes": 87342,
  "truncated": true,
  "next_offset": 20000
}
```

`content` is a UTF-8 substring of serialized JSON, not an independently
parseable JSON value. All offsets and lengths are serialized UTF-8 byte counts.
`truncated` means bytes remain after this window. A truncated window carries
exactly one of `next_offset` (the remainder is readable) or
`remainder_unavailable` (it is not); a non-truncated window carries neither.
`output_id` is present exactly when `remainder_unavailable` is absent.

### Degraded Window Envelope

A successful output that cannot be stored must not be discarded: the tool
already executed, and dropping its result invites the model to retry
side-effecting calls. The runtime instead appends a degraded first window
containing the first UTF-8-safe inline-limit bytes of serialized output, with
no `output_id`, no `next_offset`, `truncated: true`, and a
`remainder_unavailable` reason:

- `output_too_large`: serialized output exceeds the per-output limit.
- `budget_exhausted`: storing it would exceed the aggregate budget.
- `store_unavailable`: the store write failed.

Degraded windows occur only as the initial dispatch envelope, never from
reads. A degraded envelope is a successful tool result, not a turn error;
`step()` and `run()` proceed normally. The runtime emits a `tracing`
diagnostic so hosts can observe size and budget pressure. Failed store writes
release any aggregate reservation and never leave a readable partial output.

## Intrinsic Output Reader

`ai-tool-calling` always adds this reserved model-visible tool:

```text
tool_output_read(output_id, offset?, length?)
```

- `output_id` is required.
- `offset` defaults to `0` and must be no greater than `total_bytes`.
- `length` defaults to 20,000 and must be between `1` and the configured
  maximum read length.
- The offset must be a UTF-8 character boundary.
- The returned end is moved backward to the nearest UTF-8 boundary when the
  requested end splits a character.
- If no complete character fits while unread bytes remain, the read fails with
  a typed error that reports the minimum usable length.
- Reading at `offset == total_bytes` returns an empty, non-truncated window.
- Unknown, expired, and wrong-scope ids share one unavailable-output error.

The intrinsic reader is dispatched by the runtime rather than through an
injected `Tool`. A user tool may not declare the reserved `tool_output_read`
name. Reading does not write another stored output, allocate a new id, or wrap
the returned window recursively. Its execution record uses the returned window
as both its raw and model-visible result and retains the requested output id.

### Model Guidance

The reader's tool description must tell the model to read further windows only
when the task requires them and to prefer narrowing the original query at its
source. Each read consumes one model round, so hosts sizing `max_steps` must
budget for pagination rounds. The unavailable-output error text must state
that the output is no longer available and that the original tool call itself
succeeded; it must not imply the original call failed. It must advise
re-running the original tool only when that tool is read-only or otherwise
safe to repeat, and confirming with the user before repeating a
side-effecting call. Tail and search read modes are out of scope for V1.

## Raw And Model-Visible Representations

`ToolExecutionRecord` must expose these fields explicitly:

- `output_id` (present only for stored outputs)
- `raw_output`
- `model_visible_output`

Ordinary tool consumers such as image-context selection, handled
`{ "ok": false }` detection, event linking, and other current-run orchestration
inspect `raw_output`. Conversation history, provider requests, normal logging,
durable journals, public chat, and audit metadata receive
`model_visible_output` only.

`ToolCallLogResult::Success` therefore carries the bounded model-visible value,
not the hidden raw value. After a run ends the raw output is gone everywhere:
normal logs never contain more than the bounded envelope. A host that needs
raw capture for debugging or audit must take it from step execution records or
a custom store, under its own retention and redaction policy.

Redaction must be applied to raw output before the store write and envelope
construction. Window `content` is an opaque byte range of serialized JSON and
can never be redacted after the fact.

## Failure Behavior

- A tool-boundary error remains a normal tool error, receives no output id, and
  is never stored.
- Oversized, over-budget, and store-failed successful outputs degrade as
  described above instead of becoming errors.
- Invalid read input, unavailable ids, and store read failures fail
  `tool_output_read` with typed errors; `run()` treats them as recoverable
  tool failures and continues the model loop.
- Model-visible errors may include safe limits and byte counts but must not
  include raw output, storage internals, or debug/source details.

## Lifetime And Persistence

The default `InMemoryToolOutputStore` is ephemeral. Embedding runtimes must
create a fresh instance for each active agent run and drop it when that run
ends. Its ids do not survive a later wake, another thread, a restarted process,
or a different store instance. Sharing one store across runs couples their
budgets and leaks output ids between runs; a store must never be shared across
runs that belong to different principals. A host that reuses one runtime
across successive runs must swap in a fresh store at the run boundary through
the runtime's store-replacement API, which makes ids from the replaced store
unavailable.

Persisted window envelopes therefore advertise reads that fail after resume.
Hosts should avoid suspending a run while the model is mid-pagination, and may
rewrite stale envelopes (dropping `next_offset`) before replaying persisted
history into a new run.

Durable ids are not part of V1. A durable custom store requires host-owned
authorization, retention, encryption, redaction, and cleanup policy before its
ids may be replayed across runs.

## Consumer Migration

Universal envelopes are intentionally a breaking change for consumers that
currently inspect fields at the top level of conversation tool messages or log
results. Such consumers must use `raw_output` during the current run or the
inline envelope's `output` field when only bounded model-visible data is
available.

Conversations persisted before adoption contain raw tool JSON while newer
messages carry envelopes, and `replace_conversation` accepts arbitrary
content, so consumers must not assume every tool message is an envelope. Old
history is not migrated.

App runtimes that already implement private output windowing must remove that
second envelope and reader in the same change that adopts this protocol, so
app output is never wrapped twice. App or Wasm boundaries may retain their own
raw-response limits and sizing hints, but must return raw schema-bound JSON to
`ai-tool-calling` for universal wrapping.

## Required Verification

Tests must cover inline envelopes without ids, large and exact-boundary
outputs, multibyte UTF-8 windows, degraded envelopes for each
`remainder_unavailable` reason, invalid offsets and lengths, unavailable ids,
aggregate accounting that excludes inline outputs, failed-write rollback,
reserved-name collisions, reader non-recursion, store scoping,
raw/model-visible separation, handled failure inspection, ordinary tool
errors, logger payloads, conversation replay, and a multi-window smoke test.
Tests must not assume which sibling call in a multi-call round exhausts the
aggregate budget, because that order is an implementation detail.
