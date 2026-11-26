# Judgment Boundary Protocol

Status: Implemented, except the ordered options and the labelled levels
that
[Milestone 2](../../plans/shared-judgment-and-openai-decisions.md#milestone-2-ordered-options-labelled-levels-and-refusals)
adds, and the typed content, the description type, the conversation input,
and the support check that
[Milestone 3](../../plans/shared-judgment-and-openai-decisions.md#milestone-3-typed-content-conversation-input-and-the-support-check)
adds.

This protocol defines the provider-agnostic judgment boundary of
`ai-interface`: its requests, questions, local validation, support check, and
mock. A judgment model evaluates one input against typed questions and
returns calibrated probabilities. The
[judgment response protocol](judgment-responses.md) defines the answers and
the response check. The
[TypeSafe judgment provider protocol](typesafe-judgment-provider.md) and the
[OpenAI judgment provider protocol](openai-judgment-provider.md) define the
two adapters. The TypeSafe protocol also gives the ownership of the boundary.

## Scope

- One call evaluates one input against one or more independent condition,
  choice, and score questions.
- The input is content or a conversation. Content holds text, JSON, and
  inline images.
- Decision: one request type holds text, JSON, images, and conversations for
  every provider. An adapter rejects a part that its provider cannot take,
  with a typed error before transport. The rejected alternatives limited the
  request to text and JSON, or to text, JSON, and images.
- Decision: the boundary keeps its `Judgment` names, such as `JudgmentModel`
  and `JudgmentAnswer`, also for the OpenAI Decisions API. A rename changes
  every caller without a change in behaviour.
- Out of scope: audio and video content parts, hosted image URLs, prompt-cache
  controls, an end-user safety identifier, boolean choice values, token
  counting, automatic question fan-out across requests, and retry,
  concurrency, pricing, and confidence-threshold policy. Composition roots
  own those policies.

## Shared Boundary

`JudgmentModel::judge` accepts a `JudgmentRequest` and returns a
`JudgmentResponse` asynchronously. Implementations are `Send + Sync` and are
normally held through `DynJudgmentModel = Arc<dyn JudgmentModel>`. The trait
carries `unimock(api = JudgmentModelMock)` behind the `test-support` feature.

## Content

`JudgmentContent` is an ordered list of typed `JudgmentPart` values. The
input and the instructions of every question use it.

| Part | `type` | Fields | Meaning |
| --- | --- | --- | --- |
| `Text` | `text` | `text` | plain text |
| `Json` | `json` | `value` | one JSON object or one JSON array |
| `Image` | `image` | `mime_type`, `data_base64` | one inline image, with the fields of the chat [image part](conversation-parts.md#parts) |

- `JudgmentPart` is internally tagged by `type`. `JudgmentContent`
  serializes as the JSON array of its parts.
- Deserialization rejects a JSON part whose value is not an object or an
  array.
- The contracts do not check MIME types or image sizes.
- `JudgmentContent` implements `From<&str>` and `From<String>`. Each gives
  one text part.
- `JudgmentContent` also implements `TryFrom<serde_json::Value>`. A string
  gives one text part, and an object or an array gives one JSON part. The
  conversion rejects null, boolean, and number values with the local
  `UnsupportedContent` error. Callers can then serialize their own typed
  input with `serde_json::to_value` or `json!`.
- Decision: images are inline base64 data only. The rejected alternative
  added a part for a hosted image URL. The chat parts have no URL form, and
  such a part can come later without a breaking change.

`JudgmentDescription` is the type of every description: the `yes` and `no`
descriptions of a condition, and the descriptions of options and levels. It
is `Text(String)`, or `Json` with an object or an array.

- It serializes untagged: a string for text, and an object or an array for
  JSON.
- It implements the same three conversions as `JudgmentContent`, with the
  same rejection of null, boolean, and number values.
- Decision: a description holds text or JSON and no image, because no
  provider takes an image in a description. The rejected alternative used
  `JudgmentContent` for descriptions.

## Requests

`JudgmentRequest` contains:

- `input: JudgmentInput`: the material to evaluate.
- `questions: BTreeMap<String, JudgmentQuestion>`: caller-chosen ids mapped to
  questions. An empty map is the local `NoQuestions` error.

`JudgmentInput` is one of these values:

- `Content(JudgmentContent)`: content parts.
- `Conversation { system, messages }`: an optional system text and ordered
  `ConversationMessage` values. The
  [conversation parts protocol](conversation-parts.md) defines the messages,
  their roles, and their parts.

Rules for the input:

- `JudgmentInput` serializes untagged: content as the JSON array of its
  parts, and a conversation as an object with `messages` and an optional
  `system`. Each message has the stored format of the conversation parts
  protocol.
- An empty input is the local `EmptyState` error. Content is empty when it
  has no parts, or when each part is whitespace-only text or an empty JSON
  object or array. A conversation is empty when it has no messages.
- A message that breaks the [role table](conversation-parts.md#messages) of
  the conversation parts protocol fails local validation with the invalid
  request kind.
- A blank system text counts as absent, as the
  [system instruction rules](provider-call-controls.md#system-instructions)
  of model calls say.
- Decision: a conversation reuses `ConversationMessage`, so that hosts keep
  one conversation type. The rejected alternative was a message type that
  the judgment boundary owns. A host can put JSON in a text part.
- An id is a key for code. TypeSafe sends the ids as the keys of its
  `questions` object. The OpenAI adapter sends opaque names, so that the ids
  never reach its model.

## Questions

`JudgmentQuestion` is internally tagged by `type` with snake-case variants:

| Variant | Fields | Meaning |
| --- | --- | --- |
| `Condition` | `instructions: JudgmentContent`, `criteria: Option<JudgmentConditionCriteria>` | whether a condition holds; answer is the probability of yes |
| `Choice` | `instructions: JudgmentContent`, `options: Vec<JudgmentOption>` | pick one option; answer includes the full distribution |
| `Score` | `instructions: JudgmentContent`, `levels: Vec<JudgmentLevel>` | position on an ordered rubric; index zero is the first level |

- `JudgmentConditionCriteria` has optional `yes` and `no` descriptions.
- `JudgmentOption` has a `label` and an optional `description`. The options
  keep the order of the caller.
- `JudgmentLevel` has a required `label` and an optional `description`.
  Decisions requires a label for each level.
- `JudgmentOption` and `JudgmentLevel` serialize as objects with `label` and
  `description`. An absent description is left out.
- Decision: labels are strings. The rejected alternative added the boolean
  choice values that Decisions takes. The condition question already covers
  yes and no.
- Decision: the options are an ordered list, and a repeated label fails
  local validation. The model reads the options as prompt text, so a host
  must be able to put a fallback option such as "other" last. The rejected
  alternatives kept a map sorted by label, or rejected a repeated label in
  the constructor of an ordered options type. That constructor breaks the
  rule that constructors never validate.

Convenience constructors only build values and never validate:
`JudgmentQuestion::condition(instructions, criteria)`,
`JudgmentQuestion::choice(instructions, options)`,
`JudgmentQuestion::score(instructions, levels)`,
`JudgmentConditionCriteria::new(yes, no)`,
`JudgmentOption::new(label, description)`, and
`JudgmentLevel::new(label, description)`. Instructions accept
`impl Into<JudgmentContent>`. Descriptions are plain
`Option<JudgmentDescription>` values so a bare `None` infers cleanly, for
example `JudgmentConditionCriteria::new(Some("yes".into()), None)`. Options
accept any iterator of `(label, Option<JudgmentDescription>)` pairs and keep
the order of the iterator. Levels accept any iterator whose items convert
into `JudgmentLevel`. A string converts into a level with that label and no
description, so string slices can define a score rubric directly.

`JudgmentQuestionKind` (`Condition`, `Choice`, `Score`) names a question or
answer shape. `JudgmentQuestion::kind()` returns it. The
[answer rules](judgment-responses.md#answers) give the kind of an answer.

## Local Validation

Local validation is the pure `JudgmentRequest::validate` method. The mock and
every provider adapter call it before any transport call. It returns
`EmptyState`, `NoQuestions`, the invalid request error of a message that
breaks the role table, or `InvalidQuestion { id, problem }` with a typed
`JudgmentQuestionProblem`:

| Problem | Trigger |
| --- | --- |
| `BlankId` | the question id is empty or whitespace |
| `NoOptions` | a choice has no options |
| `BlankOptionLabel` | a choice option label is empty or whitespace |
| `DuplicateOptionLabel` | two options of a choice have the same label, compared exactly |
| `TooFewLevels { levels }` | a score has fewer than two levels |

## Support Check

Each adapter declares a `JudgmentSupport` value. It lists the content part
kinds, the input forms, and the conversation part kinds that the provider of
the adapter takes.

- One shared support check walks the whole request: the input and the
  instructions of every question.
- Every adapter applies the check after local validation and before
  transport.
- A content part kind, an input form, or a conversation part kind that the
  value does not list fails the call with `JudgmentError::UnsupportedInput`.
  A failed check sends no request.
- `UnsupportedInput` has the unsupported input kind of the
  [error kinds table](model-policies.md#error-kinds). It names the provider,
  the model id, and the unsupported feature as a typed value.
- Image support is part of the support value of each adapter. The
  `ModelFeature::Vision` feature of a spec is routing data, and the check
  does not read it.
- Decision: one shared check over the whole request decides what an adapter
  takes, as the [capability check](model-capabilities.md#capability-check)
  does for model controls. The rejected alternatives were a check in each
  adapter, and a gate on the `Vision` feature, which turns routing data into
  a rejection.

## Mock

`MockJudgmentModel` is deterministic: it applies the same local validation and
answers every condition with probability one, every choice with its first
option in the caller's order at probability one, and every score with level
zero at probability one. It never refuses. Its support value takes every part
kind and both input forms. It reports `provider: "mock"`, `model_id` and
`resolved_model_id` of `mock-judgment`, confidence one, and
`ModelUsage::default()`.
