# TypeSafe Judgment Provider Protocol

Status: Implemented, except the option order and the level mapping that
[Milestone 2](../../plans/shared-judgment-and-openai-decisions.md#milestone-2-ordered-options-labelled-levels-and-refusals)
adds, and the support check and the content mapping that
[Milestone 3](../../plans/shared-judgment-and-openai-decisions.md#milestone-3-typed-content-conversation-input-and-the-support-check)
adds.

## Purpose And Status

Add TypeSafe's System One models (Jev) to the workspace as a first-class
provider. TypeSafe does not generate text: it evaluates a caller-supplied
`state` against a map of typed questions and returns calibrated probabilities.
That contract does not fit `ai_interface::Model`, so the
[judgment boundary protocol](judgment-boundary.md) defines a separate
provider-agnostic boundary, and this protocol defines its TypeSafe
implementation.

The
[Add TypeSafe judgment provider plan](../../plans/add-typesafe-judgment-provider.md)
records the implementation. Credentialed verification is defined by the
[live judgment API test protocol](live-judgment-api-tests.md). The
[TypeSafe judgment response protocol](typesafe-judgment-responses.md) defines
the response, usage, and error contract.

## Scope

The adapter covers one synchronous evaluation of text or JSON content against
one or more independent condition, choice, and score questions. All questions
in a request are sent in one provider call because TypeSafe evaluates them in
parallel over the same state and prices the state once.

Out of scope: image, audio, or video content and conversation input (Jev is
text only), and model listing through `GET /v1/models`. The
[judgment boundary scope](judgment-boundary.md#scope) lists the items that are
out of scope for every adapter.

## Ownership

- `ai-interface` owns `ModelFeature::Judgment`, the model spec types, the
  judgment DTOs, typed errors, the support types and the shared support
  check, the `JudgmentModel` trait, its dyn alias and unimock API, and the
  deterministic `MockJudgmentModel`.
- `ai-models-core` owns the shared HTTP status classifier reused by this
  provider.
- `ai-models-typesafe` owns the `typesafe` provider id, the Jev specs,
  authentication, the support value, wire mapping, local validation, response
  validation, usage normalization, and provider error translation.
- `json-http` owns the injected HTTP client, the auth hook boundaries, and the
  ordered-object value that keeps the option order.
- Composition roots own API-key retrieval, wrapper policy, pricing, and
  conversion of the concrete adapter into `DynJudgmentModel`.

## Shared Boundary

The [judgment boundary protocol](judgment-boundary.md) and its
[response page](judgment-responses.md) define the provider-agnostic
`JudgmentModel` boundary: the request and response DTOs, content and question
shapes, local request validation, the support check, response validation, and
the deterministic `MockJudgmentModel`.

## Provider Identity And Endpoint

The TypeSafe provider id is `typesafe`. Successful responses use
`provider: "typesafe"`.

By default, requests use:

```text
POST https://api.typesafe.ai/v1/systemone
Authorization: Bearer <api-key>
Content-Type: application/json
```

The [provider registry](provider-registry.md) builds the judgment adapter
from a spec and the provider config of `typesafe`. The registry is the
preferred way to build the adapter. Until the user answers the
[open constructor default-effort decision](../../plans/follow-ups/constructor-default-effort.md),
the adapter also keeps its constructors.
`TypeSafeJudgmentModel::new(http_client, model_id, api_key)` and
`with_auth(http_client, model_id, auth)` accept an injected
`DynJsonHttpClient` and an explicit API key or `DynJsonHttpAuth`;
`with_endpoint` and `with_timeout` override the endpoint and the 60-second
default request timeout. A catalog spec gives the provider model id to pass.
Any provider model id is accepted so callers can pin a versioned release
before the catalog lists it. The adapter must not read environment
variables, load deployment configuration, or resolve secrets.

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

## Support

The support value of the adapter takes text and JSON content parts and the
content input form. It takes no conversation parts.

- An image part, in the input or in instructions, fails the
  [support check](judgment-boundary.md#support-check) before transport.
- A conversation input fails the support check before transport.
- A failed check has the unsupported input kind and sends no request.

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
      "criteria": { "technical": null, "billing": "Payments and refunds" }
    },
    "frustration": {
      "type": "score",
      "instructions": "How frustrated is the author of `ticket`?",
      "criteria": [
        "Calm",
        "Frustrated",
        { "label": "Very angry", "description": "Threatens to cancel" }
      ]
    }
  }
}
```

The example shows the fields in a readable order. On the wire, every object
except a choice `criteria` object has sorted keys.

Mapping rules:

- The content input becomes `state`. Instructions use the same content
  mapping.
- Content with exactly one part sends that part alone. A text part becomes a
  JSON string, and a JSON part becomes its object or array unchanged.
- Other content sends a JSON array with one item for each part, in part
  order. Each item has the form of a single part. The items carry no type
  tag, so TypeSafe reads each JSON part as the caller wrote it.
- A text description becomes a JSON string, and a JSON description becomes
  its object or array.
- `Condition` becomes `type: "noul"`; `criteria` is omitted when absent, and
  `yes`/`no` map to the `true`/`false` keys, each omitted when absent.
- `Choice` options become the `criteria` object, with one key for each
  option label, in the caller's order. An absent description serializes as
  JSON `null`.
- Decision: the `criteria` object keeps the caller's order, so that TypeSafe
  reads the options in the order that the host chose, as every provider
  does. The rejected alternative sent the options sorted by label.
- `Score` levels become the ordered `criteria` array. A level without a
  description sends its label as a JSON string. A level with a description
  sends the object `{"label": ..., "description": ...}`, so that TypeSafe and
  OpenAI read the same rubric text. No entry is JSON `null`.
- `model` is the configured provider model id. No other fields are sent.

The adapter performs no token counting. TypeSafe documents a combined budget
of 64k tokens for state plus all questions and 32k tokens for state plus the
longest question; exceeding it is a provider validation failure.

## Required Verification

Credential-free tests must cover provider identity and serde round trips,
the judgment feature, DTO serde for every content, question, and answer shape
including structured instructions and descriptions, score key rejection,
local request validation, every answer-validation problem, the deterministic
mock through `DynJudgmentModel`, the generated `JudgmentModelMock`, spec
metadata, bearer auth and
endpoint selection, exact wire mapping for every question variant, the
content array of several parts, the level mapping with and without a
description, the caller's option order on the wire, the support check of an
image part and of a conversation input without a request, response
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
