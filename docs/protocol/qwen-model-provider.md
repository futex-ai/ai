# Qwen Model Provider Protocol

## Purpose

Add QwenCloud as a first-class `ai_interface::Model` provider while preserving
the workspace's provider-agnostic routing, conversation, vision, tool-calling,
structured-output, usage, and error contracts.

## Status

Implemented by `ai-models-qwen`, with shared routing and replay types in
`ai-interface`.

## Scope

The initial adapter supports the current stable Qwen3.7 Max, Plus, and Flash
models through QwenCloud's pay-as-you-go, non-streaming, OpenAI-compatible Chat
Completions API. It supports text, image input where the selected model accepts
it, hybrid thinking, preserved reasoning, custom and parallel function calls,
locally validated structured output, and normalized cache usage.

Token Plan and Coding Plan endpoints, snapshot and preview ids, legacy Qwen
models, built-in tools, web search, video and audio content, streaming,
Responses and Anthropic-compatible APIs, and custom endpoints are outside this
initial contract. Those capabilities have different availability, billing, or
wire behavior and require explicit contracts rather than silent partial
support.

## Ownership

- `ai-interface` owns the stable `Qwen` provider identifier and typed Qwen
  assistant replay context.
- `ai-models-qwen` owns catalog metadata, configuration validation,
  authentication, request and response mapping, replay, and Qwen-specific
  error translation.
- `ai-models-core` owns shared HTTP classification, tool-argument parsing,
  structured-output validation, and model metadata types.
- `json-http` owns the injected HTTP client and authentication boundaries.
- `ai-tool-calling` owns retention of provider context and removal of private
  reasoning from model-call logger copies.
- Composition roots own credentials, retry and concurrency policy, pricing,
  deployment priority, and conversion of `QwenModel` into `DynModel`.

The provider crate must not read environment variables, resolve secrets, or
perform network work during construction.

## API Boundary

The adapter uses:

```text
POST https://dashscope-intl.aliyuncs.com/compatible-mode/v1/chat/completions
Authorization: Bearer <QwenCloud API key>
Content-Type: application/json
```

Requests set `stream: false`. The default constructor selects
`qwen3.7-plus` with thinking enabled. Token Plan and Coding Plan keys require
different endpoints and are not interchangeable with this boundary.

## Known Model Catalog

The provider exposes the three current stable general-purpose Qwen3.7 aliases,
plus explicit non-thinking variants. All have a 1,000,000-token context
window. Qwen3.7 Max is text-only; Plus and Flash accept text and images.

| Catalog id | Provider id | Intelligence | Speed | Cost | Thinking | Vision |
| --- | --- | ---: | --- | --- | --- | --- |
| `qwen3.7-max` | `qwen3.7-max` | Ten | Slow | Premium | High | No |
| `qwen3.7-max-thinking-disabled` | `qwen3.7-max` | Ten | Medium | Premium | Disabled | No |
| `qwen3.7-plus` | `qwen3.7-plus` | Nine | Medium | Medium | High | Yes |
| `qwen3.7-plus-thinking-disabled` | `qwen3.7-plus` | Nine | Fast | Medium | Disabled | Yes |
| `qwen3.7-flash` | `qwen3.7-flash` | Eight | Fast | Low | High | Yes |
| `qwen3.7-flash-thinking-disabled` | `qwen3.7-flash` | Eight | VeryFast | Low | Disabled | Yes |

Enabled variants advertise reasoning; disabled variants do not. Every variant
advertises tool calling, structured output, and long context. Plus and Flash
also advertise vision. Cost is coarse routing metadata rather than a billing
schedule.

The crate exports typed constants for all six catalog ids and a
`known_models()` function. Construction rejects unknown provider ids.
Unsupported normalized thinking levels downgrade to the highest catalog level
that does not exceed the request.

## Request Mapping

Every request sends the selected provider model id, retained conversation
messages in order, `stream: false`, and the selected thinking controls. A
nonblank normalized system prompt is a leading `system` message; empty and
whitespace-only system prompts are omitted.

Portable output limits map to `max_completion_tokens` and ordered stops map to
`stop`. Temperature and top-p map only with thinking disabled; thinking mode
keeps native sampling defaults. Non-thinking mode supports all strict choices.
Thinking mode supports `none` and `auto`; strict `required` and named-function
choices return typed `UnsupportedControl`. `RequiredOrAuto` maps to forced
`required` when thinking is disabled and to `auto` with tools retained when
thinking is enabled. A total
timeout reaches the HTTP request, `PreferDeferred` falls back to synchronous,
and `RequireDeferred` is unsupported.

Shared roles map directly to `user`, `assistant`, and `tool`. Plain content is
sent as a string. Empty user and tool content remains an empty string. Empty
assistant content is null when tool calls are present. Tool messages include
the matching `tool_call_id` and omit `name`.

Plus and Flash map shared text and image parts to OpenAI-compatible `text` and
`image_url` content parts. Base64 images use
`data:<mime-type>;base64,<data>`. Max rejects every typed content-part message
before authentication or transport. All models reject typed content parts on
assistant and tool messages because those roles accept string content at this
boundary. All models also reject shared video parts defined by the
[video input protocol](video-input.md) with a typed provider error before
transport.

Provider context owned by another adapter is never serialized as Qwen state.
Without Qwen context, assistant calls are reconstructed from normalized calls.

## Thinking And Replay

Thinking maps exactly:

| `ThinkingLevel` | `enable_thinking` | `preserve_thinking` |
| --- | --- | --- |
| `High` | `true` | `true` |
| `Disabled` | `false` | `false` |

`Low` and `Medium` downgrade to `Disabled`; `ExtraHigh` and `Max` downgrade to
`High`. The adapter does not invent provider-specific budgets for those
levels. Responses record the effective level. Enabled responses may include
private `reasoning_content`. Qwen
requires the complete assistant message, including reasoning and raw tool
calls, when a thinking-mode tool flow continues.

`ai-interface` therefore stores a typed Qwen assistant replay item with the
nullable content, nullable reasoning, and ordered raw tool-call ids, names,
and JSON argument strings. Qwen requests prefer this item over normalized
assistant fields. Null replay content remains null only when raw tool calls are
present; without calls it becomes an empty string so the next request remains
a valid assistant message. An enabled-thinking tool-call finish without
`reasoning_content` is a non-retryable provider failure because exact
continuation cannot be guaranteed.

Reasoning is private provider state. It never enters normalized assistant text
or structured output, and the tool runtime removes the complete Qwen replay
item from logger copies while retaining it in the real conversation. Replay
state participates in deterministic tool-call scope hashing.

## Tool Calling

Non-empty shared tools become Qwen `function` tools with name, description,
and JSON Schema parameters. Default tool selection sends `tool_choice: "auto"`
and `parallel_tool_calls: true`; empty tool lists omit both fields. A thinking
`RequiredOrAuto` request retains the same tools and explicit automatic choice.

Calls are dispatchable only for a `tool_calls` finish. Each call must include
an id, function name, and arguments that parse as JSON. Provider order, ids,
and raw argument strings survive parallel calls and replay. Missing, empty,
whitespace-only, or malformed dispatchable calls are provider failures.

Tool payloads attached to stop, truncated, filtered, unknown, or missing
finish reasons are ignored and not retained. Tool results send string content
and the corresponding provider id.

## Structured Output

Every structured request appends an instruction containing the word `JSON`,
the schema name, and the complete JSON Schema to the system prompt. It requires
raw JSON without Markdown fences or extra prose.

Qwen's native JSON-object mode is used only for non-thinking Plus and Flash,
where QwenCloud documents it as supported. Enabled-thinking requests and Max
use prompt enforcement only. In all cases, a normal stopped response is parsed
and validated locally against the caller's complete JSON Schema through the
shared boundary. Native JSON mode guarantees JSON syntax, not arbitrary schema
conformance, so local validation is mandatory.

Structured parsing runs only for `FinishReason::Stop` with no dispatchable
tool calls. Reasoning content is never parsed as output. Invalid JSON, an
invalid schema, and schema mismatch are provider failures.

## Response And Usage

The first choice maps to `ModelResponse`; no choices is a provider failure.
Wire deserialization failures are internal errors retaining their source.
Nullable visible content becomes an empty string.

| Qwen value | Shared outcome |
| --- | --- |
| `stop` | `FinishReason::Stop` |
| `tool_calls` | `FinishReason::ToolCalls` |
| `length` | `FinishReason::Truncated` |
| `content_filter` | `FinishReason::Filtered` |
| other string | `FinishReason::Other(raw)` |
| missing or null | `FinishReason::Other("missing")` |

Usage maps into non-overlapping normalized buckets:

- `cached_input_tokens = prompt_tokens_details.cached_tokens`;
- `input_tokens = prompt_tokens - cached_input_tokens`, saturating at zero;
- `output_tokens = completion_tokens`;
- `reasoning_tokens = 0`, because QwenCloud does not report reasoning tokens
  separately in Chat Completions usage;
- `total_tokens` uses the provider value when present, otherwise the
  saturating sum of normalized buckets;
- estimated cost and cost lines remain empty until a composition root applies
  pricing.

Missing usage and detail objects are valid and normalize to zero.

## Error Contract

HTTP `429` is rate limited; `408`, `409`, `425`, and `5xx` are transient;
documented input-length failures are context-limit errors; and other statuses
are non-retryable provider failures. Transport and authentication-hook
failures are transient. Request serialization and response deserialization
failures are internal.

Missing choices, invalid calls, missing required reasoning, and invalid
structured output are non-retryable provider failures. Errors and diagnostics
must never contain credentials or authorization headers.

## Required Verification

Unit tests use `JsonHttpTransportMock` through `TransportBackedJsonHttpClient`
and never call QwenCloud. Coverage includes provider serde, catalog metadata,
construction, bearer auth and endpoint, every message role, image mapping and
Max rejection, thinking controls, private replay and foreign-context
isolation, parallel tools, terminal-call suppression, structured-output
modes and validation, finish reasons, response shape, cache usage, HTTP and
transport errors, logger redaction, deterministic hashing, and credential-free
smoke construction.

The ignored workspace integration test `xtask/tests/live_models.rs` separately
calls every Qwen catalog entry through the production adapter when an explicit
`LIVE_MODEL_API_KEY` is supplied. GitHub Actions runs that billable suite for
eligible pull requests and from the scheduled/manual `Live model APIs`
workflow.

Full formatting, file-length lint, Clippy, workspace tests, smoke tests,
`cargo xtask check`, commit and push, and post-push `cargo xtask review` are
required before handoff.

## Official References

- [QwenCloud API keys](https://docs.qwencloud.com/api-reference/preparation/api-key)
- [QwenCloud text models](https://docs.qwencloud.com/developer-guides/getting-started/text-generation-models)
- [OpenAI-compatible Chat API](https://docs.qwencloud.com/api-reference/chat/openai-chat)
- [Thinking and preserved reasoning](https://docs.qwencloud.com/developer-guides/text-generation/thinking)
- [Function calling](https://docs.qwencloud.com/developer-guides/text-generation/function-calling)
- [Structured output](https://docs.qwencloud.com/developer-guides/text-generation/structured-output)
- [QwenCloud errors](https://docs.qwencloud.com/api-reference/preparation/error-messages)
