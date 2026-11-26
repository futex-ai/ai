# Judgment Response Protocol

Status: Implemented, except the refusal answer that
[Milestone 2](../../plans/shared-judgment-and-openai-decisions.md#milestone-2-ordered-options-labelled-levels-and-refusals)
adds.

This protocol defines the answers and the response of the judgment boundary,
and the shared response check. The
[judgment boundary protocol](judgment-boundary.md) defines the requests, the
questions, local validation, the support check, and the mock.

## Answers

`JudgmentAnswer` is internally tagged by `type` and mirrors the question:

| Variant | Fields |
| --- | --- |
| `Condition` | `probability: f64` (probability of yes, zero to one) |
| `Choice` | `selected: String`, `probabilities: BTreeMap<String, f64>`, `confidence: f64` |
| `Score` | `expected: f64`, `probabilities: BTreeMap<u32, f64>`, `confidence: f64` |
| `Refused` | none: the model declined to answer this question |

- A refusal answers one question. The other answers and the usage of the
  call stay.
- `Refused` has no fields, because Decisions sends no reason. It serializes
  as `{"type": "refused"}`.
- `JudgmentAnswer::kind()` returns `Option<JudgmentQuestionKind>`: the kind
  of an answer, or `None` for a refusal.
- Decision: a refusal is the typed answer `JudgmentAnswer::Refused`. It is
  the smallest type change that keeps the other answers and the usage, and
  an exhaustive `match` must name it.
- The rejected alternatives failed the whole call, with or without the
  partial response, wrapped each answer in an outcome, or listed the refused
  ids beside the answers. With such a list, a caller that reads only the
  answer map sees a missing id, not a refusal.

`JudgmentResponse` contains `provider`, the configured `model_id`, the
provider-reported `resolved_model_id`, `answers` keyed by the request ids, and
`ModelUsage`. Condition answers carry no confidence; a probability near 0.5
means yes and no are equally likely, not medium intensity. Choice and score
confidence summarize distribution concentration only.

Score probabilities are keyed by decimal strings on the wire. Deserialization
rejects keys that are not canonical unsigned decimal indexes and rejects two
keys that resolve to the same index.

## Response Check

A successful response must satisfy the pure
`JudgmentResponse::validate_against(&JudgmentRequest)` postconditions. Every
adapter applies it after mapping a provider body, and `MockJudgmentModel`
output satisfies it. The validator checks, for every requested id, and
returns `InvalidAnswer { provider, model_id, id, problem }` with a typed
`JudgmentAnswerProblem` on the first failure:

| Problem | Trigger |
| --- | --- |
| `Missing` | no answer for a requested id |
| `KindMismatch { expected, actual }` | an answer that is not a refusal has a kind other than the question kind |
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

A refusal passes the check for any question kind. The validator skips the
kind check and every other check for it.

The two consistency checks exist because TypeSafe defines `choice` as the
highest-probability option and `score` as the probability-weighted level. A
response that contradicts its own distribution is malformed, so the validator
rejects it instead of letting `selected` and `probabilities`, or `expected`
and `probabilities`, disagree. The tolerances absorb ties and provider
rounding. Within one answer the checks run in this order: kind, membership,
ranges, distribution sum, selection or expectation consistency, confidence.

Answers for ids the request did not send are ignored.
