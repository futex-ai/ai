# Model Completion Streaming Protocol

Status: approved on 2026-08-22; JSON HTTP foundation implemented, provider
migration pending.

## Purpose

Define internal SSE streaming for model completions so long reasoning and
generation calls are governed by stream liveness rather than a short total HTTP
timeout. The public `ai_interface::Model` contract remains buffered:
`complete(&ModelRequest)` returns one complete `ModelResponse`.

## Scope

This protocol covers completion calls for Anthropic, OpenAI Responses, Google,
DeepSeek, Kimi, MiniMax, QwenCloud, and synchronous xAI. Each adapter consumes
its provider stream, accumulates native deltas, and returns the same normalized
response that its buffered parser would return.

The following remain buffered:

- audio transcription, image generation, and video generation;
- xAI deferred submission and polling;
- non-completion JSON HTTP calls; and
- `ai-mcp`, whose existing SSE transport remains independent.

Incremental deltas are not exposed to workspace consumers in this version.
OpenAI background Responses, stream reattachment, and downstream Firna changes
are also out of scope.

## Ownership And Compatibility

- `ai-interface` owns the unchanged model request/response boundary and the
  new normalized interruption error.
- `json-http` owns SSE framing, stream opening, connect/idle/deadline
  enforcement, and transport progress errors. It does not inspect provider
  event JSON.
- `ai-models-core` owns shared stream-error classification and the pure
  OpenAI-compatible chat-completions accumulator.
- Each provider adapter owns its stream request fields, native event DTOs,
  terminal conditions, provider errors, and conversion through its existing
  response mapper.
- Retry and fallback wrappers continue to own retry and lane-selection policy.

`JsonHttpTransport::execute_sse` must have a default implementation returning
`Error::SseUnsupported`. Existing third-party transport implementations then
continue to compile, while completion calls clearly fail until that transport
adds streaming support.

## Timeout Contract

| Boundary | Default | Meaning |
| --- | ---: | --- |
| Reqwest connect timeout | 10 seconds | TCP/TLS/proxy connection establishment for buffered and streaming calls |
| Buffered JSON HTTP timeout | 600 seconds | Total reqwest request timeout for endpoints that remain buffered |
| Completion stream idle timeout | 120 seconds | Maximum wait for the next complete SSE event |
| Completion stream overall deadline | 3,600 seconds | Total stream open and consumption time |

`JsonHttpRequest::timeout` is the overall transport deadline. A new optional
`idle_timeout` is used only by `execute_sse`; buffered execution ignores it.
Provider completion adapters set the streaming defaults above. An explicit
`ModelRequest::controls.total_timeout` replaces the 3,600-second provider
default for that invocation. Direct `json-http` callers may also tighten either
request duration.

The streaming reqwest path must not call reqwest's per-request `timeout`, which
would terminate an otherwise healthy open response. It applies the overall
deadline across stream opening and every read, and applies the idle timeout
while awaiting the next decoded event. The overall deadline is never reset.
Ping events count as events for idle timing and are otherwise ignored.

## JSON HTTP SSE Contract

`json-http` exposes a pure incremental decoder and these public concepts:

- `JsonHttpSseEvent { event: Option<String>, data: String }`;
- a mutable `JsonHttpSseStream` trait whose async `next` operation returns the
  next event or end-of-stream; and
- `JsonHttpRequestBuilder::send_sse`, which applies auth and serialization and
  returns the boxed stream from `JsonHttpTransport::execute_sse`.

`execute_sse` returns a typed `HttpStatus { status, body }` error for a
non-success open. It reads at most 64 KiB, parses valid JSON as a `Value`, and
otherwise retains the bounded text as a JSON string so adapters can call
`classify_json_http_error`. Status is evaluated before content type. A
successful response is accepted only when its media type is
`text/event-stream`, including parameterized forms; any other successful
content type is a typed transport error. `SseUnsupported` is a configuration
error, not a transient provider error.

The decoder accepts arbitrary byte splits, including splits within UTF-8,
lines, CRLF pairs, and event boundaries. Its framing behavior matches the
[AI MCP protocol](ai-mcp.md) and the completed
[line-ending](../../plans/mcp-sse-line-endings.md) and
[colonless-data](../../plans/mcp-sse-colonless-data.md) compatibility work:

- CRLF, standalone CR, and LF are valid line endings in any combination;
- the first colon separates field name and value, and one optional leading
  space is removed from the value;
- a colonless field has an empty value;
- every `data` value is joined with the next by `\n`, including an empty value
  from colonless `data`;
- `event` sets the optional event name; comments and `id`, `retry`, and unknown
  fields are ignored;
- a blank line dispatches an event when at least one `data` field was seen;
  and
- a final pending data event is dispatched at EOF.

Malformed UTF-8 or framing returns a typed decoder error. The decoder never
parses provider JSON and never interprets the `[DONE]` sentinel.

The reqwest implementation wraps response byte reads with deadline and idle
timers, counts successfully emitted events, and reports:

- `Error::IdleTimeout { idle, events_received }`;
- `Error::DeadlineExceeded { timeout, events_received }`; and
- transport or decode failures together with provider-side event progress.

## Model Error And Wrapper Contract

`ModelError::Interrupted { provider, model_id, message }` means a completion
stream failed after at least one event had been consumed and no complete
`ModelResponse` can be returned. Its constructor and public docs must retain
provider and model identity.

Classification follows these rules:

1. HTTP non-success responses retain existing status/body classification,
   including rate-limit, context-limit, transient, and terminal provider
   errors.
2. A connection, timeout, EOF-before-terminal, transport, content-type, or
   decode failure before the first event is `TransientProvider`.
3. The same failure after one or more events is `Interrupted` so the same
   generation is not blindly resubmitted and billed again.
4. A native provider error event uses the provider's existing classification
   when no earlier event was consumed; after earlier progress it is
   `Interrupted`.

`SseUnsupported` is the exception to progress classification: provider
adapters map it to `ModelError::Internal` because retry cannot add a missing
transport capability.

`RetryingModel` continues retrying only `TransientProvider`, so it never
retries `Interrupted`. `MultiModel` continues its fall-through-on-any-error
policy and therefore may try the next lane after an interruption. Its README
must warn that this can bill both the interrupted provider and the fallback.

## Shared Chat-Completions Accumulator

DeepSeek, Kimi, MiniMax, QwenCloud, and synchronous xAI use one pure
`ai-models-core` accumulator for OpenAI-compatible chat-completions chunks. It:

- concatenates `choices[].delta.content` in arrival order;
- concatenates `delta.reasoning_content` independently;
- keys `delta.tool_calls[]` by tool-call `index`, retains the one-time `id` and
  function name, and concatenates function-argument fragments exactly;
- retains the last non-null terminal `finish_reason`;
- retains the final usage-bearing chunk; and
- treats an exact `data: [DONE]` value as the terminal sentinel rather than
  JSON.

The completed accumulator output must be shaped for each adapter's existing
buffered response mapper. Missing terminal state, malformed fragments,
conflicting tool-call identity, or EOF before `[DONE]` is a stream failure and
uses the progress classification above.

## Provider Mapping

| Provider | Request change | Accumulation and terminal behavior |
| --- | --- | --- |
| Anthropic | Add `"stream": true` to `/v1/messages` | Accumulate message/content block events; `message_stop` is terminal |
| OpenAI | Add `"stream": true` to `/v1/responses` | Consume events for liveness; parse the complete object in `response.completed` with the buffered mapper |
| Google | Use `:streamGenerateContent?alt=sse` | Merge `GenerateContentResponse` fragments; stream EOF after a terminal candidate is terminal |
| DeepSeek, Kimi, MiniMax, QwenCloud, xAI sync | Add `"stream": true` and request stream usage when supported | Use the shared chat-completions accumulator through `[DONE]` |

### Anthropic

`message_start` supplies input usage. `content_block_start` creates text,
thinking, or tool-use state at its content index. `content_block_delta`
appends `text_delta`, `thinking_delta`, `signature_delta`, or
`input_json_delta.partial_json`; tool input JSON is parsed only after the block
is complete. `message_delta` supplies stop reason and cumulative output usage.
`content_block_stop` closes a block, `ping` affects only liveness, and
`message_stop` finalizes. `error` follows the model error contract.

### OpenAI Responses

All `response.*` events count for liveness. Version one does not reconstruct
item deltas: `response.completed.response` is passed to the existing buffered
Responses parser. `response.failed`, `response.incomplete`, and `error` are
typed failures and apply the progress rule. EOF without one of those terminal
events is a stream failure.

### Google

Each event is a `GenerateContentResponse` fragment. For candidate zero, text
parts concatenate in order and complete `functionCall` parts append in order.
The last present `finishReason` wins, and final `usageMetadata` is retained.
Thinking and provider context must normalize identically to the buffered path.

### Chat-Completions Providers

Before enabling a provider, its current official API must be verified to emit
usage in-stream and the exact opt-in field must be recorded here. If accurate
usage is unavailable, that provider stays buffered under the 600-second
backstop; usage must never be dropped or invented. xAI deferred execution is
unchanged even when its synchronous path streams.

## Response Parity And Verification

For equivalent fixtures, streaming and buffered parsing must produce equal
`ModelResponse` values, including assistant text, structured output, tool
calls and argument JSON, thinking/reasoning replay, signatures, provider
context, finish reason, usage, provider/model identity, catalog id, and
effective thinking level.

Required deterministic coverage includes decoder framing, split chunks,
timeouts before and after progress, non-success bodies, content type, EOF,
provider keepalives/errors, every provider delta family, and response parity.
The reqwest path also requires a local streaming-server integration test.
Credentialed provider tests follow the
[live model API test protocol](live-model-api-tests.md), extended with a
streaming assertion for every enabled provider.

## Firna Revision-Bump Contract

Firna is not changed here. Before pinning the implementation revision it must:

- implement `execute_sse` in `BoundedModelHttpTransport` while preserving its
  bounded request-body rewrite;
- update URL matching because `:generateContent` does not match Google's new
  `:streamGenerateContent` endpoint;
- handle `ModelError::Interrupted`; and
- interpret its per-call timeout override as the overall stream deadline.

Until the first item lands, Firna compiles through the default
`SseUnsupported` method but streaming completions fail at runtime. The portable
request ownership rules remain defined by
[provider call controls](provider-call-controls.md).
