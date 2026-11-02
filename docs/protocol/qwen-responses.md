# Qwen Response Protocol

Status: Implemented, as the
[Qwen model provider protocol](qwen-model-provider.md) says.

This protocol defines how the Qwen adapter parses structured output, maps
responses and usage, and handles errors. The
[Qwen model provider protocol](qwen-model-provider.md) defines the API
boundary, the model specs, the request mapping, thinking and replay, tool
calling, and the verification contract.

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
tool calls. Reasoning content is never parsed as output. Invalid JSON and
schema mismatch have the invalid output kind. An invalid caller schema has
the invalid request kind.

## Response And Usage

The first choice maps to `ModelResponse`. A stream without a choice is a
stream failure under the progress rule, as the [dialect protocol](chat-completions-dialects.md)
says.
A typed response that does not deserialize has the invalid output kind and
retains its source.
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

Missing usage fields and detail objects are valid and normalize to zero.

## Error Contract

HTTP failures take their kinds from the shared status classification of
[model policies](model-policies.md#error-kinds), and documented input-length
failures are context-limit errors. Transport and authentication-hook
failures are transient. Request serialization failures are internal, and
response deserialization failures have the invalid output kind.

Invalid calls, missing required reasoning, and invalid structured output in
a generated response have the invalid output kind. Invalid authored roles
or replay state fail before transport with the invalid request kind. Errors
and diagnostics must never contain credentials or authorization headers.

The final empty-choice usage event and `[DONE]` are required. Stream failures
before progress are transient, while failures after progress have the
interrupted kind.
