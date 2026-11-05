# MiniMax Response Protocol

This protocol defines how the MiniMax adapter parses structured output, maps
responses and usage, and handles errors. The
[MiniMax model provider protocol](minimax-model-provider.md) defines the
provider identity, the model specs, the request mapping, thinking and replay,
and the verification contract.

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

The adapter consumes the first response choice. A stream without a choice
is a stream failure under the progress rule, as the
[dialect protocol](chat-completions-dialects.md) says. Wire fields that cannot be
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
- text id
- resolved effort
- visible assistant content
- normalized tool calls and finish reason
- locally validated structured output when requested
- MiniMax provider state on the reasoning part
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
