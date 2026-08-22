# MiniMax Model Provider Protocol

## Purpose

Add MiniMax as a first-class `ai_interface::Model` provider without treating
its API as an interchangeable OpenAI clone. The adapter uses MiniMax's
OpenAI-compatible Chat Completions endpoint while preserving MiniMax-specific
interleaved-thinking context, error codes, model metadata, and usage details.

## Status

Implemented. The
[completed MiniMax model provider plan](../../plans/minimax-model-provider.md)
records the implementation and verification milestones.

## Ownership

- `ai-interface` owns the stable `minimax` provider identifier and typed
  MiniMax conversation context required for stateless replay.
- `ai-models-minimax` owns MiniMax catalog metadata, request and response DTOs,
  authentication, transport dispatch, provider error handling, and response
  normalization.
- `ai-models-core` continues to own provider-agnostic HTTP classification,
  structured-output validation, routing metadata types, and pricing wrappers.
- Composition roots own credential lookup, retry/concurrency/pricing wrappers,
  route ordering, and selection of a catalog model.

The provider crate must not read environment variables, load configuration, or
resolve secrets.

## API Boundary

The V1 adapter uses:

- Endpoint: `POST https://api.minimax.io/v1/chat/completions`
- Authentication: `Authorization: Bearer <api-key>`
- Content type: `application/json`
- Response mode: internally accumulated SSE
- Provider identifier: `minimax`

`MiniMaxModel::new` accepts an injected `DynJsonHttpClient`, model id, and API
key. `MiniMaxModel::with_auth` accepts an injected `DynJsonHttpAuth`.
`MiniMaxModel::with_catalog_auth` additionally separates the catalog id from
the upstream model id and accepts `ThinkingLevel`.

The international endpoint is the V1 boundary. The China-region endpoint,
custom endpoint overrides, public incremental streaming, server-side tools,
and the MiniMax Responses and Anthropic-compatible APIs are out of scope.

## Known Model Catalog

The initial catalog contains current, non-legacy agent models:

| Catalog id | Provider model id | Context | Intelligence | Speed | Cost | Thinking | Features |
| --- | --- | ---: | --- | --- | --- | --- | --- |
| `MiniMax-M3` | `MiniMax-M3` | 1,000,000 | Nine | Medium | Low | Medium/adaptive | tools, structured output, vision, video input, long context, reasoning |
| `MiniMax-M3-thinking-disabled` | `MiniMax-M3` | 1,000,000 | Nine | Fast | Low | Disabled | tools, structured output, vision, video input, long context |
| `MiniMax-M2.7` | `MiniMax-M2.7` | 204,800 | Eight | Medium | Low | Medium/always enabled | tools, structured output, long context, reasoning |
| `MiniMax-M2.7-highspeed` | `MiniMax-M2.7-highspeed` | 204,800 | Eight | Fast | Medium | Medium/always enabled | tools, structured output, long context, reasoning |

The provider crate exports typed constants for every catalog id and a
`known_models()` function. Legacy M2.5, M2.1, and M2 models and the dialogue-
specific M2-her model are not included.

The M3 disabled-thinking entry is a catalog variant: it sends the same
`MiniMax-M3` provider id with a different thinking control. The tiers above
must be covered by tests; cost is coarse routing metadata, not a hard-coded
billing schedule.

## Request Mapping

Every request must:

1. Send the selected provider model id.
2. Set `stream: true`, `stream_options.include_usage: true`, and
   `reasoning_split: true`.
3. Add a nonblank normalized system prompt as the first `system` message and
   omit empty or whitespace-only system prompts.
4. Append retained conversation messages in order.
5. Send function tools using their name, description, and JSON Schema
   parameters.
6. Omit optional collections and fields when they are empty or unavailable.

Conversation roles map as follows:

- `User` -> `user`
- `Assistant` -> `assistant`
- `Tool` -> `tool`

Plain content is sent as a string. Shared text, image, and video content parts
are sent as OpenAI-compatible `text`, `image_url`, and `video_url` parts.
Base64 images and videos use a `data:<mime-type>;base64,<data>` URL. MiniMax-M3
supports video input, and only its catalog entries advertise the shared
`video_input` feature; the
[video input protocol](video-input.md) defines the shared contract.

Assistant history must include visible content, normalized tool calls, and any
retained MiniMax reasoning context. Tool results include `content` and the
provider tool-call id but no unsupported name field. Tool `content` remains
present when the result is an empty string; only unavailable assistant content
is omitted. Provider context owned by another adapter must not be serialized
as MiniMax reasoning.

MiniMax does not support legacy `function_call`; the adapter sends and accepts
modern `tools` and `tool_calls` only.

Portable temperature and top-p map to their native fields, and an output limit
maps to `max_completion_tokens`. All catalog models support portable `none`
and `auto`. MiniMax-M3 additionally maps strict `Required` and
`RequiredOrAuto` to `tool_choice: "required"`. Other catalog models retain
strict rejection for `Required`, while `RequiredOrAuto` preserves tools and
maps to `auto`. Named-function choices and nonempty stop sequences return typed
`UnsupportedControl` before transport. A total timeout replaces the default
3,600-second overall SSE deadline; the idle timeout is 120 seconds.
`PreferDeferred` falls back to synchronous,
and `RequireDeferred` is unsupported.

MiniMax streams cumulative visible-content snapshots. The adapter validates
that each snapshot extends the prior value, converts it to a suffix delta for
the shared accumulator, and retains the final complete `reasoning_details`
snapshot for replay. Final usage and `[DONE]` are required.

MiniMax's public reference currently enumerates only `none` and `auto`. Firna
reported a successful live MiniMax-M3 request with a real tool and
`tool_choice: "required"`; this adapter deliberately retains that verified
provider behavior instead of rejecting it from documentation absence alone.
The credentialed repository suite repeats the forced-tool request and asserts
the returned `live_probe` call whenever `LIVE_MODEL_API_KEY` is available.

## Thinking And Replay

MiniMax-M3 thinking maps from `ThinkingLevel`:

- `Disabled` -> `{ "thinking": { "type": "disabled" } }`
- `Medium` -> `{ "thinking": { "type": "adaptive" } }`
- `Low` downgrades to `Disabled`
- `High`, `ExtraHigh`, and `Max` downgrade to `Medium`

M2.x reasoning cannot be disabled. The known M2.7 catalog entries therefore
use an enabled thinking level and never advertise a disabled variant.
Responses record the effective catalog level after downgrade resolution.

With `reasoning_split: true`, the provider may return both
`reasoning_content` and `reasoning_details`. A reasoning detail preserves all
populated provider fields:

- `type`
- `id`
- `format`
- `index`
- `text`

`ai-interface` stores these values in a typed MiniMax provider-conversation
item. The item is attached to the normalized assistant response and serialized
back onto the corresponding assistant history message on the next request.
This replay is mandatory for tool continuations because MiniMax requires the
complete assistant response to maintain interleaved thinking.

Reasoning text is provider replay state. It must not be concatenated into
`assistant_message`, surfaced as ordinary assistant content, or copied into
normal user-visible logging. The tool runtime therefore removes the complete
MiniMax replay item from request and response copies passed to model-call
loggers while preserving it in the actual model request and retained
conversation.

## Structured Output

MiniMax Chat Completions does not document native JSON Schema response-format
enforcement. When `ModelRequest.response_schema` is present, the adapter:

1. Appends plain instructions and the schema to the system prompt.
2. Requests raw JSON without Markdown fences or additional prose.
3. Parses and locally validates the final stopped assistant text through
   `ai-models-core`.
4. Populates `structured_output` only for a natural stop with no tool calls.

Invalid JSON, an invalid requested schema, or a schema mismatch returns a
typed model-boundary error. Structured-output parsing must never run on a
filtered, truncated, tool-calling, missing, or unknown finish.

## Response Mapping

The adapter consumes the first response choice. A successfully decoded
response with no choices is a provider failure. Wire fields that cannot be
decoded into the typed response DTO are internal deserialization failures, as
defined under Error Handling.

Finish reasons normalize as follows:

| MiniMax value | Shared value |
| --- | --- |
| `stop` | `FinishReason::Stop` |
| `tool_calls` | `FinishReason::ToolCalls` |
| `length` | `FinishReason::Truncated` |
| `content_filter` | `FinishReason::Filtered` |
| unknown value | `FinishReason::Other(raw)` |
| missing value | `FinishReason::Other("missing")` |

Tool calls are exposed only when the normalized finish reason is `ToolCalls`.
Terminal, filtered, truncated, missing, and unknown responses must not dispatch
tool-call payloads. Function arguments are parsed as JSON; malformed arguments
return a provider error instead of reaching the tool runtime.

The normalized response records:

- provider `minimax`
- provider model id
- catalog model id
- selected thinking level
- visible assistant content
- normalized tool calls and finish reason
- locally validated structured output when requested
- typed MiniMax replay context
- normalized usage

## Usage

Usage parsing accepts absent fields and the documented OpenAI-compatible
details:

- `prompt_tokens`
- `completion_tokens`
- `total_tokens`
- `prompt_tokens_details.cached_tokens`
- `completion_tokens_details.reasoning_tokens`

Normalized buckets must not overlap. Cached tokens are subtracted from prompt
tokens, and reasoning tokens are subtracted from completion tokens using
saturating arithmetic. When `total_tokens` is absent, it is reconstructed from
the four normalized buckets. Provider parsing leaves estimated cost at zero;
composition roots may apply `UsagePricingModel`.

## Error Handling

HTTP failures use the shared status classifier: rate limits are retryable,
timeouts and conflict/early-data/server statuses are transient, recognized
context overflows are context-limit errors, and other statuses are provider
errors. Transport and authentication-hook failures are transient; local
request serialization and response deserialization failures are internal.

MiniMax may also report failure through a non-zero `base_resp.status_code`,
including on an HTTP-success response. The adapter must check this before
accepting choices:

- `1002`, `1041`, `2045`, and `2056` -> rate-limited
- `1000`, `1001`, `1013`, `1024`, and `1033` -> transient provider failure
- `1039` -> context-limit exceeded
- Every other non-zero code, including `1004`, `1008`, `1026`, `1027`, `2013`,
  and `2049` -> non-retryable provider failure

The provider's numeric code and `status_msg` must be retained in the error
message. Missing `base_resp` or status code zero means no provider-level error.
Before stream progress those numeric codes retain the classifications above;
after progress they become `ModelError::Interrupted`.

## Verification Contract

Unit tests use `JsonHttpTransportMock` through
`TransportBackedJsonHttpClient` and must not perform live network calls.
Coverage includes:

- catalog ids, provider ids, context windows, features, and thinking variants
- provider config parsing and serde round trips
- endpoint and bearer/custom authentication
- text, image, video, assistant, tool-call, and tool-result request
  serialization
- thinking control and reasoning-split serialization
- reasoning-context parsing, non-disclosure, serde, and continuation replay
- multiple tool calls, malformed arguments, and terminal-call suppression
- every normalized finish reason and empty-choice behavior
- structured-output prompting, success, and validation failures
- strict and fallback tool-choice behavior, including MiniMax-M3 required
- cached/reasoning usage normalization and missing usage
- HTTP and `base_resp` error classification
- credential-free provider construction in `cargo xtask smoke-test`

The ignored workspace integration test `xtask/tests/live_models.rs` separately
calls every MiniMax catalog entry through the production adapter when an
explicit `LIVE_MODEL_API_KEY` is supplied. GitHub Actions runs that billable
suite for eligible pull requests and from the scheduled/manual `Live model
APIs` workflow. Before the catalog probe, it verifies MiniMax-M3 with a real
tool and strict required selection. The implementation environment for this
change had no MiniMax credential, so that billable assertion could not be
re-run locally; deterministic wire coverage and the credentialed workflow
remain mandatory.

The full workspace must pass formatting, Clippy, tests, the Rust file-length
lint, smoke tests, and `cargo xtask check`.

## Official References

- [Model invocation](https://platform.minimax.io/docs/guides/text-generation)
- [OpenAI Chat Completions API](https://platform.minimax.io/docs/api-reference/text-chat-openai)
- [Tool use and interleaved thinking](https://platform.minimax.io/docs/guides/text-m3-function-call)
- [Error codes](https://platform.minimax.io/docs/api-reference/errorcode)
- [Pay-as-you-go pricing](https://platform.minimax.io/docs/guides/pricing-paygo)
