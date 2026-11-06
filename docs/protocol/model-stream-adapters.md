# Model Stream Adapter Protocol

Status: Implemented.

This page is part of the [model stream protocol](model-stream.md), and it
uses the terms and the progress rule of that protocol. It gives the mapping
of each native adapter from its provider stream to model events, and the SSE
transport rules of the HTTP component.

## Native adapter mapping

| Adapter | Content events from | Terminal provider event | Provider state on |
| --- | --- | --- | --- |
| Anthropic Messages | text, thinking, and tool-use content blocks | `message_stop` | reasoning parts: thinking and redacted-thinking blocks with signatures. Text parts: the original text block. Tool-call parts: the original tool-use block |
| OpenAI Responses | output text and refusal text, reasoning summary or reasoning text, and function-call items | `response.completed` or `response.incomplete` | reasoning parts: item id, summary, and encrypted content. Text parts: message phase, or a message marker without a phase in a tool-call response. Tool-call parts: item id, call id, name, and raw argument text |
| Google Gemini | candidate-zero parts: text, thought text, and function calls | end of stream after a finish reason or a prompt block | every part of a response with a thought signature: its original Gemini parts, with signatures, empty text, and signature-only parts. Every tool-call part: its original call and its runtime call, which a message without a signature replays. First text part of an unsigned reply whose text follows a call or spans multiple blocks: a marker for replay of joined text before calls |

- Anthropic and OpenAI send `"stream": true`. Google calls
  `:streamGenerateContent` with `alt=sse`.
- Anthropic and Google: a text block that holds only whitespace adds no
  text. The first text delta of each later text block starts with a
  newline, so that the response text joins the blocks with newlines.
- Anthropic: `message_start` gives the input usage, and `message_delta`
  gives the stop reason and the output usage. A `ping` event only keeps the
  stream alive. Unknown events and unknown deltas produce no model event.
- Anthropic: a block that starts and does not stop before `message_stop` is
  a stream failure under the progress rule. A tool-use block without
  argument fragments has the input of its start as its argument text.
- Anthropic: a `signature_delta` adds no text. Its signature belongs to the
  provider state of its thinking block.
- OpenAI: output text and refusal text join without a separator.
- OpenAI: `response.incomplete` ends the attempt with its partial output.
  Its finish reason is truncated for `max_output_tokens`, filtered for
  `content_filter`, and other otherwise. The other value is the raw reason,
  or `incomplete` when the provider gives no reason.
- OpenAI: `response.failed` and `error` events are failures under the
  progress rule. Before progress, their codes follow the
  [shared error kinds rule](model-policies.md#error-kinds).
- Google: the first text part of a chunk continues the last text part of
  the previous chunk when both are thought text or both are not. Separate
  text parts of one chunk are separate blocks.
- Google: one function-call part gives a start event, one argument delta,
  and an end event. A call without arguments has the argument text `{}`.
- Google: the last finish reason and the last usage of the stream are those
  of the attempt.
- Google: a thought signature never changes the text of a delta.
- The [dialect protocol](chat-completions-dialects.md) gives the mapping of
  the Chat Completions engine.

## SSE transport

- An SSE open with a non-success status opens no stream. Its error carries
  the status and at most 64 KiB of the body. The body is JSON when it parses
  as JSON, and a JSON string otherwise.
- The status is checked before the content type. A success opens a stream
  only with the media type `text/event-stream`, with or without parameters.
  Another media type fails the open.
- A transport without SSE support fails every SSE open with a typed
  unsupported error.
- Every stream failure of the HTTP component reports how many events the
  stream delivered before the failure. The adapter uses this count for the
  progress rule.
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
