# DeepSeek Model Provider Protocol

## Purpose

Add DeepSeek V4 as a first-class `ai_interface::Model` provider while
preserving the workspace's provider-agnostic routing, conversation,
tool-calling, structured-output, usage, and error contracts.

## Status

Implemented. The
[DeepSeek Model Provider implementation plan](../../plans/add-deepseek-model-provider.md)
records the milestones and verification used to deliver this contract.

## Scope

The initial provider supports `deepseek-v4-pro` and `deepseek-v4-flash`
through DeepSeek's official, non-streaming OpenAI-format Chat Completions API.
It includes text, custom and parallel function calls, optional thinking,
reasoning effort, locally validated JSON output, and normalized cache and
reasoning usage.

The initial provider does not support the retired `deepseek-chat` or
`deepseek-reasoner` aliases, streaming, image input, the Anthropic-format API,
third-party or custom endpoints, FIM completion, chat prefix completion,
strict tool mode, beta endpoints, or live credential-dependent tests. These
capabilities require separate contracts rather than silent partial support.

## Ownership

- `ai-interface` owns the stable `DeepSeek` provider identifier and typed
  DeepSeek assistant replay context.
- `ai-models-deepseek` owns DeepSeek catalog metadata, configuration
  validation, authentication, request and response mapping, replay behavior,
  and provider error translation.
- `ai-models-core` owns shared HTTP classification, tool and structured-output
  parsing, model metadata, and deterministic tool-call identity.
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

Requests set `stream: false`. `DeepSeekModel::new` and `with_auth` construct
the default high-thinking `deepseek-v4-pro`; `with_catalog_auth` accepts the
injected client and auth, catalog id, provider id, and `ThinkingLevel`, then
validates the selection before returning a model.

## Known Model Catalog

Both provider models advertise a 1,000,000-token context window. Enabled
variants advertise tool calling, structured output, long context, and
reasoning. Disabled variants advertise the same features except reasoning.
DeepSeek V4 does not advertise vision.

| Catalog id | Provider id | Intelligence | Speed | Cost | Thinking |
| --- | --- | ---: | --- | --- | --- |
| `deepseek-v4-pro` | `deepseek-v4-pro` | Ten | Medium | Low | High |
| `deepseek-v4-pro-thinking-max` | `deepseek-v4-pro` | Ten | Slow | Low | Max |
| `deepseek-v4-pro-thinking-disabled` | `deepseek-v4-pro` | Ten | Fast | Low | Disabled |
| `deepseek-v4-flash` | `deepseek-v4-flash` | Nine | Fast | Low | High |
| `deepseek-v4-flash-thinking-max` | `deepseek-v4-flash` | Nine | Medium | Low | Max |
| `deepseek-v4-flash-thinking-disabled` | `deepseek-v4-flash` | Nine | VeryFast | Low | Disabled |

The crate exports `DEEPSEEK_V4_PRO`, `DEEPSEEK_V4_PRO_THINKING_MAX`,
`DEEPSEEK_V4_PRO_THINKING_DISABLED`, `DEEPSEEK_V4_FLASH`,
`DEEPSEEK_V4_FLASH_THINKING_MAX`,
`DEEPSEEK_V4_FLASH_THINKING_DISABLED`, and `known_models()`.

The cost tier is coarse routing metadata rather than a hard-coded billing
schedule. Construction rejects unknown provider model ids and normalized
thinking levels other than `Disabled`, `High`, and `Max`.

## Request Mapping

Every request sends the provider model id, a leading normalized `system`
message, and retained messages in order. `User`, `Assistant`, and `Tool` map
to `user`, `assistant`, and `tool`.

Plain content remains a JSON string. Empty user, assistant, and tool content
remains an empty string. User and assistant names are sent only when present;
tool messages omit `name` and include the matching `tool_call_id`.

DeepSeek V4 is text-only at this boundary. A message with any typed
`content_parts`, including an image part, fails locally with a typed provider
error before authentication or transport is invoked. The adapter must not
drop an image, use only its text fallback, or claim vision support.

The request omits unrepresented sampling parameters and `tool_choice`;
DeepSeek V4 thinking rejects `tool_choice`, while `tools` alone enables
automatic selection.

## Thinking Controls

Catalog thinking maps exactly:

| `ThinkingLevel` | `thinking.type` | `reasoning_effort` |
| --- | --- | --- |
| `Disabled` | `disabled` | omitted |
| `High` | `enabled` | `high` |
| `Max` | `enabled` | `max` |

`Low`, `Medium`, and `ExtraHigh` are rejected during construction instead of
depending on provider compatibility coercions. Every response records the
selected normalized thinking level separately from its catalog and provider
model ids.

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

## Structured Output

When `ModelRequest.response_schema` is present, the adapter:

1. Appends an instruction containing the word `JSON`, the schema name, and the
   complete JSON Schema to the system prompt.
2. Requires raw JSON without Markdown fences or additional prose.
3. Sends `response_format: { "type": "json_object" }`.
4. Parses and validates a normal stopped response through the shared
   `ai-models-core` JSON Schema boundary.

DeepSeek guarantees JSON syntax, not arbitrary schema conformance. Local
validation is mandatory; empty or invalid output, an invalid schema, and a
schema mismatch are typed provider failures.

Structured-output parsing runs only for `FinishReason::Stop` with no
dispatchable tool calls. Reasoning content is never part of the structured
value. Tool, truncated, filtered, resource-limited, unknown, and missing
finishes do not attempt parsing.

## Response Mapping

The first response choice maps to `ModelResponse`. A response with no choices
is a non-retryable provider failure. Typed wire deserialization failures are
internal errors that retain their source.

Finish reasons map as follows:

| DeepSeek value | Shared outcome |
| --- | --- |
| `stop` | `FinishReason::Stop` |
| `tool_calls` | `FinishReason::ToolCalls` |
| `length` | `FinishReason::Truncated` |
| `content_filter` | `FinishReason::Filtered` |
| `insufficient_system_resource` | transient `ModelError` |
| any other string | `FinishReason::Other(raw)` |
| missing or null | `FinishReason::Other("missing")` |

Nullable visible content becomes an empty string. The response includes
provider `deepseek`, provider and catalog ids, thinking level, visible content,
calls, finish, optional structured output, private context, and usage.

## Usage

DeepSeek usage maps into non-overlapping normalized buckets:

- `cached_input_tokens = prompt_cache_hit_tokens`;
- `input_tokens = prompt_cache_miss_tokens` when present, otherwise
  `prompt_tokens - prompt_cache_hit_tokens` with saturating arithmetic;
- `reasoning_tokens = completion_tokens_details.reasoning_tokens`;
- `output_tokens = completion_tokens - reasoning_tokens` with saturating
  arithmetic;
- `total_tokens = total_tokens` when present, otherwise the saturating sum of
  the four normalized buckets;
- estimated cost and cost lines remain zero and empty until a composition root
  applies `UsagePricingModel`.

Missing usage and missing detail fields are valid and normalize to zero or the
documented fallback calculations.

## Error Contract

HTTP failures use the shared classifier: `429` is rate limited; `408`, `409`,
`425`, and `5xx` are transient; recognized context-limit codes are
context-limit failures; and other statuses, including `400`, `401`, `402`,
and `422`, are non-retryable provider failures.

Transport and auth-hook failures are transient. Local serialization and typed
deserialization failures are internal. Missing choices, invalid calls,
missing required reasoning, malformed arguments, and invalid structured
output are provider failures. Credentials never enter errors or diagnostics.

An HTTP-success response with `finish_reason:
"insufficient_system_resource"` returns `ModelError::TransientProvider`
rather than a terminal response.

## Required Verification

Tests use `JsonHttpTransportMock` through
`TransportBackedJsonHttpClient`; unit tests never make live provider calls.
Coverage includes provider identity and catalog metadata; construction,
endpoint, and auth; every message role and local multimodal rejection;
thinking controls; parallel tools and raw replay; reasoning non-disclosure,
redaction, serde, and hashing; JSON prompting and validation; finish and
response shapes; cache/reasoning usage; error classification; and
credential-free smoke construction.

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
