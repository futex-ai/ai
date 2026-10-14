# OpenAI Judgment Provider Protocol

## Purpose And Status

Add OpenAI's Decisions API as a second implementation of the provider-agnostic
judgment boundary. The Decisions API evaluates text or image input against
named `predicate`, `choice`, and `score` questions and returns a probability,
a selected choice with its distribution, or a probability-weighted score. That
matches the `Condition`, `Choice`, and `Score` questions of
`ai_interface::JudgmentModel`, so `ai-models-openai` implements the same trait
instead of a new boundary.

This contract is implemented. The
[Add OpenAI Decisions judgment provider plan](../../plans/add-openai-judgment-provider.md)
records the implementation. Credentialed verification is defined by the
[live judgment API test protocol](live-judgment-api-tests.md).

## Shared Boundary

The request, question, answer, response, local validation, response
postconditions, and typed `JudgmentError` contract are defined once in the
[TypeSafe judgment provider protocol](typesafe-judgment-provider.md) and are
not repeated here. Every rule in that document's Shared Boundary and Error
Contract sections applies to the OpenAI adapter. This document defines only
the OpenAI identity, catalog, wire mapping, usage, and provider-specific error
behavior.

## Scope

One synchronous evaluation of text or JSON state against one or more
independent questions, all sent in one Decisions call. Out of scope: image
input (the shared `JudgmentContent` is text or JSON only), `safety_identifier`,
token counting, question fan-out across requests, model listing, retry,
concurrency, and pricing wrappers, request-id capture, and
confidence-threshold policy. Composition roots own those.

## Ownership

- `ai-interface` owns the shared judgment DTOs, trait, validation, and typed
  errors, including the `Refused` variant introduced by this change.
- `ai-models-core` owns the shared HTTP status classifier plus the shared
  judgment helpers introduced by this change: credential redaction
  (`redact_secrets`, `secrets_from_headers`), failure-body decoding, status
  and transport error classification into `JudgmentError`, and redaction of
  provider-controlled `InvalidAnswer` labels.
- `ai-models-openai` owns the Decisions catalog entry, wire mapping,
  response decoding, refusal mapping, usage normalization, and OpenAI error
  message extraction. `json-http` owns the injected client and auth hook
  boundaries. Composition roots own API-key retrieval, wrapper policy,
  pricing, and conversion of the adapter into `DynJudgmentModel`.

## Provider Identity And Endpoint

`ProviderKind::OpenAi` serializes and parses as `openai`. Successful
responses use `provider: "openai"`.

Requests use:

```text
POST https://api.openai.com/v1/decisions
Authorization: Bearer <api-key>
Content-Type: application/json
```

`OpenAiJudgmentModel::new(http_client, model_id, api_key)` and
`with_auth(http_client, model_id, auth)` accept an injected
`DynJsonHttpClient` and an explicit API key or `DynJsonHttpAuth`;
`with_endpoint` and `with_timeout` override the endpoint and the 60-second
default request timeout. The model id is the provider model id sent on the
wire, normally `GPT_6_LUNA`. Any provider model id is accepted so callers can
adopt a new Decisions model before the catalog lists it. The adapter must not
read environment variables, load config, or resolve secrets.

## Catalog

| Catalog id | Constant | Provider model id | Notes |
| --- | --- | --- | --- |
| `gpt-6-luna-decisions` | `GPT_6_LUNA_DECISIONS` | `gpt-6-luna` (`GPT_6_LUNA`) | the only model the Decisions API currently serves |

The entry advertises only `ModelFeature::Judgment`, so the chat, image, and
video suites exclude it automatically. It uses the documented GPT-6 Luna
context window of 1,050,000 tokens, `IntelligenceScore::Eight`,
`SpeedTier::VeryFast`, `CostTier::Low`, and `ThinkingLevel::Disabled`. The
catalog id carries the `-decisions` suffix so a future GPT-6 Luna chat entry
can own the bare `gpt-6-luna` catalog id. `GPT_6_LUNA` is exported as the
provider model id because the adapter constructors take the provider id, not
the catalog id.

## Content Rendering

The Decisions API accepts only text (and images, which are out of scope).
Every `JudgmentContent` value therefore renders to one string:

| Content | Rendering |
| --- | --- |
| `Text` | the text verbatim, including surrounding whitespace |
| `Object` | compact JSON with no insertion of whitespace, for example `{"ticket":"My payouts failed","attempts":4}` |
| `Array` | compact JSON, for example `["verbatim",null,3]` |

The same rendering applies to state, instructions, option descriptions, and
score levels. A rendering failure is an `Internal` error.

## Request Contract

The adapter serializes one JSON body:

```json
{
  "model": "gpt-6-luna",
  "input": "{\"ticket\":\"My payouts have been failing for 3 days.\"}",
  "questions": [
    {
      "type": "choice",
      "name": "department",
      "instructions": "Which team should handle `ticket`?",
      "choices": [
        { "value": "billing", "description": "Payments and refunds" },
        { "value": "technical" }
      ]
    },
    {
      "type": "score",
      "name": "frustration",
      "instructions": "How frustrated is the author of `ticket`?",
      "levels": [{ "label": "Calm" }, { "label": "Frustrated" }, { "label": "Very angry" }]
    },
    {
      "type": "predicate",
      "name": "is_urgent",
      "instructions": "Does `ticket` convey urgency?\n\nTrue when: Explicitly time-sensitive\nFalse when: No urgency"
    }
  ]
}
```

Mapping rules:

- `model` is the configured provider model id.
- `input` is the rendered state string.
- `questions` lists every request question in ascending id order (the
  `BTreeMap` order). Each entry sends the caller id as `name`, so OpenAI
  receives the id; TypeSafe does not. OpenAI echoes `name` in every answer.
- `Condition` becomes `type: "predicate"`. When `criteria` is present and
  has at least one description, the instructions become the rendered
  instructions, a blank line, then `True when: <yes>` and `False when: <no>`
  on separate lines, each line present only when its description is present.
  Absent criteria, or criteria with neither description, send the
  instructions unchanged.
- `Choice` options become the `choices` array in ascending label order. Each
  entry has `value: <label>`; `description` is the rendered description and
  is omitted when the description is `None`.
- `Score` levels become the ordered `levels` array. Each entry has
  `label: <rendered level>`; no `description` is sent because the shared
  boundary carries one content value per level.
- No other fields are sent. `safety_identifier` is never sent.

The adapter performs no provider-specific local validation beyond the shared
`JudgmentRequest::validate`. OpenAI requires between 2 and 255 unique choice
values and rejects other requests with HTTP 400, which the adapter reports as
a terminal `Provider` error.

## Response Contract

A `2xx` body has this shape:

```json
{
  "model": "gpt-6-luna",
  "answers": [
    { "type": "choice", "name": "department", "choice": "billing",
      "probabilities": [
        { "value": "billing", "probability": 0.84 },
        { "value": "technical", "probability": 0.16 }
      ],
      "confidence": 0.6 },
    { "type": "score", "name": "frustration", "score": 1.6,
      "probabilities": [
        { "value": 0, "label": "Calm", "probability": 0.05 },
        { "value": 1, "label": "Frustrated", "probability": 0.3 },
        { "value": 2, "label": "Very angry", "probability": 0.65 }
      ],
      "confidence": 0.78 },
    { "type": "predicate", "name": "is_urgent", "probability": 0.92 }
  ],
  "usage": {
    "input_tokens": 312,
    "input_tokens_details": { "cached_tokens": 0, "cache_write_tokens": 0 },
    "output_tokens": 0,
    "output_tokens_details": { "reasoning_tokens": 0 },
    "total_tokens": 312
  }
}
```

The adapter maps `model` to `resolved_model_id`, `probability` to
`Condition.probability`, `choice` to `Choice.selected`, each choice
`probabilities[].value` to an option label, `score` to `Score.expected`, and
each score `probabilities[].value` to a zero-based level index. The score
`label` is not surfaced because the caller owns the levels. Unknown fields
are ignored.

The adapter reads the response as raw bytes and deserializes the success body
directly from those bytes. Answer entries are held as raw JSON fragments. A
fragment whose `name` cannot be read, is `null`, or is not a requested id is
ignored even when the rest of the fragment would not decode. A fragment whose
`name` is a requested id must decode as a typed answer. Two fragments with the
same requested name, two choice probability entries with the same value, and
two score probability entries with the same index are rejected. Choice values
decode as strings only because the adapter never sends boolean values. Only
statuses in `200..300` are treated as success.

A body that fails to decode is a non-retryable `Provider` error whose message
is the fixed text `malformed provider payload` followed by the decoder's own
diagnostic. After mapping, the adapter applies the shared `validate_against`
postconditions, so a missing answer, a kind mismatch, an unknown or missing
option or level, an out-of-range value, a bad distribution sum, a selection
that is not the most probable option, or an inconsistent expectation is a
typed `InvalidAnswer` error.

## Refusals

OpenAI may answer a question with `{ "type": "refusal", "name": "<id>" }`
when the model declines to answer it; other questions in the same request can
still receive answers. The adapter decodes every requested fragment first, so
malformed payloads are reported before refusals. If any requested answer is a
refusal, the call fails with `JudgmentError::Refused { provider, model_id,
id }` for the lowest refused id in ascending id order, and no partial response
is returned, because the remaining answers describe the same state. The
`Refused` variant is terminal: do not retry; callers may route the state
elsewhere. TypeSafe never produces it.

## Usage

OpenAI usage uses the Responses conventions. `input_tokens_details.cached_tokens`
maps to `cached_input_tokens` and is subtracted from `input_tokens` with
saturating arithmetic; `output_tokens_details.reasoning_tokens` maps to
`reasoning_tokens` and is subtracted from `output_tokens`. `total_tokens` is
the provider value when present and otherwise the saturating sum of the four
normalized buckets. Every counter and detail object is nullable; absent or
`null` values map to zero, and a missing `usage` object is
`ModelUsage::default()`. `cache_write_tokens` is not surfaced. Cost lines
remain empty until a composition root applies `ai_models_core::price_usage`;
OpenAI currently bills Decisions input tokens only.

## Error Contract

The shared `JudgmentError` table applies. OpenAI-specific rules:

- Error bodies are decoded best-effort: an `error` object supplies its
  `message`; a JSON string body is used directly; any other body, including
  JSON `null`, is retained as compact JSON or text; an empty body falls back
  to `HTTP <status>`.
- Status classification uses `ai_models_core::classify_http_status`: 429 is
  `RateLimited`; 408, 409, 425, and 5xx are `TransientProvider`; every other
  failure status, including 400 validation, 401 and 403 authentication, and
  the `403` returned while an account lacks Decisions access, is `Provider`.
- Transport and auth-hook failures, redirects, malformed payloads, internal
  serialization failures, and credential redaction follow the TypeSafe
  protocol exactly, through the shared `ai-models-core` helpers: the auth hook
  is applied exactly once per call, the transmitted header values and their
  scheme-stripped tokens are redacted longest first from every diagnostic and
  from provider-controlled `UnknownOption` and `MissingOption` labels, and a
  failing hook is reported as `TransientProvider` with the fixed message
  `authentication hook failed`.

## Required Verification

Credential-free tests must cover the catalog entry, bearer auth and endpoint
selection, construction overrides, exact wire mapping for every question
variant including criteria folding and compact JSON rendering, response
mapping, ignored unnamed and unrequested fragments, duplicate names and
values, refusals, every validation failure class, usage normalization, status
classification, transport failures, malformed bodies, and credential
redaction. The credential-free smoke test must construct the adapter with
`GPT_6_LUNA`, and the ignored live suite calls every judgment-capable OpenAI
catalog entry through `DynJudgmentModel` when `LIVE_JUDGMENT_API_KEY` is
supplied. The repository gate, commit and push, and the post-push review are
required before handoff.

## References

- [OpenAI Decisions guide](https://developers.openai.com/api/docs/guides/decisions)
- [OpenAI Python SDK decisions resource](https://github.com/openai/openai-python/blob/main/src/openai/resources/decisions.py)
- [OpenAI Python SDK decision response types](https://github.com/openai/openai-python/blob/main/src/openai/types/decision.py)
- [OpenAI Node SDK decisions resource](https://github.com/openai/openai-node/blob/master/src/resources/decisions.ts)
- [TypeSafe judgment provider protocol](typesafe-judgment-provider.md)
