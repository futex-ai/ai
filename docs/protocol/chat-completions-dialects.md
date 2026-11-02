# Chat Completions Dialects Protocol

Status: Approved, not implemented. Plan: [plans/ai-library-redesign.md](../../plans/ai-library-redesign.md).

When implemented, this protocol governs the shared Chat Completions engine
and replaces the shared chat-completions accumulator section of
[model-completion-streaming.md](model-completion-streaming.md).

This protocol defines the engine that serves the Chat Completions providers,
the settings of a dialect, and the xAI dialect. The provider docs of
DeepSeek, Kimi, Qwen, and MiniMax keep the values of their dialects.

## Scope

- One engine serves five dialects: DeepSeek, Kimi, Qwen, MiniMax, and xAI.
- Each dialect has its own provider id. The engine never replays the state
  of one dialect to another dialect.
- Each dialect keeps the wire behavior that its provider doc gives. This doc
  gives the xAI values.
- Decision: one engine with one dialect for each provider. The rejected
  alternative kept one crate for each provider, so that one protocol had
  five copies and each fix had to land five times.
- Decision: the spec decides which controls a model supports, through the
  [capability check](model-capabilities.md). A dialect decides only their
  wire form. The rejected alternative let each dialect hold its own control
  rules.

## Request

The engine sends `POST` with a JSON body to the base URL and the path of
the dialect, with the auth of the dialect.

| Body field | Value |
| --- | --- |
| `model` | The provider model id of the spec |
| `messages` | A leading `system` message with the system prompt when it is not blank, then the conversation messages in order |
| `tools` | One `function` tool for each tool definition, with its name, its description, and its complete input schema as `parameters` |
| `tool_choice` | The wire form of the resolved tool choice |
| `temperature`, `top_p` | The resolved sampling controls |
| Output-limit field | The resolved output limit |
| `stop` | The stop sequences in caller order |
| `stream`, `stream_options` | `true` and `{ "include_usage": true }` on the synchronous lifecycle |
| Thinking fields | The fields of the dialect for the resolved effort |
| Structured-output fields | The fields of the dialect when the request has a response schema |

- The engine leaves out a field that has no value and a collection that is
  empty.
- A user message with one text part and no other part sends its text as
  string content.
- Any other user message sends a content array of `text`, `image_url`, and
  `video_url` parts, in part order. A user message without parts sends an
  empty string.
- Image and video data travel as `data:<MIME type>;base64,<data>` URLs.
- An assistant message sends the concatenation of its text parts as its
  content. It sends its tool calls as `tool_calls`. It sends its own state
  as the replay rule of the dialect says.
- A replayed tool call sends the raw argument text from own state when the
  dialect keeps it, and the compact JSON of its arguments otherwise.
- A tool message sends the role `tool`, the call id as `tool_call_id`, and
  the result content as string content. It sends no name, except under the
  legacy rule of the xAI dialect.
- The engine sends the participant name of a user or assistant message only
  when the dialect takes names.

## Stream

- Each SSE `data` value is one JSON chunk or the exact value `[DONE]`.
- Only choice index 0 produces model events. The engine ignores other
  choices.
- `delta.content` gives text deltas. The reasoning field of the dialect
  gives reasoning deltas.
- When one chunk has both reasoning and text, the reasoning delta comes
  first.
- `delta.tool_calls` entries have an `index`. The first entry of an index
  gives the call id and the function name, and starts a tool-call part.
- Each `function.arguments` fragment gives one argument delta.
- A later entry of an index with another id or another name is a stream
  failure under the progress rule.
- The engine ends every tool-call part when the stream reaches its terminal
  event.
- The last non-null `finish_reason` is the finish reason.
- The last chunk with usage gives the usage.
- `[DONE]` is the terminal event, unless the dialect sets another terminal
  condition.
- An end of stream before the terminal event is a stream failure under the
  progress rule.
- A chunk with a standard `error` object is a provider error event.
- Before progress, the error code gives the kind. Rate-limit codes give
  rate limited, context-limit codes give context limit exceeded, server and
  overload codes give transient, and other codes give invalid request.
- After progress, a provider error event has the interrupted kind.
- The [model stream protocol](model-stream.md) defines the progress rule,
  the event order, and the checks before the usage event.

## Response rules

| Finish value | Finish reason |
| --- | --- |
| `stop` | Stop |
| `tool_calls` | Tool calls |
| `length` | Truncated |
| `content_filter` | Filtered |
| Any other value | Other, with the raw value |
| Missing or null | Other, with the value `missing` |

- A dialect can map more finish values, as its stream quirks say.
- A dispatchable tool call needs a call id, a function name, and argument
  text that is one JSON value. Otherwise the call fails with the invalid
  output kind.
- The engine validates structured output against the full response schema
  for every dialect. A provider JSON mode never replaces this check.
- Usage maps into buckets that do not overlap:
  - Cached input is the cached-token count of the dialect.
  - Input is the prompt tokens minus the cached input, or the cache-miss
    count when the dialect reports one. It saturates at zero.
  - Reasoning is the reasoning-token count of the dialect, or zero.
  - Output is the completion tokens minus the reasoning tokens, when the
    dialect reports reasoning separately. It saturates at zero.
  - Total is the provider total, or the saturating sum of the four buckets.
- Missing usage fields count as zero.
- HTTP failures use the shared classification of
  [model policies](model-policies.md).

## Dialect settings

| Setting | What the dialect decides |
| --- | --- |
| Endpoint | The default base URL and the path |
| Auth | The scheme that sends an API key |
| Model ids | The provider model ids of its specs |
| Output-limit field | `max_tokens` or `max_completion_tokens` |
| Reasoning field | The delta and message field of reasoning text, and any extra reasoning request field |
| Replay rule | The own state that it writes, and how a previous assistant message goes back: empty or null content, reasoning, and raw tool calls |
| Thinking parameter | The request fields for each effort |
| Usage fields | The source of the cached-token and reasoning-token counts |
| Tool-choice forms | The wire form of each allowed tool choice, and the choice that it sends when tools are present and the request sets none |
| Structured output | An instruction in the system prompt, a response format, or both |
| Input parts | The part kinds that the provider takes. Other parts fail with the unsupported input kind |
| Participant names | Whether the provider takes names on user and assistant messages |
| Stream quirks | Other terminal conditions, the position of usage, snapshot content, provider error codes, and extra finish values |
| Deferred lifecycle | An optional submit-and-poll lifecycle |

| Dialect | Provider id | Values |
| --- | --- | --- |
| DeepSeek | `deepseek` | [DeepSeek model provider](deepseek-model-provider.md) |
| Kimi | `kimi` | [Kimi model provider](kimi-model-provider.md) |
| Qwen | `qwen` | [Qwen model provider](qwen-model-provider.md) |
| MiniMax | `minimax` | [MiniMax model provider](minimax-model-provider.md) |
| xAI | `xai` | The xAI dialect section below |

## xAI dialect

| Setting | Value |
| --- | --- |
| Endpoint | `https://api.x.ai` and `/v1/chat/completions` |
| Auth | Bearer token |
| Model ids | The models of the xAI catalog |
| Output-limit field | `max_tokens` |
| Reasoning field | `reasoning_content` deltas when xAI exposes them. The dialect writes no reasoning state |
| Thinking parameter | `reasoning_effort`: `low`, `medium`, or `high` for those efforts, `xhigh` for extra high, and `high` for max. Left out for disabled and for Grok 4.20 models |
| Usage fields | Cached input from `prompt_tokens_details.cached_tokens`. Reasoning from `completion_tokens_details.reasoning_tokens` |
| Tool-choice forms | `none`, `auto`, `required`, and a named `function` object. `auto` when tools are present and the request sets none |
| Structured output | `response_format` of type `json_schema` with `strict: false`, and no instruction |
| Input parts | Text and image |
| Participant names | Taken |

- Legacy `delta.function_call` fragments form one tool call. The dialect
  gives it a synthetic call id that the same request always gives again.
- The finish value `function_call` maps to tool calls.
- A running usage report is replaced by the last usage chunk.
- The replay rule sends a legacy function call back as an assistant
  `function_call` without `tool_calls`. Its result goes back with the role
  `function` and the function name, without `tool_call_id`.
- The dialect leaves out empty assistant content.
- The deferred lifecycle follows the xAI deferred lifecycle section of
  [provider call controls](provider-call-controls.md).
