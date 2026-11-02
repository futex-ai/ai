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

Invalid JSON or a schema mismatch has the invalid output kind, and an invalid
requested schema has the invalid request kind. Structured-output parsing must
never run on a filtered, truncated, tool-calling, missing, or unknown finish.

## Response Mapping

The adapter consumes the first response choice. A stream without a choice
is a stream failure under the progress rule, as the
[dialect protocol](chat-completions-dialects.md) says. Wire fields that cannot be
decoded into the typed response DTO are deserialization failures with the
invalid output kind, as defined under Error Handling.

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
return an error of the invalid output kind instead of reaching the tool
runtime.

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

HTTP failures take their kinds from the shared status classification of
[model policies](model-policies.md#error-kinds), also when the body holds a
`base_resp` code. An HTTP 400 body with `base_resp` status code `1039` gives
the context limit exceeded kind, as a recognized context code does.
Transport and authentication-hook failures are transient; local request
serialization failures are internal, and response deserialization failures
have the invalid output kind.

MiniMax may also report failure through a non-zero `base_resp.status_code`,
including on an HTTP-success response. The adapter must check this before
accepting choices:

- `1002`, `1041`, `2045`, and `2056` -> rate-limited
- `1000`, `1001`, `1013`, `1024`, and `1033` -> transient
- `1039` -> context-limit exceeded
- `1004`, `1008`, and `2049` -> rejected key
- `2022` -> unknown model
- `1027` -> invalid output, because a chat adapter never gives the content
  policy kind
- Every other non-zero code, including `1026` and `2013` -> invalid request

The provider's numeric code and `status_msg` must be retained in the error
message, also when an HTTP failure body holds `base_resp`. Missing
`base_resp` or status code zero means no provider-level error.
Before stream progress those numeric codes retain the classifications above;
after progress they have the interrupted kind.
