# Judgment Boundary Protocol

This protocol defines the provider-agnostic judgment boundary of
`ai-interface`. The
[TypeSafe judgment provider protocol](typesafe-judgment-provider.md) gives
the purpose, status, scope, and ownership of this boundary, and defines its
TypeSafe implementation.

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
