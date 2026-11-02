# Qwen Model Provider Protocol

## Purpose

Add QwenCloud as a first-class `ai_interface::Model` provider while preserving
the workspace's provider-agnostic routing, conversation, vision, tool-calling,
structured-output, usage, and error contracts.

## Status

Implemented by `ai-models-compat`, with shared routing types, conversation
parts, and provider state in `ai-interface`.

Once implemented, the
[Chat Completions dialects protocol](chat-completions-dialects.md) governs the
shared engine that serves this provider.

## Scope

The adapter supports the current stable Qwen3.7 Max, Plus, and Flash models
through QwenCloud's pay-as-you-go, OpenAI-compatible Chat Completions API,
streamed over SSE as the [model stream protocol](model-stream.md) defines. It
supports text, image input where the selected model accepts it, hybrid
thinking, preserved reasoning, custom and parallel function calls, locally
validated structured output, and normalized cache usage. Its stream emits
model events for the primary choice: `delta.content` gives text deltas, and
`delta.reasoning_content` gives reasoning deltas.

Token Plan and Coding Plan endpoints, snapshot and preview ids, legacy Qwen
models, built-in tools, web search, video and audio content, Responses and
Anthropic-compatible APIs, and custom endpoints are outside this initial
contract. Those capabilities have different availability,
billing, or wire behavior and require explicit contracts rather than silent
partial support.

## Ownership

- `ai-interface` owns the model spec types, conversation parts, opaque
  provider state, and log copies.
- `ai-models-compat` owns the stable `qwen` provider id, the Qwen model specs
  and custom defaults, authentication, request and response mapping, the
  private Qwen replay state and its version 1 conversion, replay, and
  Qwen-specific error translation.
- `ai-models-core` owns shared HTTP classification, the capability check,
  tool-argument parsing, and structured-output validation.
- `json-http` owns the injected HTTP client and authentication boundaries.
- `ai-tool-calling` retains the response parts with their provider state,
  and gives model-call loggers log copies.
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

Requests set `stream: true` and `stream_options.include_usage: true`. The
default constructor selects the `qwen3.7-plus` spec, whose default effort
enables thinking. `with_catalog_auth` accepts a model spec. Token Plan and
Coding Plan keys require different endpoints and are not interchangeable
with this boundary.

## Known Model Specs

The provider exposes the three current stable general-purpose Qwen3.7
aliases, with one spec for each. Qwen3.7 Max is text-only; Plus and Flash
accept text and images. The
[control matrix](model-capabilities.md#control-matrix) gives the efforts,
the default effort, the max output, and the context window of each spec.

| Text id | Constant | Intelligence | Speed | Cost | Vision |
| --- | --- | ---: | --- | --- | --- |
| `qwen/qwen3.7-max` | `QWEN_3_7_MAX` | Ten | Slow | Premium | No |
| `qwen/qwen3.7-plus` | `QWEN_3_7_PLUS` | Nine | Medium | Medium | Yes |
| `qwen/qwen3.7-flash` | `QWEN_3_7_FLASH` | Eight | Fast | Low | Yes |

Every spec advertises tool calling, structured output, long context, and
reasoning. Plus and Flash also advertise vision. The routing tiers describe
each model at its default high effort. Cost is coarse routing metadata
rather than a billing schedule. A model that the catalog does not list uses
a [custom spec](model-capabilities.md#custom-specs) with the Qwen custom
defaults.

## Request Mapping

Every request sends the selected provider model id, retained conversation
messages in order, streaming usage fields, and the selected thinking controls. A
nonblank normalized system prompt is a leading `system` message; empty and
whitespace-only system prompts are omitted.

Portable output limits map to `max_completion_tokens` and ordered stops map to
`stop`. Temperature and top-p map only with thinking disabled; thinking mode
keeps native sampling defaults. Non-thinking mode supports all strict choices.
Thinking mode supports `none` and `auto`; strict `required` and named-function
choices return typed `UnsupportedControl`. `RequiredOrAuto` maps to forced
`required` when thinking is disabled and to `auto` with tools retained when
thinking is enabled. A total timeout replaces the default 3,600-second overall
SSE deadline; the idle timeout is 120 seconds. `PreferDeferred` falls back to synchronous,
and `RequireDeferred` is unsupported.

Shared roles map directly to `user`, `assistant`, and `tool`. Plain content is
sent as a string. Empty user and tool content remains an empty string. Empty
assistant content is null when tool calls are present. Tool messages include
the matching `tool_call_id` and omit `name`.

Plus and Flash map shared text and image parts to OpenAI-compatible `text` and
`image_url` content parts. Base64 images use
`data:<mime-type>;base64,<data>`. Max rejects every message with an image or
video part before authentication or transport. All models reject image and
video parts on assistant and tool messages, because the role table of the
[conversation parts protocol](conversation-parts.md) does not allow them. All
models also reject shared video parts defined by the
[video input protocol](video-input.md) with a typed provider error before
transport.

Provider state owned by another adapter is never serialized as Qwen state.
Without Qwen state, assistant calls are reconstructed from the portable
tool-call parts.

## Thinking And Replay

The resolved effort maps exactly:

| Effort | `enable_thinking` | `preserve_thinking` |
| --- | --- | --- |
| High | `true` | `true` |
| Disabled | `false` | `false` |

The Qwen specs list these two efforts. The capability check lowers low and
medium to disabled, and extra high and max to high. The adapter does not
invent provider-specific budgets for those efforts. Responses record the
resolved effort. Enabled responses may include
private `reasoning_content`. Qwen
requires the complete assistant message, including reasoning and raw tool
calls, when a thinking-mode tool flow continues.

The adapter therefore records Qwen provider state, as the
[conversation parts protocol](conversation-parts.md) defines. The text part
keeps the nullable content. The reasoning part keeps the nullable reasoning.
Each tool-call part keeps its raw call id, name, and JSON argument string, in
provider order. Qwen requests prefer this state over the portable parts.
Null replay content remains null only when raw tool calls are present; without
calls it becomes an empty string so the next request remains a valid assistant
message. An enabled-thinking tool-call finish without `reasoning_content` is a
non-retryable provider failure because exact continuation cannot be
guaranteed.

Reasoning is private provider state. It never enters the response text or
structured output. Model-call log copies drop the provider state and the
reasoning text, while the real conversation keeps them. Provider state
participates in deterministic tool-call scope hashing.

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

## Responses And Errors

The [Qwen response protocol](qwen-responses.md) defines structured output,
response and usage mapping, and the error contract.

## Required Verification

Unit tests use `JsonHttpTransportMock` through `TransportBackedJsonHttpClient`
and never call QwenCloud. Coverage includes provider identity, spec metadata,
construction, bearer auth and endpoint, every message role, image mapping and
Max rejection, thinking controls, private replay and foreign-state
isolation, parallel tools, terminal-call suppression, structured-output
modes and validation, finish reasons, response shape, cache usage, HTTP and
transport errors, logger redaction, deterministic hashing, and credential-free
smoke construction.

The ignored workspace integration test `xtask/tests/live_models.rs` separately
calls each listed effort of every Qwen spec through the production adapter
when an explicit `LIVE_MODEL_API_KEY` is supplied. GitHub Actions runs that
billable suite for eligible pull requests and from the scheduled/manual
`Live model APIs` workflow.

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
