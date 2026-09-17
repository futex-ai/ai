# TypeSafe Judgment Provider Protocol

## Purpose And Status

Add TypeSafe's System One models (Jev) to the workspace as a first-class
provider. TypeSafe does not generate text: it evaluates a caller-supplied
`state` against a map of typed questions and returns calibrated probabilities.
That contract does not fit `ai_interface::Model`, so this protocol defines a
separate provider-agnostic judgment boundary and the TypeSafe implementation
of it.

This contract is planned. The
[Add TypeSafe judgment provider plan](../../plans/add-typesafe-judgment-provider.md)
tracks the implementation. Credentialed verification is defined by the
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

- `ai-interface` owns `ProviderKind::TypeSafe`, `ModelFeature::Judgment`, the
  judgment DTOs, typed errors, the `JudgmentModel` trait, its dyn alias and
  unimock API, and the deterministic `MockJudgmentModel`.
- `ai-models-core` owns the shared HTTP status classifier reused by this
  provider, plus catalog metadata types.
- `ai-models-typesafe` owns the Jev catalog, authentication, wire mapping,
  local validation, response validation, usage normalization, and provider
  error translation.
- `json-http` owns the injected HTTP client and auth hook boundaries.
- Composition roots own API-key retrieval, wrapper policy, pricing, and
  conversion of the concrete adapter into `DynJudgmentModel`.

## Shared Boundary

`JudgmentModel::judge` accepts a `JudgmentRequest` and returns a
`JudgmentResponse` asynchronously. Implementations are `Send + Sync` and are
normally held through `DynJudgmentModel = Arc<dyn JudgmentModel>`. The trait
carries `unimock(api = JudgmentModelMock)` behind the `test-support` feature.

`JudgmentContent` is the shared content type for state, instructions, and
descriptions. It serializes untagged as one of:

| Variant | JSON |
| --- | --- |
| `Text(String)` | string |
| `Object(serde_json::Map<String, Value>)` | object |
| `Array(Vec<Value>)` | array |

`JudgmentContent` implements `From<&str>`, `From<String>`, and
`TryFrom<serde_json::Value>`. The fallible conversion rejects null, boolean,
and number values with the local `UnsupportedContent` error so callers can
serialize their own typed state with `serde_json::to_value` or `json!`.

`JudgmentRequest` contains:

- `state: JudgmentContent`: the material to evaluate. Whitespace-only text,
  an empty object, or an empty array is the local `EmptyState` error.
- `questions: BTreeMap<String, JudgmentQuestion>`: caller-chosen ids mapped to
  questions. Ids are for code only and are never sent to the model. An empty
  map is the local `NoQuestions` error.

`JudgmentQuestion` is internally tagged by `type` with snake-case variants:

| Variant | Fields | Meaning |
| --- | --- | --- |
| `Condition` | `instructions: JudgmentContent`, `criteria: Option<JudgmentConditionCriteria>` | whether a condition holds; answer is the probability of yes |
| `Choice` | `instructions: JudgmentContent`, `options: BTreeMap<String, Option<JudgmentContent>>` | pick one option; answer includes the full distribution |
| `Score` | `instructions: JudgmentContent`, `levels: Vec<Option<JudgmentContent>>` | position on an ordered rubric; index zero is the first level |

`JudgmentConditionCriteria` has optional `yes` and `no` descriptions.
`None` descriptions leave an option or level undescribed.

Convenience constructors only build values and never validate:
`JudgmentQuestion::condition(instructions, criteria)`,
`JudgmentQuestion::choice(instructions, options)`,
`JudgmentQuestion::score(instructions, levels)`, and
`JudgmentConditionCriteria::new(yes, no)`. Instructions and descriptions
accept `impl Into<JudgmentContent>`; options accept any iterator of
`(label, Option<JudgmentContent>)` pairs and levels any iterator of
`Option<JudgmentContent>`.

Local validation is the pure `JudgmentRequest::validate` method. The mock and
every provider adapter call it before any transport call. It returns
`EmptyState`, `NoQuestions`, or `InvalidQuestion { id, problem }` with a typed
`JudgmentQuestionProblem`:

| Problem | Trigger |
| --- | --- |
| `BlankId` | the question id is empty or whitespace |
| `NoOptions` | a choice has no options |
| `BlankOptionLabel` | a choice option label is empty or whitespace |
| `TooFewLevels { levels }` | a score has fewer than two levels |

`JudgmentAnswer` is internally tagged by `type` and mirrors the question:

| Variant | Fields |
| --- | --- |
| `Condition` | `probability: f64` (probability of yes, zero to one) |
| `Choice` | `selected: String`, `probabilities: BTreeMap<String, f64>`, `confidence: f64` |
| `Score` | `expected: f64`, `probabilities: BTreeMap<u32, f64>`, `confidence: f64` |

`JudgmentResponse` contains `provider`, the configured `model_id`, the
provider-reported `resolved_model_id`, `answers` keyed by the request ids, and
`ModelUsage`. Condition answers carry no confidence; a probability near 0.5
means yes and no are equally likely, not medium intensity. Choice and score
confidence summarize distribution concentration only.

`MockJudgmentModel` is deterministic: it applies the same local validation and
answers every condition with probability one, every choice with its first
option at probability one, and every score with level zero at probability one.
It reports `provider: "mock"`, `model_id` and `resolved_model_id` of
`mock-judgment`, confidence one, and `ModelUsage::default()`.

## Provider Identity And Endpoint

`ProviderKind::TypeSafe` serializes and parses as `typesafe`. Successful
responses use `provider: "typesafe"`.

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
default request timeout. Any provider model id is accepted so callers can pin
a versioned release before the catalog lists it. The adapter must not read
environment variables, load config, or resolve secrets.

## Catalog

Every entry advertises only `ModelFeature::Judgment`, so chat, image, and
video suites exclude it automatically.

| Catalog id | Constant | Provider model id | Notes |
| --- | --- | --- | --- |
| `jev-latest` | `JEV_LATEST` | `jev-latest` | default; alias to the newest stable release |
| `jev-1.13.0` | `JEV_1_13_0` | `jev-1.13.0` | pinned release for tuned thresholds |

Both entries use a 64,000-token context window (the documented combined state
and question budget), `IntelligenceScore::Five`, `SpeedTier::VeryFast`,
`CostTier::Low`, and `ThinkingLevel::Disabled`. `jev-preview` is not in the
catalog because its target changes without notice.

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
- `Score` levels become the ordered `criteria` array; `None` levels serialize
  as JSON `null`.
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

Response validation failures are non-retryable `Provider` errors: a missing
answer for a requested id, an answer whose type differs from its question, a
selected choice or probability label outside the requested options, a score
probability index at or beyond the level count, or a body that fails to
deserialize. Answers for ids the request did not send are ignored.

## Usage

`usage.input_tokens` and `usage.output_tokens` map to `ModelUsage`
`input_tokens` and `output_tokens`; `total_tokens` is their saturating sum and
the cached and reasoning buckets remain zero. Missing usage is
`ModelUsage::default()`. Cost lines remain empty until a composition root
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
`message`; a `detail` string is used directly; any other body is retained as
compact JSON or text. TypeSafe currently answers a missing key with `403` and
`{"detail": {"error_type": "authentication_error", "message": "..."}}`. API
keys and auth headers must never appear in errors or diagnostics.

## Required Verification

Credential-free tests must cover provider config/serde round trips, the
judgment feature, DTO serde for every content, question, and answer shape,
local validation, the deterministic mock, catalog metadata, bearer auth and
endpoint selection, exact wire mapping for every question variant, response
mapping and every validation failure, usage normalization, status
classification, transport failures, and malformed bodies.

The credential-free smoke test must construct the default catalog model. The
ignored live suite calls every catalog entry through `DynJudgmentModel` when
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
