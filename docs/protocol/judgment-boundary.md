# Judgment Boundary Protocol

## Purpose And Status

This protocol defines the provider-agnostic judgment boundary, the shared
judgment transport, and the typed error contract that every judgment adapter
implements. It is implemented by two providers. The
[TypeSafe judgment provider protocol](typesafe-judgment-provider.md) and the
[OpenAI judgment provider protocol](openai-judgment-provider.md) define only
their own identity, catalog, wire mapping, usage, and failure-message rules.
The [Add TypeSafe judgment provider plan](../../plans/add-typesafe-judgment-provider.md)
and the [Add OpenAI Decisions judgment provider plan](../../plans/add-openai-judgment-provider.md)
record the implementation.

## Ownership

- `ai-interface` owns `ProviderKind::TypeSafe`, `ModelFeature::Judgment`, the
  judgment DTOs, typed errors, the `JudgmentModel` trait, its dyn alias and
  unimock API, and the deterministic `MockJudgmentModel`.
- `ai-models-core` owns the shared HTTP status classifier reused by this
  provider, catalog metadata types, and the provider-neutral judgment helpers
  shared with the OpenAI adapter: credential redaction (`redact_secrets`,
  `secrets_from_headers`), failure-body decoding, status and transport error
  classification into `JudgmentError`, and redaction of provider-controlled
  `InvalidAnswer` labels.
- `ai-models-core` also owns the shared judgment transport defined below.

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
  questions. Ids are for code only. TypeSafe never sends them to the model;
  the OpenAI adapter sends them as the question `name` so answers can be
  matched by name. An empty map is the local `NoQuestions` error.

`JudgmentQuestion` is internally tagged by `type` with snake-case variants:

| Variant | Fields | Meaning |
| --- | --- | --- |
| `Condition` | `instructions: JudgmentContent`, `criteria: Option<JudgmentConditionCriteria>` | whether a condition holds; answer is the probability of yes |
| `Choice` | `instructions: JudgmentContent`, `options: BTreeMap<String, Option<JudgmentContent>>` | pick one option; answer includes the full distribution |
| `Score` | `instructions: JudgmentContent`, `levels: Vec<JudgmentContent>` | position on an ordered rubric; index zero is the first level |

`JudgmentConditionCriteria` has optional `yes` and `no` descriptions, and
choice option descriptions are optional. Every score level requires a
description because TypeSafe rejects `null` entries in a score rubric.

Convenience constructors only build values and never validate:
`JudgmentQuestion::condition(instructions, criteria)`,
`JudgmentQuestion::choice(instructions, options)`,
`JudgmentQuestion::score(instructions, levels)`, and
`JudgmentConditionCriteria::new(yes, no)`. Instructions accept
`impl Into<JudgmentContent>`. Condition and choice descriptions are plain
`Option<JudgmentContent>` values so a bare `None` infers cleanly, for example
`JudgmentConditionCriteria::new(Some("yes".into()), None)`. Options accept
any iterator of `(label, Option<JudgmentContent>)` pairs. Levels accept any
iterator whose items convert into `JudgmentContent`, so string slices can
define a score rubric directly.

`JudgmentQuestionKind` (`Condition`, `Choice`, `Score`) names a question or
answer shape; `JudgmentQuestion::kind()` and `JudgmentAnswer::kind()` return
it.

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

Score probabilities are keyed by decimal strings on the wire. Deserialization
rejects keys that are not canonical unsigned decimal indexes and rejects two
keys that resolve to the same index.

A successful response must satisfy the pure
`JudgmentResponse::validate_against(&JudgmentRequest)` postconditions. Every
adapter applies it after mapping a provider body, and `MockJudgmentModel`
output satisfies it. The validator checks, for every requested id, and
returns `InvalidAnswer { provider, model_id, id, problem }` with a typed
`JudgmentAnswerProblem` on the first failure:

| Problem | Trigger |
| --- | --- |
| `Missing` | no answer for a requested id |
| `KindMismatch { expected, actual }` | answer kind differs from the question kind |
| `ProbabilityOutOfRange { value }` | a condition, option, or level probability is not finite within zero to one |
| `ConfidenceOutOfRange { value }` | a confidence is not finite within zero to one |
| `UnknownOption { label }` | a selected or probability label is not a requested option |
| `MissingOption { label }` | a requested option has no probability |
| `UnknownLevel { index }` | a probability index is at or beyond the level count |
| `MissingLevel { index }` | a level index has no probability |
| `ExpectedOutOfRange { value }` | a score expectation is not finite within zero and the last level index |
| `DistributionSum { sum }` | option or level probabilities differ from one by more than `PROBABILITY_SUM_TOLERANCE` (0.02) |
| `SelectedNotMaximal { selected, maximal }` | the selected option's probability is more than `SELECTED_PROBABILITY_TOLERANCE` (0.02) below the highest option probability |
| `ExpectedInconsistent { expected, weighted }` | a score expectation differs from the probability-weighted level, computed over the sum-normalized distribution, by more than `EXPECTED_SCORE_TOLERANCE` (0.1) |

The two consistency checks exist because TypeSafe defines `choice` as the
highest-probability option and `score` as the probability-weighted level. A
response that contradicts its own distribution is malformed, so the validator
rejects it instead of letting `selected` and `probabilities`, or `expected`
and `probabilities`, disagree. The tolerances absorb ties and provider
rounding. Within one answer the checks run in this order: kind, membership,
ranges, distribution sum, selection or expectation consistency, confidence.

Answers for ids the request did not send are ignored.

`MockJudgmentModel` is deterministic: it applies the same local validation and
answers every condition with probability one, every choice with its first
option at probability one, and every score with level zero at probability one.
It reports `provider: "mock"`, `model_id` and `resolved_model_id` of
`mock-judgment`, confidence one, and `ModelUsage::default()`.

## Shared Transport

`ai-models-core` sends every judgment call through one transport, so an
adapter adds only its endpoint constant, wire mapping, failure-message
extractor, and response parser:

- `JudgmentFailureMessage = fn(&serde_json::Value) -> String` is the pure
  extractor from a decoded failure body to the provider message. Each adapter
  owns one. It returns a JSON string body unchanged.
- `JudgmentEndpoint { provider, url, timeout, failure_message }` is a `Copy`
  constant per adapter with a `&'static str` provider and URL: OpenAI uses
  `openai`, `https://api.openai.com/v1/decisions`, and 60 s; TypeSafe uses
  `typesafe`, `https://api.typesafe.ai/v1/systemone`, and 60 s.
- `JudgmentHttpResponse { body: Vec<u8>, secrets: Vec<String> }` holds the
  `2xx` body bytes and the redaction set of the transmitted credentials.
- `JudgmentTransport` is a `Send + Sync` async trait with
  `post(&self, body: serde_json::Value) -> JudgmentResult<JudgmentHttpResponse>`
  and `unimock(api = JudgmentTransportMock)` behind the `test-support` feature.
- `HttpJudgmentTransport` is the `Clone` implementation.
  `new(endpoint, model_id, http_client, auth)` takes a `DynJsonHttpClient` and
  a `DynJsonHttpAuth`. `with_explicit_secret` adds a redaction secret,
  `with_endpoint` and `with_timeout` override the URL and timeout, and
  `provider()`, `model_id()`, `endpoint()`, and `timeout()` read them back.
- `malformed_judgment_payload(provider, model_id, detail)` returns the
  terminal `Provider` error whose message is `malformed provider payload: `
  followed by the detail.

`post` applies the auth hook exactly once to an empty header map. A failing
hook returns `TransientProvider` with `AUTH_HOOK_FAILED` and makes no
transport call. `post` derives the secrets with `secrets_from_headers` from
the explicit secrets and the applied headers, sends one `POST` with exactly
those headers, the timeout, and the JSON body, and classifies builder and send
failures with `classify_judgment_request_error`. Only statuses in `200..300`
are success. Any other status runs `decode_failure_body`, the endpoint's
`failure_message`, and `classify_judgment_status`. The transport logs nothing.

Every adapter's `judge` runs these steps in order: `JudgmentRequest::validate`;
build the provider body, where a failure is `Internal` and is reported before
the auth hook runs; serialize the body to a `serde_json::Value` (`Internal` on
failure); `post`; parse the `2xx` body; and pass any parse error through
`redact_judgment_error` with the returned secrets.

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
| `Refused` | the provider declined to answer one question; carries provider, model id, and the question id. TypeSafe never returns it; see the [OpenAI protocol](openai-judgment-provider.md) | terminal; route the state elsewhere |
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

Error bodies are decoded best-effort: each provider protocol names the fields
that supply the message; any other body, including JSON `null`, is retained
as compact JSON or text; an empty body falls back to `HTTP <status>`.

API keys and auth headers must never appear in errors or diagnostics. The
adapter enforces this rather than assuming it, through the shared
`ai-models-core` judgment helpers that the OpenAI adapter also uses. It applies the injected auth
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

## Related Protocols

- [TypeSafe judgment provider](typesafe-judgment-provider.md) and
  [OpenAI judgment provider](openai-judgment-provider.md) define the provider
  catalogs, wire mappings, usage, and failure messages.
- [Live judgment API tests](live-judgment-api-tests.md) defines the
  credentialed catalog verification for every judgment adapter.
