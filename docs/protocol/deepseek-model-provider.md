# DeepSeek Model Provider Protocol

## Purpose

Add DeepSeek V4 as a first-class `ai_interface::Model` provider while
preserving the workspace's provider-agnostic routing, conversation,
tool-calling, structured-output, usage, and error contracts.

## Status

Implemented. The
[DeepSeek Model Provider implementation plan](../../plans/add-deepseek-model-provider.md)
records the milestones and verification used to deliver this contract.

Once implemented, the
[Chat Completions dialects protocol](chat-completions-dialects.md) governs the
shared engine that serves this provider.

## Scope

The provider supports `deepseek-v4-pro` and `deepseek-v4-flash` through
DeepSeek's official OpenAI-format Chat Completions API, streamed over SSE as
the [model stream protocol](model-stream.md) defines.
It includes text, custom and parallel function calls, optional thinking,
reasoning effort, locally validated JSON output, and normalized cache and
reasoning usage. Its stream emits model events for the primary choice:
`delta.content` gives text deltas, and `delta.reasoning_content` gives
reasoning deltas.

The provider does not support the retired `deepseek-chat` or
`deepseek-reasoner` aliases, image input, the Anthropic-format API, third-party
or custom endpoints, FIM completion, chat prefix completion, strict tool mode,
or beta endpoints. These capabilities
require separate contracts rather than silent partial support.

## Ownership

- `ai-interface` owns the model spec types and typed DeepSeek assistant
  replay context.
- `ai-models-compat` owns the stable `deepseek` provider id, the DeepSeek
  model specs and custom defaults, authentication, request and response
  mapping, replay behavior, and provider error translation.
- `ai-models-core` owns shared HTTP classification, the capability check,
  tool and structured-output parsing, and deterministic tool-call identity.
- `json-http` owns the injected HTTP client and authentication boundaries.
- `ai-tool-calling` owns retention of provider context and removal of private
  replay state from logger copies.
- Composition roots own credentials, policy wrappers, deployment priority,
  and conversion of `DeepSeekModel` into `DynModel`.

The provider crate must not read environment variables, load deployment
configuration, resolve secrets, or perform ambient network work during
construction.

## API Boundary

The adapter uses:

```text
POST https://api.deepseek.com/chat/completions
Authorization: Bearer <api-key>
Content-Type: application/json
```

Requests set `stream: true` and `stream_options.include_usage: true`.
`DeepSeekModel::new` and `with_auth` construct the `deepseek-v4-pro` spec,
whose default effort is high. `with_catalog_auth` accepts the injected
client, a model spec, and auth.

Each completion uses a 3,600-second overall deadline and a 120-second idle
timeout. An explicit portable total timeout replaces the overall deadline.

## Known Model Specs

The catalog lists one spec for each provider model. Both specs advertise
tool calling, structured output, long context, and reasoning. DeepSeek V4
does not advertise vision. The
[control matrix](model-capabilities.md#control-matrix) gives the efforts,
the default effort, the max output, and the context window of each spec.

| Text id | Constant | Intelligence | Speed | Cost |
| --- | --- | ---: | --- | --- |
| `deepseek/deepseek-v4-pro` | `DEEPSEEK_V4_PRO` | Ten | Medium | Low |
| `deepseek/deepseek-v4-flash` | `DEEPSEEK_V4_FLASH` | Nine | Fast | Low |

The routing tiers describe each model at its default high effort. The cost
tier is coarse routing metadata rather than a hard-coded billing schedule.
A model that the catalog does not list uses a
[custom spec](model-capabilities.md#custom-specs) with the DeepSeek custom
defaults.

## Request Mapping

Every request sends the provider model id and retained messages in order. A
nonempty normalized system prompt is a leading `system` message; an empty
prompt is omitted. `User`, `Assistant`, and `Tool` map to `user`, `assistant`,
and `tool`.

Plain content remains a JSON string. Empty user, assistant, and tool content
remains an empty string. User and assistant names are sent only when present;
tool messages omit `name` and include the matching `tool_call_id`.

DeepSeek V4 is text-only at this boundary. A message with any typed
`content_parts`, including an image or video part, fails locally with a typed
provider error before authentication or transport is invoked. The adapter must
not drop an image or video, use only its text fallback, or claim vision
support.

Portable output limits map to `max_tokens` and ordered stops map to `stop` at
every effort. With thinking disabled, temperature, top-p, and all
portable tool choices map to native fields. With thinking enabled, sampling
keeps provider defaults, `auto` uses omission plus `tools`, `none` omits tools,
and strict required or named-function choice returns typed
`UnsupportedControl`. `RequiredOrAuto` maps to native automatic semantics by
retaining `tools` and omitting `tool_choice`. The DeepSeek specs carry these
rules as spec data, and the shared capability check applies them.
A total timeout controls the overall SSE deadline, `PreferDeferred` falls back
to synchronous, and `RequireDeferred` is unsupported.

## Thinking Controls

The resolved effort maps exactly:

| Effort | `thinking.type` | `reasoning_effort` |
| --- | --- | --- |
| Disabled | `disabled` | omitted |
| High | `enabled` | `high` |
| Max | `enabled` | `max` |

The DeepSeek specs list these three efforts. The capability check lowers
low and medium to disabled, and extra high to high. The adapter performs this
resolution locally rather than depending on provider coercion. Every
response records the resolved effort separately from its text id and
provider model id.

## Preserved Assistant Context

Thinking-mode tool calls require the complete assistant turn, including
`reasoning_content`, on subsequent requests. Normalized messages cannot
preserve nullable reasoning or raw JSON arguments without loss.

`ai-interface` therefore adds:

```rust
DeepSeekAssistantMessage {
    content: String,
    reasoning_content: Option<String>,
    tool_calls: Vec<DeepSeekToolCallContext>,
}
```

Each call context retains the provider id, function name, and raw JSON
arguments. It is added only for a dispatchable `tool_calls` finish. Null
assistant content becomes an empty string so replay always sends non-null
`content`.

An enabled-thinking tool-call response without `reasoning_content` is a
non-retryable provider failure because it cannot be continued correctly. A
disabled-thinking tool call may omit reasoning.

On continuation, the adapter prefers the raw DeepSeek item and uses normalized
assistant fields only without one. Foreign context is never serialized as
DeepSeek state.

Reasoning is private replay data: it never enters normalized assistant text,
structured output, or model-call logger copies. The replay item participates
in deterministic tool-call scope hashing.

## Tool Calling

Non-empty tools become standard `function` tools with `name`, `description`,
and `parameters`. The stable endpoint omits beta-only `strict`.

Calls are dispatchable only for a `tool_calls` finish. Each requires an id,
name, and JSON arguments; provider order, ids, and raw arguments survive
parallel calls and replay.

A `tool_calls` finish with an absent, null, empty, or malformed tool-call
collection is a non-retryable provider failure. Tool payloads attached to
terminal, truncated, filtered, unknown, or missing finish reasons are neither
parsed, dispatched, nor retained for replay.

Tool results send `role: "tool"`, string `content`, and the matching id. The
preceding assistant message replays its original content, reasoning, and raw
calls.

## Responses And Errors

The [DeepSeek response protocol](deepseek-responses.md) defines structured
output, response mapping, usage, and the error contract.

## Required Verification

Tests use `JsonHttpTransportMock` through
`TransportBackedJsonHttpClient`; unit tests never make live provider calls.
Coverage includes provider identity and spec metadata; construction,
endpoint, and auth; every message role and local multimodal rejection;
thinking controls; parallel tools and raw replay; reasoning non-disclosure,
redaction, serde, and hashing; JSON prompting and validation; finish and
response shapes; cache/reasoning usage; error classification; and
credential-free smoke construction.

The ignored workspace integration test `xtask/tests/live_models.rs` separately
calls each listed effort of every DeepSeek spec through the production adapter
when an explicit `LIVE_MODEL_API_KEY` is supplied. GitHub Actions runs that
billable suite for eligible pull requests and from the scheduled/manual
`Live model APIs` workflow.

Full formatting, Rust file-length lint, Clippy, workspace tests, smoke tests,
and `cargo xtask check` must pass before commit and push. `cargo xtask review`
runs after the complete branch is pushed.

## Official References

- [DeepSeek models and pricing](https://api-docs.deepseek.com/quick_start/pricing)
- [DeepSeek Chat Completions API](https://api-docs.deepseek.com/api/create-chat-completion)
- [DeepSeek thinking mode](https://api-docs.deepseek.com/guides/thinking_mode)
- [DeepSeek tool calls](https://api-docs.deepseek.com/guides/tool_calls)
- [DeepSeek JSON output](https://api-docs.deepseek.com/guides/json_mode)
- [DeepSeek error codes](https://api-docs.deepseek.com/quick_start/error_codes/)
- [DeepSeek V4 release and legacy retirement](https://api-docs.deepseek.com/news/news260424/)
