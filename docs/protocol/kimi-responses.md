# Kimi Response Protocol

Status: Implemented, as the
[Kimi model provider protocol](kimi-model-provider.md) says.

This protocol defines how the Kimi adapter parses structured output, maps
responses and usage, and handles errors. The
[Kimi model provider protocol](kimi-model-provider.md) defines the provider
identity and endpoint, the model spec, the request contract, preserved
assistant context, tool calling, and the verification contract.

## Structured Output

When `response_schema` is present, the request sends:

```json
{
  "response_format": {
    "type": "json_schema",
    "json_schema": {
      "name": "<schema name>",
      "schema": {
        "type": "object",
        "properties": {
          "answer": { "type": "string" }
        },
        "required": ["answer"]
      },
      "strict": false
    }
  }
}
```

The `schema` object is copied verbatim from
`ModelRequest::response_schema.schema`; the object above is illustrative.
Kimi recommends strict mode, but strict mode accepts the narrower Moonshot
Flavored JSON Schema contract. The workspace accepts general JSON Schema, so
the adapter uses non-strict provider generation and always applies the shared
local JSON parse and schema validation boundary.

Only `message.content` from a normal `stop` response with no dispatchable tool
calls is parsed. Reasoning content is never part of structured output.
Truncated, filtered, tool-call, unknown, and missing-finish responses preserve
their finish reason and do not attempt structured-output parsing.

## Response Contract

The first response choice maps to `ModelResponse`. A stream without a
choice is a stream failure under the progress rule, as the
[dialect protocol](chat-completions-dialects.md) says. An invalid provider
payload has the invalid output kind.

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
reports the text id and the resolved effort separately from the provider
model id.

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

Missing usage fields are valid and map to zero values. A stream without a
usage chunk fails, as the [dialect protocol](chat-completions-dialects.md) says.

## Error Contract

HTTP failures take their kinds from the shared status classification of
[model policies](model-policies.md#error-kinds), and known typed
context-limit codes in a `400` body give the context limit exceeded kind.
Provider error messages may be retained, but API keys and auth headers must
never appear in errors or diagnostics.

Transport and authentication-hook failures are transient. Local request
serialization failures are internal errors. Response deserialization
failures and invalid provider choices, tool arguments, or structured output
have the invalid output kind.

Malformed, incomplete, or provider-error streams before progress keep the
kind of their cause. The same failures after progress have the interrupted
kind and are not blindly replayed.
