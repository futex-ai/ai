# OpenAI Judgment Provider Protocol

## Purpose

Hosts call the OpenAI Decisions API through the shared
[judgment boundary](judgment-boundary.md). Decisions evaluates text, JSON,
images, and conversations. It answers predicate, choice, and score questions
with probabilities. It can refuse one question and answer the others. The
[OpenAI judgment response protocol](openai-judgment-responses.md) defines the
response mapping, the usage, the errors, the timeout, and the billing notes.

- Decision: the Decisions adapter implements `JudgmentModel`, so that hosts
  keep one swappable `DynJudgmentModel` and one mock. The rejected
  alternatives were a separate client with OpenAI-shaped types, that client
  with a judgment adapter on top, and an adapter behind a boundary without
  images and conversations.
- The chat `Model` trait does not carry Decisions, because it hides the
  distributions and Decisions does not stream. An emulation through the
  Responses API does not use the Decisions API or its pricing.

## Scope

- One judgment call sends one `POST /v1/decisions` request with every
  question of the judgment request.
- Out of scope: prompt-cache controls, the end-user `safety_identifier`,
  hosted image URLs, the image `detail` field, custom tool calls, the
  organization usage endpoint, and automatic splitting of more than 400
  questions. The [judgment boundary scope](judgment-boundary.md#scope) lists
  the items that are out of scope for every adapter.

## Ownership

- `ai-models-openai` owns `OpenAiJudgmentModel`, the `GPT_6_LUNA` spec, the
  support value, the wire mapping, usage normalization, and error
  translation.
- The adapter shares the `openai` provider id and the provider config with
  the other OpenAI adapters, and the error code table with the OpenAI chat
  adapter.
- `ai-interface` owns the judgment types, local validation, the support
  check, and the response check.
- `ai-models-registry` builds the adapter from a spec and the `openai`
  provider config.
- Composition roots own API-key retrieval, retry, and pricing.
- Decision: the adapter lives in `ai-models-openai`, because the
  [registry](provider-registry.md#building-adapters) has one Cargo feature
  for each provider. The rejected alternative was a separate
  `ai-models-openai-decisions` crate, which needs a second `openai` feature.

## Endpoint And Construction

```text
POST <base URL>/v1/decisions
Authorization: Bearer <api-key>
Content-Type: application/json
```

- The default base URL is `https://api.openai.com`. The
  [provider registry](provider-registry.md) gives the base URL, auth, extra
  header, and timeout rules of the `openai` config.
- `OpenAiJudgmentModel::from_config(http, spec, config)` is the only
  constructor. The registry calls it. Building sends no request.
- Decision: the adapter has no `new` or `with_auth` constructor, because
  [remove the per-provider model constructors](../../plans/follow-ups/remove-provider-constructors.md)
  proposes the registry as the one factory. The rejected alternative copied
  the constructors of the other OpenAI adapters.
- The adapter sends the provider model id of its spec as `model`. It accepts
  any provider model id, so that a custom spec can name another Decisions
  model.
- The adapter does not read environment variables, load deployment
  configuration, or resolve secrets.

## Spec

| Text id | Constant | Provider model id |
| --- | --- | --- |
| `openai/gpt-6-luna` | `GPT_6_LUNA` | `gpt-6-luna` |

- The spec advertises `ModelFeature::Judgment` and `ModelFeature::Vision`.
- It lists no effort and has no default effort.
- Its routing tiers have intelligence score 5, the very fast speed tier, and
  the low cost tier.
- Its context window and its max output are 0, because the Decisions docs
  give no token limits.
- It holds no price. Hosts configure prices, as the
  [model spec rules](model-capabilities.md#model-specs) require.
- The OpenAI catalog lists the spec once. The
  [catalog rules](model-capabilities.md#catalogs) say how the spec gains chat
  features later.

## Support

| Place | Taken | Fails the support check |
| --- | --- | --- |
| Content parts of the input and of instructions | text, JSON, image | none |
| Input forms | content, conversation | none |
| Conversation parts | text, image, tool call, tool result, reasoning | video |

- The adapter takes images for every spec. It does not read
  `ModelFeature::Vision`.
- A failed check has the unsupported input kind and sends no request.
- The adapter does not check MIME types or image sizes. OpenAI rejects an
  image that it cannot read.

## Request Mapping

The adapter serializes one JSON body. In this example, the question ids are
`department`, `frustration`, and `is_urgent`. The last one is a condition
with criteria.

```json
{
  "model": "gpt-6-luna",
  "input": [{
    "role": "user",
    "content": [
      { "type": "input_text", "text": "Evaluate the ticket." },
      { "type": "input_json", "data": { "ticket": "My payouts failed for 3 days." } }
    ]
  }],
  "questions": [
    { "type": "choice", "name": "q0",
      "instructions": "Which team should handle the ticket?",
      "choices": [
        { "value": "billing", "description": "Payments and refunds" },
        { "value": "other" }
      ] },
    { "type": "score", "name": "q1",
      "instructions": "How frustrated is the author?",
      "levels": [
        { "label": "Calm" },
        { "label": "Very angry", "description": "Threatens to cancel" }
      ] },
    { "type": "choice", "name": "q2",
      "instructions": "Does the ticket convey urgency?",
      "choices": [
        { "value": true, "description": "Explicitly time-sensitive" },
        { "value": false, "description": "No urgency" }
      ] }
  ]
}
```

The example shows the fields in a readable order. On the wire, the keys of
every object are sorted.

Content:

- Content that is one text part becomes the `input` string.
- Other content becomes one `user` message with one part for each content
  part, in order.
- A text part becomes `input_text`. An image part becomes `input_image`, with
  the data URL `data:<mime_type>;base64,<data_base64>`.
- A JSON object, or a JSON array of objects, becomes `input_json`. Another
  JSON array becomes `input_text` with its compact JSON, because
  `input_json` takes only objects.

Conversation:

- A system text that is not blank becomes the first message, with the
  `developer` role.
- The messages keep their order. A user message becomes a `user` message,
  and its parts map as content parts do. A user message without parts sends
  empty content.
- An assistant message maps in part order. Its text becomes the content of
  an `assistant` message. Each tool call becomes a `function_call` item with
  the call id, the tool name, and the compact JSON of the arguments.
- A tool message becomes a `function_call_output` item with the call id and
  the content of its result. The output needs an earlier `function_call`
  item with the same call id. A result without its call fails shared local
  validation with `InvalidRequest` before transport.
- The adapter sends no reasoning part, no provider state, and no participant
  name. It sends each part in its portable form, also when the part carries
  OpenAI state. The
  [replay rules](conversation-parts.md#replay-and-skip-rules) do not apply,
  because a decision continues no conversation.

Questions:

- The adapter names the questions `q0`, `q1`, and so on, in the sorted order
  of their ids. Decision: opaque names keep the caller ids away from the model.
  The rejected alternative sent the ids as names.
- Instructions map as content does, without a message: one text part becomes
  a string, and other instructions become an array of `input_text`,
  `input_json`, and `input_image` parts.
- A condition without criteria becomes a `predicate`.
- A condition with criteria becomes a `choice` between the boolean values
  `true` and `false`, in that order. The `yes` description goes with `true`,
  and the `no` description goes with `false`.
- A choice becomes a `choice` with one string `value` for each option, in
  the caller's order.
- A score becomes a `score` with one `levels` entry for each level, in order.
  Each entry has the `label` and the `description` of its level.
- A text description becomes the `description` string. A JSON description
  becomes its compact JSON text, because Decisions takes only a string. An
  absent description is left out.

Other fields:

- The adapter sends no other field: no `prompt_cache_key`,
  `prompt_cache_options`, `prompt_cache_breakpoint`, `safety_identifier`, or
  image `detail`. OpenAI caches the shared input by default.
- The adapter checks none of the OpenAI limits, such as 400 questions, 255
  choices, or 10 levels. OpenAI rejects a request over a limit, and that
  rejection has the invalid request kind.

## Required Verification

Credential-free tests must cover the request mapping of each input form,
content part, description, and question kind, and the opaque names. They
must cover the support check of a video part without a request. They must
cover the response mapping of each answer kind, a refusal, a repeated name,
a repeated distribution value, and a boolean value in a string choice. They
must also cover the usage subtraction, the mapping of error codes and
statuses, the timeout, and the redaction of the API key and request content.

The credential-free smoke test builds the adapter. The ignored live suite
calls the spec, as the
[live judgment API test protocol](live-judgment-api-tests.md) says.

## References

- [OpenAI create decision reference](https://developers.openai.com/api/reference/resources/decisions/methods/create)
- [OpenAI prompt caching guide](https://developers.openai.com/api/docs/guides/prompt-caching)
- [OpenAI pricing](https://developers.openai.com/api/docs/pricing)
