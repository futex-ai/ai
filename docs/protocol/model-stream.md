# Model Stream Protocol

Status: Approved, not implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

When implemented, this protocol replaces
[model-completion-events.md](model-completion-events.md) and
[model-completion-streaming.md](model-completion-streaming.md).

This protocol defines the single entry point of a model call. It covers the
stream method, the model events and their order, and the fold into a
response. It also covers deadlines, interruption, cancellation, and the
stream rules of the policy layers. [Conversation parts](conversation-parts.md)
defines the parts and the provider state that events carry.
[Model policies](model-policies.md) defines the error kinds.

## Terms

- **Model event:** one item that the event stream returns.
- **Attempt:** one call to one model. A fallback makes one attempt for each
  model that it tries.
- **Progress:** an attempt has progress after its adapter read the first
  event of the provider stream, also when that event produced no model
  event.
- **Part position:** the zero-based position of a part in the assistant
  message of the attempt.

## Boundary

- `Model::stream(&request)` is the one method that an implementation of
  `Model` provides.
- `stream` returns an error or an open event stream.
- The event stream is a boxed trait object with an async `next` method.
  Decision: a boxed object with `async fn next(&mut self)` works with `dyn`
  dispatch and `unimock`. The rejected alternative was a generic stream
  type, which works with neither.
- `next` returns the next model event, `None` after the finish event, or an
  error.
- After `next` returns `None` or an error, each later call returns `None`.
- Every failure before the provider stream opens comes from `stream`. Every
  later failure comes from `next`.
- `Model::complete(&request)` is a provided method. It returns the fold of
  the events of `stream`.
- No product implementation overrides `complete`. Decision: one stream
  method replaces the buffered method and the opt-in event sink. The
  rejected alternative kept both paths, and two paths drift apart.

## Model events

| Event | Fields | Meaning |
| --- | --- | --- |
| Text delta | part position, text, optional provider state | New assistant text. |
| Reasoning delta | part position, text, optional provider state | New reasoning text that the provider exposes. |
| Tool-call start | part position, call id, tool name | A tool call starts. |
| Tool-call argument delta | part position, JSON text | The next fragment of the argument text of a call. |
| Tool-call end | part position, optional provider state | The argument text of a call is complete. |
| Usage | usage | The usage of the attempt. |
| Finish | finish reason, provider id, model id, effort, optional structured output | The attempt is complete. |
| Fallback | error of the failed attempt | The attempt failed, and its events are void. |

- The event type is non-exhaustive.
- The first event of a new part uses the lowest unused part position.
- A part has the kind of its first event: text, reasoning, or tool call.
- Events of different parts can interleave.
- The text of a delta is never empty, except in a delta that carries
  provider state.
- The JSON text of an argument delta is never empty.
- An adapter emits text in generation order. It does not trim, merge, or
  rewrite text, except for the block rules of the native adapter mapping
  below and the snapshot rules of its provider doc.
- An event carries only the provider state of its own part.
- The finish event names the provider id and the provider model id of the
  attempt, and the effort that the capability check resolved.
- Only a policy layer emits a fallback event. An adapter never emits one.

## Event order

For one attempt:

1. Content events come in provider generation order. Content events are
   text deltas, reasoning deltas, and tool-call events.
2. Each tool-call part has one start event, then its argument deltas, then
   one end event.
3. Every tool-call part ends before the usage event.
4. A successful attempt emits exactly one usage event, then exactly one
   finish event.
5. After the finish event, `next` returns `None`.

A failed attempt emits no usage event and no finish event.

## Finish rules

- A consumer acts on a tool call only after the finish event, and only when
  the finish reason is tool calls.
- A stream finishes with the reason tool calls only when it emitted at least
  one complete tool call. Otherwise `next` fails with the invalid output
  kind.
- When the finish reason is tool calls, the argument text of each tool call
  is one JSON value. Otherwise `next` fails with the invalid output kind.
- When the request has a response schema and the finish reason is stop, the
  message text is JSON that matches the schema. The finish event carries the
  parsed value. Otherwise `next` fails with the invalid output kind.
- These checks complete before the usage event, so that a policy layer sees
  the failure.
- A call with a response schema emits text deltas like any other call.
  Decision: the fold builds the response from the events. The rejected
  alternative suppressed the events of schema calls.
- The finish reasons are stop, tool calls, truncated, filtered, and other
  with the raw provider value. A missing provider value is other with the
  value `missing`.

## The fold

`complete` reads every event and returns a `ModelResponse`:

- The assistant message has one part for each part position, in position
  order.
- A text part or a reasoning part holds the concatenation of the text of
  its deltas.
- A tool-call part holds the call id, the tool name, and the JSON value of
  its argument text.
- Each part carries the last provider state that an event of the part
  carried.
- The fold keeps tool-call parts only when the finish reason is tool calls.
- The response takes the usage from the usage event. It takes the finish
  reason, the provider id, the model id, the effort, and the structured
  output from the finish event.
- A fallback event removes every part that the fold built since the start
  of the failed attempt. It adds the failed attempt to the attempt list of
  the response.
- An error from `stream` or `next` is the error of `complete`.
- When the events break the event order, `complete` fails with the
  internal kind.
- The text of a response is the concatenation of its text parts.

## Deadlines

| Deadline | Default | Meaning |
| --- | ---: | --- |
| Stream idle timeout | 120 s | The longest wait for the next provider stream event |
| Stream overall deadline | 3,600 s | The total time to open and read the provider stream |

- The total timeout control of a request replaces the overall deadline for
  that call. It does not change the idle timeout.
- A provider config can set other defaults, as the
  [provider registry protocol](provider-registry.md) says.
- The overall deadline covers the open and every read. It never restarts.
- A keepalive event of the provider counts for the idle timeout and
  produces no model event.
- A deferred lifecycle applies the total timeout across its submission,
  waits, and retrievals, as
  [provider call controls](provider-call-controls.md) says.
- A deferred lifecycle emits every event of the call after the provider
  result arrives. Decision: the fold needs the events. The rejected
  alternative kept deferred calls silent.

## Errors and interruption

- A failure before progress has the kind of its cause, such as the kind of
  an HTTP status or of a provider error event.
- These failures have the transient kind before progress, and the
  interrupted kind after progress:
  - A transport failure or a timeout.
  - A decode failure or a wrong content type.
  - An end of stream before the terminal event of the provider.
- A provider error event after progress has the interrupted kind.
- A transport that cannot open a stream gives the internal kind.
- A consumer can receive the interrupted kind before it receives any model
  event.
- Decision: no layer repeats an interrupted attempt on the same model,
  because the provider can bill the partial generation twice. A move to
  another model after an interruption follows the fallback policy.

## Cancellation

- Dropping an event stream before `None` cancels the call and closes the
  provider connection.
- A dropped stream reports no error and emits no event.
- A policy layer releases what it holds when its stream is dropped.

## Policy layers on the stream

- A policy layer returns an event stream that keeps the event order.
- It forwards the events of the inner stream in order and without change.
  The pricing layer adds prices to the usage event. The fallback layer adds
  fallback events.
- The retry layer opens the inner stream again only before the inner stream
  returned its first event. A consumer never sees an event of a retried
  attempt.
- The concurrency layer holds its permit from the call of `stream` until its
  stream returns `None`, returns an error, or is dropped.
- When an attempt fails and the fallback policy moves on, the fallback layer
  emits one fallback event, then the events of the next attempt.
- When models fail before their streams open and a later model opens its
  stream, the stream starts with one fallback event for each failed model.
- When every model fails before a stream opens, `stream` returns the error
  of the last attempt.
- When the last attempt fails after its stream opened, `next` returns the
  error of that attempt.
- A consumer that receives a fallback event discards every event since the
  start of the stream or since the previous fallback event.
- [Model policies](model-policies.md) says when each layer acts.

## Native adapter mapping

| Adapter | Content events from | Terminal provider event | Provider state on |
| --- | --- | --- | --- |
| Anthropic Messages | text, thinking, and tool-use content blocks | `message_stop` | reasoning parts: thinking and redacted-thinking blocks with signatures |
| OpenAI Responses | output text and refusal text, reasoning summary or reasoning text, and function-call items | `response.completed` or `response.incomplete` | reasoning parts: item id and encrypted content. Text parts: message phase. Tool-call parts: item id |
| Google Gemini | candidate-zero parts: text, thought text, and function calls | end of stream after a finish reason or a prompt block | the part that carries a thought signature |

- Anthropic and Google: a text block that holds only whitespace adds no
  text. The first text delta of each later text block starts with a
  newline, so that the response text joins the blocks with newlines.
- Anthropic: `message_start` gives the input usage, and `message_delta`
  gives the stop reason and the output usage. A `ping` event only keeps the
  stream alive. Unknown events and unknown deltas produce no model event.
- Anthropic: a block that starts and does not stop before `message_stop` is
  a stream failure under the progress rule. A tool-use block without
  argument fragments has the input of its start as its argument text.
- OpenAI: output text and refusal text join without a separator.
- OpenAI: `response.failed` and `error` events are failures under the
  progress rule. Before progress, a rate-limit code has the rate limited
  kind, a server or overload code has the transient kind, and another code
  has the invalid request kind.
- Google: the first text part of a chunk continues the last text part of
  the previous chunk when both are thought text or both are not. Separate
  text parts of one chunk are separate blocks.
- Google: one function-call part gives a start event, one argument delta,
  and an end event. A call without arguments has the argument text `{}`.
- Google: the last finish reason and the last usage of the stream are those
  of the attempt.
- The [dialect protocol](chat-completions-dialects.md) gives the mapping of
  the Chat Completions engine.

## SSE transport

- An SSE open with a non-success status opens no stream. Its error carries
  the status and at most 64 KiB of the body. The body is JSON when it parses
  as JSON, and a JSON string otherwise.
- The status is checked before the content type. A success opens a stream
  only with the media type `text/event-stream`, with or without parameters.
  Another media type fails the open.
- The decoder accepts any split of the bytes, also inside a UTF-8 character
  or a line ending.
- CRLF, CR, and LF are line endings, in any mix.
- The first colon separates the field name from its value. One leading
  space of the value is removed. A field without a colon has an empty value.
- The `data` values of one event are joined with `\n`. `event` sets the
  event name. Comments, `id`, `retry`, and unknown fields are ignored.
- A blank line dispatches an event when the event has at least one `data`
  field. Pending data is dispatched at the end of the stream.
- Malformed UTF-8 is a decode error. The decoder never parses JSON and never
  reads the `[DONE]` value.
