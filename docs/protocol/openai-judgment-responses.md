# OpenAI Judgment Response Protocol

Status: Not implemented.
[Milestone 4](../../plans/shared-judgment-and-openai-decisions.md#milestone-4-openai-decisions-adapter)
adds this mapping.

This protocol defines how the OpenAI judgment adapter maps responses and
usage, and how it handles errors, timeouts, and retries. The
[OpenAI judgment provider protocol](openai-judgment-provider.md) defines the
purpose, the scope, the endpoint, the spec, the support, the request mapping,
and the verification contract.

## Response Mapping

A `2xx` body for the
[example request](openai-judgment-provider.md#request-mapping) has this
shape:

```json
{
  "model": "gpt-6-luna",
  "answers": [
    { "type": "choice", "name": "q0", "choice": "billing", "confidence": 0.82,
      "probabilities": [
        { "value": "billing", "probability": 0.91 },
        { "value": "other", "probability": 0.09 }
      ] },
    { "type": "refusal", "name": "q1" },
    { "type": "choice", "name": "q2", "choice": true, "confidence": 0.76,
      "probabilities": [
        { "value": true, "probability": 0.88 },
        { "value": false, "probability": 0.12 }
      ] }
  ],
  "usage": {
    "input_tokens": 1500,
    "input_tokens_details": { "cached_tokens": 1024, "cache_write_tokens": 0 },
    "output_tokens": 0,
    "output_tokens_details": { "reasoning_tokens": 0 },
    "total_tokens": 1500
  }
}
```

- `model` becomes `resolved_model_id`.
- The adapter matches each answer to its question by the echoed `name`, not
  by position. It ignores an answer without a name, or with a name that the
  request did not send.
- Two answers with the same name are invalid output.
- `predicate` becomes `Condition`, with its `probability`.
- `choice` becomes `Choice`. `choice` becomes `selected`, the
  `probabilities` entries give the probability of each label, and
  `confidence` stays. A boolean value in the answer to a choice question is
  invalid output, because the labels are strings.
- The answer to a condition with criteria is a `choice` with boolean values.
  The adapter checks it as the response check checks a choice whose options
  are the boolean values `true` and `false`. A string value in this answer is
  invalid output. The probability of `true` then becomes the condition
  probability.
- `score` becomes `Score`. `score` becomes `expected`, each `probabilities`
  entry gives the probability of the level at index `value`, and
  `confidence` stays. The adapter ignores the `label` of each entry, because
  the caller owns the levels.
- A repeated `value` in a choice or score distribution is invalid output.
- `refusal` becomes `JudgmentAnswer::Refused`.
- After the mapping, the adapter applies the shared
  [response check](judgment-responses.md#response-check).
- A body that fails to deserialize has the invalid output kind. Unknown
  fields are ignored.

## Usage

| `ModelUsage` field | Source |
| --- | --- |
| `input_tokens` | `input_tokens` minus `input_tokens_details.cached_tokens` |
| `cached_input_tokens` | `input_tokens_details.cached_tokens` |
| `output_tokens` | `output_tokens` minus `output_tokens_details.reasoning_tokens` |
| `reasoning_tokens` | `output_tokens_details.reasoning_tokens` |
| `total_tokens` | `total_tokens`, or the sum of the four counts when it is missing |

- OpenAI counts cached and cache-write tokens inside `input_tokens`. The
  subtraction stops a cache read from counting twice, as in the OpenAI
  Responses adapter. The example response gives 476 input tokens and 1,024
  cached input tokens.
- Each subtraction stops at zero. A missing usage object or counter counts
  as zero.
- Cache-write tokens stay in the input count, so they get the plain input
  price. [Cache-write tokens get the plain input price](../../plans/follow-ups/cache-write-token-pricing.md)
  records this gap.
- Cost lines stay empty until a host applies `ai_models_policy::price_usage`.

## Errors

Every `JudgmentError` carries one error core of the
[model policies protocol](model-policies.md#error-core). The errors of local
validation, the support check, and the response check have the kinds that
the [judgment boundary](judgment-boundary.md) and the
[judgment response protocol](judgment-responses.md) give.

An error response has the OpenAI envelope
`{"error": {"message": ..., "type": ..., "code": ..., "param": ...}}`.

- A known `error.code` gives its kind from the code table of the OpenAI chat
  adapter. For example, `insufficient_quota` gives the rejected key kind,
  and `context_length_exceeded` gives the context limit exceeded kind.
- Without a known code, the HTTP status gives the kind, as the
  [error kinds table](model-policies.md#error-kinds) says.
- The message is the `error.message` of the envelope, or the text of a body
  without an envelope.
- The error core keeps the `x-request-id` response header as the provider
  request id, and `Retry-After` as the retry-after.
- No error text holds the API key, an auth header value, or the base64 data
  of a request image. The adapter redacts them in provider messages and
  diagnostics before it builds the error. The provider message can repeat
  other request text, as in the other OpenAI adapters.

## Timeout, Retries, And Billing

- The default total timeout is 60 s. A total timeout in the `openai`
  provider config replaces it.
- The adapter sends one request for each call, and it never retries.
- A transport failure, also a transport timeout, has the transient kind.
- Warning: OpenAI can process and bill a request after the timeout ends the
  call. A host that retries the transient kind can then pay twice for one
  decision.
- The same risk exists for
  [xAI deferred timeouts](../../plans/follow-ups/xai-deferred-timeout-rebills.md),
  [DeepSeek resource errors](../../plans/follow-ups/deepseek-resource-error-rebills.md),
  and
  [Google video poll failures](../../plans/follow-ups/google-video-poll-failure-rebills.md).
