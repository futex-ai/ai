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
| Maximum serialized size of one output | 1,048,576 bytes |
| Maximum serialized output stored per runtime | 16,777,216 bytes |

All four limits are configurable when the runtime is composed. Configuration
must reject zero values, an inline or read limit larger than the per-output
limit, and a per-output limit larger than the aggregate limit.

The aggregate budget counts the full serialized size of every stored output,
including outputs returned inline. A write that would exceed the budget fails
atomically. V1 does not evict old outputs because an earlier id may still be in
model context.

## Successful Tool Output Flow

For every successful ordinary tool call, the runtime must:

1. Keep the raw `Value` available to current-run internal consumers.
2. Serialize it once as compact UTF-8 JSON and measure its byte length.
3. Reject it before storage when it exceeds the per-output limit.
4. Reserve its bytes against the aggregate budget and write the complete
   serialized output to the injected `ToolOutputStore`.
5. Receive an opaque `ToolOutputId` from the store.
6. Build either an inline envelope or the first UTF-8-safe window.
7. Append only that model-visible envelope to conversation state and send only
   that envelope to normal tool-call logging.

An output id uses the `toolout_` prefix followed by a lowercase hyphenated
UUIDv7. The Rust API represents it with a newtype rather than a raw string.
Callers must treat the complete value as opaque and must not derive meaning
from its timestamp or other bits.

### Inline Envelope

When serialized output is at most the inline limit, the complete structured
value is included while still receiving an id:

```json
{
  "type": "tool_output",
  "output_id": "toolout_018f...",
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
`truncated` means bytes remain after this window. `next_offset` is present only
when `truncated` is true.

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

## Raw And Model-Visible Representations

`ToolExecutionRecord` must expose these fields explicitly:

- `output_id`
- `raw_output`
- `model_visible_output`

Ordinary tool consumers such as image-context selection, handled
`{ "ok": false }` detection, event linking, and other current-run orchestration
inspect `raw_output`. Conversation history, provider requests, normal logging,
durable journals, public chat, and audit metadata receive
`model_visible_output` only.

`ToolCallLogResult::Success` therefore carries the bounded model-visible value,
not the hidden raw value. An embedding runtime that needs raw fields performs
that work from step execution records before discarding them.

## Failure Behavior

- A tool-boundary error remains a normal tool error, receives no output id, and
  is never stored.
- Per-output overflow, aggregate-budget exhaustion, store failure, invalid read
  input, and unavailable ids use typed output-management errors.
- A dispatch-time output-management failure appends a bounded, stable
  `{ "ok": false, "error": ... }` tool message so the model receives one
  response for every provider tool-call id.
- `step()` surfaces the typed failure to its caller. `run()` treats it as a
  recoverable tool-dispatch failure and continues the model loop, matching
  existing tool error behavior.
- Model-visible errors may include safe limits and byte counts but must not
  include raw output, storage internals, or debug/source details.
- Failed store writes release any aggregate reservation and do not leave a
  readable partial output.

## Lifetime And Persistence

The default `InMemoryToolOutputStore` is ephemeral. Embedding runtimes should
create a fresh instance for each active agent run and drop it when that run
ends. Its ids do not survive a later wake, another thread, a restarted process,
or a different store instance.

Durable ids are not part of V1. A durable custom store requires host-owned
authorization, retention, encryption, redaction, and cleanup policy before its
ids may be replayed across runs.

## Consumer Migration

Universal envelopes are intentionally a breaking change for consumers that
currently inspect fields at the top level of conversation tool messages or log
results. Such consumers must use `raw_output` during the current run or the
inline envelope's `output` field when only bounded model-visible data is
available.

App runtimes that already implement private output windowing must remove that
second envelope and reader when adopting this protocol. App or Wasm boundaries
may retain their own raw-response limits and sizing hints, but must return raw
schema-bound JSON to `ai-tool-calling` for universal wrapping.

## Required Verification

Tests must cover inline output ids, large and exact-boundary outputs, multibyte
UTF-8 windows, invalid offsets and lengths, unavailable ids, single-output and
aggregate limits, failed-write rollback, reserved-name collisions, reader
non-recursion, store scoping, raw/model-visible separation, handled failure
inspection, ordinary tool errors, logger payloads, conversation replay, and a
multi-window smoke test.
