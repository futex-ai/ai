# Kimi Model Provider Protocol

## Purpose

Add Kimi K3 as a first-class `ai_interface::Model` provider while preserving
the workspace's provider-agnostic conversation, tool-calling, structured
output, routing, usage, and error contracts.

## Status

This contract is implemented by `ai-models-compat`, with shared routing
types, conversation parts, and provider state in `ai-interface`. The
[Add Kimi Model Provider plan](../../plans/add-kimi-model-provider.md) records
the completed implementation milestones.

Once implemented, the
[Chat Completions dialects protocol](chat-completions-dialects.md) governs the
shared engine that serves this provider.

## Scope

The provider supports Kimi K3 through Moonshot AI's Chat Completions API,
streamed over SSE as the [model stream protocol](model-stream.md) defines. It
includes text and image input, custom function tools, parallel tool calls,
structured JSON output, automatic cache usage reporting, and K3 reasoning
efforts. Its stream emits model events for the primary choice:
`delta.content` gives text deltas, and `delta.reasoning_content` gives
reasoning deltas.

The provider does not support K2.x models, Moonshot V1 models, Partial Mode,
video or file upload, dynamic or official Kimi tools, `prompt_cache_key`, or
`safety_identifier`. Those capabilities require
separate contracts rather than silent partial support. Shared video content
parts defined by the [video input protocol](video-input.md) are rejected with
the unsupported input kind before transport.

## Ownership

- `ai-interface` owns the model spec types, conversation parts, opaque
  provider state, and log copies.
- `ai-models-compat` owns the `kimi` provider id, the Kimi model spec and
  custom defaults, authentication, request and response mapping, the private
  Kimi replay state and its version 1 conversion, replay behavior, and
  provider error translation.
- `ai-models-core` owns shared HTTP error classification, the capability
  check, tool-argument parsing, and structured-output parsing and
  validation.
- `json-http` owns the injected HTTP and authentication boundaries.
- Composition roots own API-key retrieval, wrapper policy, deployment
  priority, and conversion of `KimiModel` into `DynModel`.

## Provider Identity And Endpoint

The Kimi provider id is `kimi`. Successful responses use `provider: "kimi"`.

Requests use:

```text
POST https://api.moonshot.ai/v1/chat/completions
Authorization: Bearer <api-key>
Content-Type: application/json
```

The model accepts an injected `DynJsonHttpClient` and explicit API key or
`DynJsonHttpAuth`. `new` and `with_auth` construct the `kimi-k3` spec, and
`with_catalog_auth` accepts a model spec. The model must not read
environment variables, load config, or resolve secrets.

## Initial Model Spec

The catalog lists one spec. It sends provider model id `kimi-k3` and exposes
tool calling, structured output, vision, long-context, and reasoning
features. The [control matrix](model-capabilities.md#control-matrix) gives
its efforts, max output, and context window.

| Text id | Constant | Intelligence | Speed | Cost |
| --- | --- | ---: | --- | --- |
| `kimi/kimi-k3` | `KIMI_K3` | 10 | Slow | Premium |

The spec lists the low, high, and max efforts. Its default effort is max
because K3 always reasons and its provider default is `max`. The routing
tiers describe K3 at that default effort. The capability check lowers medium
to low and extra high to high. A request with the disabled effort fails with
`UnsupportedControl` for effort before transport, because no K3 effort is at
or below it. The adapter never sends K2-specific or invented parameter
mappings. A model that the catalog does not list uses a
[custom spec](model-capabilities.md#custom-specs) with the Kimi custom
defaults.

## Request Contract

Every request contains the selected provider model id and retained conversation
messages in order. A nonempty `ModelRequest::system_prompt` is one leading
`system` message; an empty prompt is omitted.

K3 request mapping must:

- map the resolved low, high, and max efforts to top-level
  `reasoning_effort: "low"`, `"high"`, and `"max"`;
- omit fixed K3 parameters such as `temperature`, `top_p`, `n`, presence
  penalty, and frequency penalty, including when portable sampling is supplied;
- map portable output limits to `max_completion_tokens`, ordered stops to
  `stop`, and `none`, `auto`, `required`, or named-function tool choice to the
  native `tool_choice` shape;
- apply a portable total timeout to the overall SSE deadline, use ordinary synchronous
  completion for `PreferDeferred`, and reject `RequireDeferred` with typed
  `UnsupportedControl`;
- serialize a user message with one text part as a string, and other user
  messages as content arrays of text and image parts, using base64 data URLs
  for images; reject shared video parts with the unsupported input kind
  before transport; empty user and tool content remains an empty string, while
  unavailable assistant content may be null;
- serialize user and assistant names only when present and supported;
- ignore provider state owned by other providers;
- send `stream: true` and `stream_options.include_usage: true`, while sending
  no partial, file, video, or K2 `thinking` fields.

The default overall deadline is 3,600 seconds and the maximum idle gap is 120
seconds. Kimi reports usage on the final choice-bearing event; its direct
`cached_tokens` field is retained before `[DONE]` finalizes accumulation.

## Preserved Assistant Context

K3 requires complete prior assistant messages, including
`reasoning_content`, on multi-turn and tool-calling continuations. Portable
conversation parts alone cannot preserve nullable content or the raw JSON
argument strings returned for tool calls.

The response adapter therefore records Kimi provider state, as the
[conversation parts protocol](conversation-parts.md) defines:

- the text part keeps the nullable raw assistant `content`;
- the reasoning part keeps the nullable raw `reasoning_content`;
- each tool-call part keeps the provider call id, the function name, and the
  raw JSON argument string of its call, in provider order.

The tool-calling runtime keeps these parts with their state on the retained
assistant message. On a later Kimi request, the adapter replays the raw Kimi
values of its own state. It falls back to the portable parts only for
caller-authored messages without Kimi state.

Provider reasoning is replay-only data. It must not be appended to the
response text, parsed as structured output, or exposed as a tool call.
Model-call loggers receive log copies, which drop the provider state and the
reasoning text. The actual model request and the retained conversation keep
them. Kimi provider state participates in the shared synthetic tool-call
scope hash so distinct retained conversations cannot derive the same scope
accidentally.

## Tool Calling

Non-empty `ModelRequest::tools` become Kimi `function` tools with `name`,
`description`, and `parameters`; the request uses `tool_choice: "auto"`.
Provider-specific strict tool-schema mode is omitted because the shared tool
schema contract is not limited to Moonshot Flavored JSON Schema.

Assistant tool calls are dispatchable only when `finish_reason` is
`tool_calls`. Every dispatchable call must have a provider id, function name,
and arguments that parse as a JSON value. Parallel calls preserve provider
order and ids. Invalid arguments fail the model call before any tool is
dispatched.

Tool results serialize as `role: "tool"` with matching `tool_call_id` and
string content, including an empty string for an empty result. They do not send
a `name` field. The preceding assistant message must replay its Kimi raw tool
calls and reasoning state so every tool result remains paired with the
provider call that produced it.

Tool-call payloads returned with terminal, truncated, filtered, unknown, or
missing finish reasons are neither dispatched nor replayed.

## Responses And Errors

The [Kimi response protocol](kimi-responses.md) defines structured output,
the response contract, usage, and the error contract.

## Required Verification

Tests must cover provider identity, spec metadata, bearer auth,
endpoint selection, all message roles, image input, reasoning-effort mapping,
foreign provider-state isolation, raw Kimi assistant replay, parallel tool
calls and paired results, terminal tool suppression, structured-output request
and local validation, finish reasons, nullable content, missing choices,
usage normalization, HTTP classification, transport failures, and malformed
responses.

The credential-free workspace smoke test must construct the default Kimi
model. The ignored workspace integration test `xtask/tests/live_models.rs`
separately calls each listed effort of the Kimi spec through the production
adapter when an explicit `LIVE_MODEL_API_KEY` is supplied; GitHub Actions runs
it for eligible pull requests and from the scheduled/manual `Live model APIs`
workflow. Full formatting, file-length lint, Clippy, workspace tests,
`cargo xtask check`, commit and push, and post-push `cargo xtask review` are
required before implementation handoff.

## References

- [Kimi model list](https://platform.kimi.ai/docs/models)
- [Kimi Chat Completions API](https://platform.kimi.ai/docs/api/chat)
- [Kimi K3 guide](https://platform.kimi.ai/docs/guide/kimi-k3-quickstart)
- [Kimi model parameter reference](https://platform.kimi.ai/docs/api/models-overview)
- [Kimi structured output guide](https://platform.kimi.ai/docs/guide/response_format)
- [Kimi tool-calling guide](https://platform.kimi.ai/docs/guide/use-kimi-api-to-complete-tool-calls)
- [Kimi error codes](https://platform.kimi.ai/docs/api/errors)
