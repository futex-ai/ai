# Kimi Model Provider Protocol

## Purpose

Add Kimi K3 as a first-class `ai_interface::Model` provider while preserving
the workspace's provider-agnostic conversation, tool-calling, structured
output, routing, usage, and error contracts.

## Status

This contract is planned but not yet implemented. The
[Add Kimi Model Provider plan](../../plans/add-kimi-model-provider.md) defines
the implementation milestones.

## Scope

The initial provider supports Kimi K3 through Moonshot AI's non-streaming Chat
Completions API. It includes text and image input, custom function tools,
parallel tool calls, structured JSON output, automatic cache usage reporting,
and K3 reasoning-effort variants.

The initial version does not support K2.x models, Moonshot V1 models,
streaming, Partial Mode, video or file upload, dynamic or official Kimi tools,
`prompt_cache_key`, `safety_identifier`, or live credential-dependent tests.
Those capabilities require separate contracts rather than silent partial
support.

## Ownership

- `ai-interface` owns the `Kimi` provider identifier and typed Kimi assistant
  replay context.
- `ai-models-kimi` owns Kimi catalog entries, authentication, request and
  response mapping, replay behavior, and provider error translation.
- `ai-models-core` owns shared HTTP error classification, tool-argument
  parsing, structured-output parsing and validation, and model metadata types.
- `json-http` owns the injected HTTP and authentication boundaries.
- Composition roots own API-key retrieval, wrapper policy, deployment
  priority, and conversion of `KimiModel` into `DynModel`.

## Provider Identity And Endpoint

`ProviderKind::Kimi` serializes and parses as `kimi`. Successful responses use
`provider: "kimi"`.

Requests use:

```text
POST https://api.moonshot.ai/v1/chat/completions
Authorization: Bearer <api-key>
Content-Type: application/json
```

The model accepts an injected `DynJsonHttpClient` and explicit API key or
`DynJsonHttpAuth`. It must not read environment variables, load config, or
resolve secrets.

## Initial Model Catalog

All initial catalog entries send provider model id `kimi-k3`, advertise a
1,000,000-token context window, and expose tool calling, structured output,
vision, long-context, and reasoning features.

| Catalog id | Constant | Effort | Intelligence | Speed | Cost |
| --- | --- | --- | ---: | --- | --- |
| `kimi-k3` | `KIMI_K3` | `max` | 10 | Slow | Premium |
| `kimi-k3-thinking-high` | `KIMI_K3_THINKING_HIGH` | `high` | 10 | Medium | Premium |
| `kimi-k3-thinking-low` | `KIMI_K3_THINKING_LOW` | `low` | 10 | Fast | High |

`KIMI_K3` is the default max-effort catalog entry because K3 always reasons
and its provider default is `max`. Construction must reject unsupported
thinking levels or provider model ids instead of sending K2-specific or
invented parameter mappings.

## Request Contract

Every request contains the selected provider model id, one leading `system`
message containing `ModelRequest::system_prompt`, and the retained
conversation messages in order.

K3 request mapping must:

- map `ThinkingLevel::Low`, `High`, and `Max` to top-level
  `reasoning_effort: "low"`, `"high"`, and `"max"`;
- omit fixed K3 parameters such as `temperature`, `top_p`, `n`, presence
  penalty, and frequency penalty;
- serialize plain content as a string and typed text/image parts as content
  arrays, using base64 data URLs for images;
- serialize user and assistant names only when present and supported;
- ignore provider context owned by other providers;
- send no streaming, partial, file, video, or K2 `thinking` fields.

## Preserved Assistant Context

K3 requires complete prior assistant messages, including
`reasoning_content`, on multi-turn and tool-calling continuations. Normalized
`ConversationMessage` fields alone cannot preserve nullable content or the raw
JSON argument strings returned for tool calls.

`ai-interface` therefore adds a typed Kimi assistant replay item containing:

- nullable raw assistant `content`;
- nullable raw `reasoning_content`;
- ordered raw tool calls, each with provider id, function name, and raw JSON
  argument string.

The response adapter retains this item in `ModelResponse::provider_context`.
The tool-calling runtime already copies provider context onto the retained
assistant message. On a later Kimi request, the adapter replays the raw Kimi
assistant item when present and falls back to normalized assistant fields only
for caller-authored messages without Kimi context.

Provider reasoning is replay-only data. It must not be appended to normalized
assistant text, parsed as structured output, or exposed as a tool call.
Kimi replay context participates in the shared synthetic tool-call scope hash
so distinct retained conversations cannot derive the same scope accidentally.

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
string content. They do not send a `name` field. The preceding assistant
message must replay its Kimi raw tool calls and reasoning context so every
tool result remains paired with the provider call that produced it.

Tool-call payloads returned with terminal, truncated, filtered, unknown, or
missing finish reasons are neither dispatched nor replayed.

## Structured Output

When `response_schema` is present, the request sends:

```json
{
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "<schema name>",
      "schema": {},
      "strict": false
    }
  }
}
```

Kimi recommends strict mode, but strict mode accepts the narrower Moonshot
Flavored JSON Schema contract. The workspace accepts general JSON Schema, so
the adapter uses non-strict provider generation and always applies the shared
local JSON parse and schema validation boundary.

Only `message.content` from a normal `stop` response with no dispatchable tool
calls is parsed. Reasoning content is never part of structured output.
Truncated, filtered, tool-call, unknown, and missing-finish responses preserve
their finish reason and do not attempt structured-output parsing.

## Response Contract

The first response choice maps to `ModelResponse`. An absent choice or invalid
provider payload is a typed non-retryable provider failure.

Finish reasons map as follows:

| Kimi value | Shared value |
| --- | --- |
| `stop` | `FinishReason::Stop` |
| `tool_calls` | `FinishReason::ToolCalls` |
| `length` | `FinishReason::Truncated` |
| `content_filter` | `FinishReason::Filtered` |
| any other string | `FinishReason::Other(raw)` |
| missing or null | `FinishReason::Other("missing")` |

Nullable assistant content normalizes to an empty string unless valid
structured output supplies its compact JSON representation. The response
reports the selected catalog id and normalized thinking level separately from
the provider model id.

## Usage

Kimi usage fields map as follows:

- `cached_input_tokens = cached_tokens`;
- `input_tokens = prompt_tokens - cached_tokens`, saturating at zero;
- `output_tokens = completion_tokens`;
- `reasoning_tokens = 0`, because Kimi does not report a separate reasoning
  token quantity;
- `total_tokens = total_tokens` when supplied, otherwise the saturating sum of
  normalized categories;
- estimated cost and cost lines remain zero and empty until a composition root
  applies `UsagePricingModel`.

Missing usage is valid and maps to zero values.

## Error Contract

HTTP failures use the shared status classifier: `429` is rate limited;
`408`, `409`, `425`, and `5xx` are transient; known typed context-limit codes
are context-limit failures; other statuses are non-retryable provider
failures. Provider error messages may be retained, but API keys and auth
headers must never appear in errors or diagnostics.

Transport and authentication-hook failures are transient provider failures.
Local request serialization and response deserialization failures are
internal errors. Invalid provider choices, tool arguments, or structured
output are non-retryable provider failures.

## Required Verification

Tests must cover provider serde/config parsing, catalog metadata, bearer auth,
endpoint selection, all message roles, image input, reasoning-effort mapping,
foreign provider-context isolation, raw Kimi assistant replay, parallel tool
calls and paired results, terminal tool suppression, structured-output request
and local validation, finish reasons, nullable content, missing choices,
usage normalization, HTTP classification, transport failures, and malformed
responses.

The credential-free workspace smoke test must construct the default Kimi
model. Full formatting, file-length lint, Clippy, workspace tests,
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
