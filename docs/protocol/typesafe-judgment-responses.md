# TypeSafe Judgment Response Protocol

This protocol defines how the TypeSafe adapter maps responses and usage, and
how it handles errors. The
[TypeSafe judgment provider protocol](typesafe-judgment-provider.md) defines
the provider identity, the catalog, the support, the request mapping, and the
verification contract.

## Response Contract

A `2xx` body has this shape:

```json
{
  "model": "jev-1.13.0",
  "answers": {
    "is_urgent": { "type": "noul", "noul": 0.92 },
    "department": {
      "type": "choice",
      "choice": "billing",
      "probabilities": { "billing": 0.84, "technical": 0.16 },
      "confidence": 0.6
    },
    "frustration": {
      "type": "score",
      "score": 1.6,
      "legend": { "0": "Calm", "1": "Frustrated", "2": "Very angry" },
      "probabilities": { "0": 0.05, "1": 0.3, "2": 0.65 },
      "confidence": 0.78
    }
  },
  "usage": { "input_tokens": 312, "output_tokens": 48 }
}
```

The adapter maps `model` to `resolved_model_id`, `noul` to
`Condition.probability`, `choice` to `Choice.selected`, and `score` to
`Score.expected`. Score probability keys parse from decimal strings to `u32`
level indexes. `legend` is not surfaced because the caller owns the levels.
Unknown fields are ignored. TypeSafe has no refusal answer, so the adapter
never returns `JudgmentAnswer::Refused`.

The adapter reads the response as raw bytes and deserializes successful
bodies directly from those bytes, never through an intermediate
`serde_json::Value`, so repeated JSON keys reach the typed decoder instead of
silently collapsing. The typed decoder rejects a repeated key in the `answers`
map, in a choice `probabilities` map, and in a score `probabilities` map, so
an ambiguous payload never becomes a successful judgment. Only statuses in
`200..300` are treated as success;
every other status is classified as a failure. Answer entries are held as raw
JSON fragments until their id is matched to a requested question, so
unrequested ids are ignored even when their fragments would not decode.

A body that fails to deserialize, including a non-decimal or duplicate score
level key, has the invalid output kind. Its message is the fixed text
`malformed provider payload` followed by the decoder's own diagnostic.
After mapping, the adapter applies the shared `validate_against`
postconditions, so a missing answer, a kind mismatch, an unknown or missing
option or level, an out-of-range probability, confidence, or expectation, a
distribution that does not sum to one, a selection that is not the most
probable option, or an expectation that contradicts its distribution is a
typed `InvalidAnswer` error.

## Usage

`usage.input_tokens` and `usage.output_tokens` map to `ModelUsage`
`input_tokens` and `output_tokens`; `total_tokens` is their saturating sum and
the cached and reasoning buckets remain zero. Missing usage is
`ModelUsage::default()`. TypeSafe documents each counter as nullable, so an
absent or `null` counter maps to zero rather than failing the response. Cost lines remain empty until a composition root
applies `ai_models_policy::price_usage`; TypeSafe currently bills input tokens
only.

## Error Contract

Every `JudgmentError` carries one error core of the
[model policies protocol](model-policies.md#error-core) and uses the
`[ai_interface/judgment]` display prefix. The core kind, not the variant,
gives the failure category. The kind decides retry and fallback, as the
[error kinds table](model-policies.md#error-kinds) says.

| Cause | Kind |
| --- | --- |
| `UnsupportedContent { json_type }`: local null, boolean, or number JSON content, with a typed `Null`, `Boolean`, or `Number` value | invalid request |
| `EmptyState`: local empty input | invalid request |
| `NoQuestions`: local empty question map | invalid request |
| `InvalidQuestion`: local typed question problem | invalid request |
| `UnsupportedInput`: an image part or a conversation input that the shared support check rejects before transport | unsupported input |
| `InvalidAnswer`: a provider answer failed the shared `validate_against` postconditions; carries the answer id and a typed `JudgmentAnswerProblem` | invalid output |
| HTTP 429 | rate limited |
| transport or auth-hook failure, HTTP 408/409/425, or 5xx including 529 overloaded | transient |
| HTTP 401, 402, or 403 authentication | rejected key |
| HTTP 404 | unknown model |
| HTTP 400 with a recognized context-limit code in `detail.error_type` | context limit exceeded |
| HTTP 422 validation, or another 4xx status that no row above names | invalid request |
| invalid response semantics, a malformed payload, or an unexpected status below 400 | invalid output |
| local serialization or invariant failure | internal |

The typed variants keep their detail fields. `Failure` holds every other
classified failure. `Internal` holds a failure with the shared tracked
`InternalError` source, such as a serialization failure. Every core holds
the provider, the configured model id, and the available provider message.

Status classification uses the shared status classification of
[model policies](model-policies.md#error-kinds), as the image and video
boundaries do.
Error bodies are decoded best-effort: a `detail` object supplies its
`message`; a `detail` string is used directly; any other body, including JSON
`null`, is retained as compact JSON or text; an empty body falls back to
`HTTP <status>`. TypeSafe currently answers a missing key with `403` and
`{"detail": {"error_type": "authentication_error", "message": "..."}}`.

API keys and auth headers must never appear in errors or diagnostics. The
adapter enforces this rather than assuming it. It applies the injected auth
hook exactly once per call, sends exactly the header values that application
produced, and derives the redaction set from those values: each full value
and, for scheme-prefixed values such as `Bearer <token>`, the token after the
last whitespace. Every provider message, transport diagnostic, and
malformed-payload diagnostic passes through a redaction step that replaces
each secret with `[redacted]` before the error is built. Secrets are applied
longest first so a shorter secret that is a substring of a longer one cannot
leave the longer credential partially exposed. `InvalidAnswer`
problems that carry provider-controlled labels, and the question and answer
ids of typed errors, are redacted the same way. A
failing auth hook cannot be sanitized because its credentials were never
discovered, so it has the transient kind with the fixed message
`authentication hook failed`, and its own diagnostic is discarded.
