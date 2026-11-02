# DeepSeek Response Protocol

Status: Implemented, as the
[DeepSeek model provider protocol](deepseek-model-provider.md) says.

This protocol defines how the DeepSeek adapter parses structured output,
maps responses and usage, and handles errors. The
[DeepSeek model provider protocol](deepseek-model-provider.md) defines the
provider identity, the model specs, the request mapping, thinking controls,
preserved assistant context, tool calling, and the verification contract.

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

The first response choice maps to `ModelResponse`. A stream without a
choice is a stream failure under the progress rule, as the
[dialect protocol](chat-completions-dialects.md) says. Typed wire deserialization failures are
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
provider `deepseek`, the provider model id and text id, the resolved effort,
visible content, calls, finish, optional structured output, private context,
and usage.

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

Missing usage fields and missing detail fields are valid and normalize to
zero or the documented fallback calculations.

## Error Contract

HTTP failures use the shared classifier: `429` is rate limited; `408`, `409`,
`425`, and `5xx` are transient; recognized context-limit codes are
context-limit failures; and other statuses, including `400`, `401`, `402`,
and `422`, are non-retryable provider failures.

Transport and auth-hook failures are transient. Local serialization and typed
deserialization failures are internal. Invalid calls, missing required
reasoning, malformed arguments, and invalid structured output are provider
failures. Credentials never enter errors or diagnostics.

The final empty-choice usage event must precede `[DONE]`. Stream failures and
provider error events before progress retain retryable/provider
classification; after any event they become `ModelError::Interrupted`.

An HTTP-success response with `finish_reason:
"insufficient_system_resource"` returns `ModelError::TransientProvider`
rather than a terminal response.
