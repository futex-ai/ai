# TypeSafe Judgment Provider Protocol

## Purpose And Status

Add TypeSafe's System One models (Jev) to the workspace as a first-class
provider. TypeSafe does not generate text: it evaluates a caller-supplied
`state` against a map of typed questions and returns calibrated probabilities.
That contract does not fit `ai_interface::Model`, so this protocol defines a
separate provider-agnostic judgment boundary and the TypeSafe implementation
of it.

This contract is implemented. The
[Add TypeSafe judgment provider plan](../../plans/add-typesafe-judgment-provider.md)
records the implementation. Credentialed verification is defined by the
[live judgment API test protocol](live-judgment-api-tests.md).

## Scope

The boundary covers one synchronous evaluation of text or JSON state against
one or more independent condition, choice, and score questions. All questions
in a request are sent in one provider call because TypeSafe evaluates them in
parallel over the same state and prices the state once.

Out of scope: image, audio, or video state (Jev is text only), token counting,
automatic question fan-out across requests, model listing through
`GET /v1/models`, retry, concurrency, and pricing wrappers, request-id
capture, and confidence-threshold policy. Composition roots own those.

## Ownership

- `ai-interface` owns `ModelFeature::Judgment`, the model spec types, the
  judgment DTOs, typed errors, the `JudgmentModel` trait, its dyn alias and
  unimock API, and the deterministic `MockJudgmentModel`.
- `ai-models-core` owns the shared HTTP status classifier reused by this
  provider.
- `ai-models-typesafe` owns the `typesafe` provider id, the Jev specs,
  authentication, wire mapping, local validation, response validation, usage
  normalization, and provider error translation.
- `json-http` owns the injected HTTP client and auth hook boundaries.
- Composition roots own API-key retrieval, wrapper policy, pricing, and
  conversion of the concrete adapter into `DynJudgmentModel`.

## Shared Boundary

The [judgment boundary protocol](judgment-boundary.md) defines the
provider-agnostic `JudgmentModel` boundary: the request and response DTOs,
content and question shapes, local request validation, response validation,
and the deterministic `MockJudgmentModel`.

## Provider Identity And Endpoint

The TypeSafe provider id is `typesafe`. Successful responses use
`provider: "typesafe"`.

Requests use:

```text
POST https://api.typesafe.ai/v1/systemone
Authorization: Bearer <api-key>
Content-Type: application/json
```

`TypeSafeJudgmentModel::new(http_client, model_id, api_key)` and
`with_auth(http_client, model_id, auth)` accept an injected
`DynJsonHttpClient` and an explicit API key or `DynJsonHttpAuth`;
`with_endpoint` and `with_timeout` override the endpoint and the 60-second
default request timeout. A catalog spec gives the provider model id to pass.
Any provider model id is accepted so callers can pin a versioned release
before the catalog lists it. The adapter must not read environment
variables, load config, or resolve secrets.

## Catalog

The catalog lists one spec for each Jev model. Every spec advertises only
`ModelFeature::Judgment`, so chat, image, and video suites exclude it
automatically.

| Text id | Constant | Provider model id | Notes |
| --- | --- | --- | --- |
| `typesafe/jev-latest` | `JEV_LATEST` | `jev-latest` | default; alias to the newest stable release |
| `typesafe/jev-1.13.0` | `JEV_1_13_0` | `jev-1.13.0` | pinned release for tuned thresholds |

Both specs use a 64,000-token context window (the documented combined state
and question budget), `IntelligenceScore::Five`, `SpeedTier::VeryFast`,
`CostTier::Low`, no effort, and no default effort. `jev-preview` is not in
the catalog because its target changes without notice.

## Request Contract

The adapter serializes one JSON body:

```json
{
  "state": { "ticket": "My payouts have been failing for 3 days." },
  "model": "jev-latest",
  "questions": {
    "is_urgent": {
      "type": "noul",
      "instructions": "Does `ticket` convey urgency?",
      "criteria": { "true": "Explicitly time-sensitive", "false": "No urgency" }
    },
    "department": {
      "type": "choice",
      "instructions": "Which team should handle `ticket`?",
      "criteria": { "billing": "Payments and refunds", "technical": null }
    },
    "frustration": {
      "type": "score",
      "instructions": "How frustrated is the author of `ticket`?",
      "criteria": ["Calm", "Frustrated", "Very angry"]
    }
  }
}
```

Mapping rules:

- `state` and every `JudgmentContent` value serialize verbatim.
- `Condition` becomes `type: "noul"`; `criteria` is omitted when absent, and
  `yes`/`no` map to the `true`/`false` keys, each omitted when absent.
- `Choice` options become the `criteria` object; `None` descriptions serialize
  as JSON `null`.
- `Score` levels become the ordered `criteria` array. Every entry is required
  content and therefore cannot serialize as JSON `null`.
- `model` is the configured provider model id. No other fields are sent.

The adapter performs no token counting. TypeSafe documents a combined budget
of 64k tokens for state plus all questions and 32k tokens for state plus the
longest question; exceeding it is a provider validation failure.

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
Unknown fields are ignored.

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
level key, is a non-retryable `Provider` error whose message is the fixed
text `malformed provider payload` followed by the decoder's own diagnostic.
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
applies `ai_models_core::price_usage`; TypeSafe currently bills input tokens
only.

## Error Contract

`JudgmentError` derives the shared `ErrorContract` and uses the
`[ai_interface/judgment]` display prefix:

| Variant | Meaning | Retry/fallback behavior |
| --- | --- | --- |
| `UnsupportedContent { json_type }` | local null, boolean, or number JSON content, with a typed `Null`, `Boolean`, or `Number` value | fix request; do not retry |
| `EmptyState` | local blank or empty state | fix request; do not retry |
| `NoQuestions` | local empty question map | fix request; do not retry |
| `InvalidQuestion` | local typed question problem | fix request; do not retry |
| `InvalidAnswer` | a provider answer failed the shared `validate_against` postconditions; carries provider, model id, answer id, and a typed `JudgmentAnswerProblem` | terminal |
| `RateLimited` | HTTP 429 | retry with backoff |
| `TransientProvider` | transport or auth-hook failure, HTTP 408/409/425, or 5xx including 529 overloaded | retry with backoff |
| `Provider` | HTTP 401/403 authentication, 422 validation, other statuses, or invalid response semantics | terminal |
| `Internal` | local serialization or invariant failure | internal failure |

All provider variants retain the provider, the configured model id, and the
available provider message. `Internal` uses the shared tracked
`InternalError` carrier.

Status classification uses `ai_models_core::classify_http_status`, a new
boundary-neutral helper introduced by this change. It returns
`Option<HttpFailureClass>`: `None` for statuses below 400 and otherwise
`RateLimited` (429), `Transient` (408, 409, 425, 5xx), or `Terminal`, so the
image, video, and judgment boundaries stop duplicating the same status table.
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
problems that carry provider-controlled labels are redacted the same way. A
failing auth hook cannot be sanitized because its credentials were never
discovered, so it is reported as `TransientProvider` with the fixed message
`authentication hook failed` and its own diagnostic is discarded.

## Required Verification

Credential-free tests must cover provider identity and serde round trips,
the judgment feature, DTO serde for every content, question, and answer shape
including structured instructions and descriptions, score key rejection,
local request validation, every answer-validation problem, the deterministic
mock through `DynJudgmentModel`, the generated `JudgmentModelMock`, spec
metadata, bearer auth and
endpoint selection, exact wire mapping for every question variant, response
mapping and every validation failure, usage normalization, status
classification, transport failures, and malformed bodies.

The credential-free smoke test must construct the default catalog model. The
ignored live suite calls every catalog spec through `DynJudgmentModel` when
`LIVE_JUDGMENT_API_KEY` is supplied. Formatting, file-length lint, Clippy,
workspace tests, `cargo xtask check`, commit and push, and post-push
`cargo xtask review` are required before handoff.

## References

- [TypeSafe API reference](https://docs.typesafe.ai/api)
- [TypeSafe models and aliases](https://docs.typesafe.ai/models)
- [TypeSafe primitives](https://docs.typesafe.ai/primitives)
- [TypeSafe advanced structure](https://docs.typesafe.ai/primitives/advanced)
- [TypeSafe confidence](https://docs.typesafe.ai/confidence)
- [Jev 1.13 jaggedness and context limits](https://docs.typesafe.ai/model-jaggedness/jev-1.13)
- [TypeSafe Python SDK retries](https://docs.typesafe.ai/sdk/python/api/retries)
