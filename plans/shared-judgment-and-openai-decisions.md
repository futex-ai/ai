# Shared Judgment Contract And OpenAI Decisions

Status: Active. On branch `calummoore/adding-openai-decisions-api-9ler`, no pull request yet. The pull request that completes the work deletes this plan, as the `plans` skill says.

Hosts cannot call OpenAI's Decisions API (`POST /v1/decisions`, model
`gpt-6-luna`) through this library. The library already has one
provider-neutral judgment boundary, `JudgmentModel` in
`crates/ai-interface/src/judgment/`, with TypeSafe Jev as its only provider.
Its types follow TypeSafe: text or JSON state, a sorted map of choice
options, unlabelled score levels, and no per-question refusal. Decisions
asks the same three question kinds, but it also accepts images and
conversation history, takes ordered options and labelled levels, and can
refuse one question while it answers the others.

This change first widens the judgment boundary into one shared contract for
judgment and decisions. Then it adds an OpenAI Decisions adapter behind it.
When an adapter cannot send a part of a request, one shared support check
rejects the request with a typed error before transport. The change does not
add prompt-cache controls, the end-user `safety_identifier`, hosted image
URLs, boolean choice values, judgment retry or pricing layers, or automatic
splitting of more than 400 questions.

Saved copies of the OpenAI guide and reference, fetched on 2026-11-26, are in
`.context/openai-decisions/`. The options check of the decisions is in
`.context/shared-judgment-and-openai-decisions/options-check.md`.

## Decisions

The user answered decisions 1 to 4 on 2026-11-26. An options check ran over
them on the same day. It kept each choice and added the options and the
building blocks that the entries below name. Decision 5 came from that check.
The user answered it on 2026-11-26.

### 1. How hosts reach the Decisions API

- **Choice:** D. Widen the judgment boundary for images, conversations,
  ordered options, labelled levels, and refusals. Then add the OpenAI adapter
  behind it.
- **Alternatives:**
  - A. An OpenAI adapter behind the current boundary, with images and
    conversations left for later.
  - B. A separate client with OpenAI-shaped public types, outside the
    judgment boundary.
  - C. The client of B plus a judgment adapter on top of it.
- **Left out:**
  - The chat `Model` trait. It hides the distributions, and Decisions does
    not stream.
  - A separate `ai-models-openai-decisions` crate. The registry has one
    Cargo feature for each provider, as
    `docs/protocol/provider-registry.md` says. A second crate needs a
    second `openai` feature, or it gains nothing over a module.
  - Emulation through the Responses API. It does not use the requested API
    or its pricing.
  - No change. It fails the goal.
- **Reason:** hosts keep one swappable `DynJudgmentModel` and one mock, and
  still reach the Decisions features.

### 2. How a host learns that the model declined one question

- **Root question:** how does a host learn that the model declined one
  question, and still keep the other answers and the usage of the billed
  call?
- **Choice:** C. Add a typed `JudgmentAnswer::Refused` answer that the
  response check accepts for any question kind.
- **Alternatives:**
  - A. Fail the whole call with the `InvalidOutput` error kind.
  - B. Fail the whole call with the `ContentPolicy` error kind.
  - D. Add a list of refused ids beside the answer map.
  - E. Wrap each answer in an outcome that is `Answered` or `Refused`.
    Added by the options check.
  - F. Fail the call with a typed refusal error that carries the partial
    response. Added by the options check.
- **Reason:** C is the smallest type change that keeps the other answers
  and the usage. An exhaustive `match` must name the refusal. D also keeps
  the answers and the usage, but a caller that reads only the map sees a
  missing id, not a refusal.

### 3. Which inputs hosts can send

- **Root question:** what can a host ask a judgment model to evaluate, and
  how does the host learn that a model cannot take part of it?
- **Choice:** one shared request for judgment and decisions. It holds text,
  JSON, images, and conversations. When a provider does not support a part
  of the request, for example an image, the adapter rejects the request with
  a typed error before transport. This is option C with an explicit
  rejection rule. The reading of the user's answer is the agent's.
- **Alternatives:**
  - A. Text and JSON only.
  - B. Text, JSON, and images.
  - C. Text, JSON, images, and conversations.
- **Building blocks from the options check:** the design uses check
  options F, D, and G. F is one shared support check over the whole
  request. D gives descriptions a text-or-JSON type without images. G
  sets image support for each adapter, and leaves `ModelFeature::Vision`
  as routing data. The check also offered E, a message type that the
  judgment boundary owns, and H, a part for a hosted image URL. The design
  calls below leave them out.
- **Reason:** the user asked for one approach for all vendors, with
  unsupported input as an error.

### 4. Who controls the order of choice options

- **Root question:** how does a host make the model see the choice options
  in the order that the host chose?
- **Choice:** B. Choice options become an ordered list. Duplicate labels
  fail local validation.
- **Alternatives:**
  - A. Keep the sorted map and document the order.
  - C. An ordered options type whose constructor rejects a repeated label.
    Added by the options check. It breaks the boundary rule that
    constructors never validate.
- **Reason:** the model reads the options as prompt text. Hosts must be
  able to put a fallback option such as "other" last.

### 5. How TypeSafe sees the options in the host's order

- **Root question:** how does a host make TypeSafe see the options in the
  order that the host chose?
- **Background:** the options check of decision 4 found that json-http
  sorts the keys of every JSON object, at
  `crates/json-http/src/canonical/object.rs:18`, and
  `crates/json-http/src/_tests_/request_bytes_tests.rs:32` tests that rule.
  The TypeSafe `criteria` field is an object, so TypeSafe would still
  receive the options in alphabetical order.
- **Choice:** B, answered by the user on 2026-11-26. Add an opt-in
  ordered-object value to json-http that keeps insertion order. The
  TypeSafe `criteria` object uses it. Every other body keeps its sorted
  bytes, so the recordings do not change.
- **Alternatives:** A. Document that TypeSafe receives the options sorted
  by label.
- **Left out:** an array form for TypeSafe choice criteria. The TypeSafe
  protocol doc defines `criteria` as an object for a choice, and no source
  says that TypeSafe accepts an array.
- **Reason:** decision 4 then holds for every provider, and the opt-in
  value keeps all other wire bytes as they are.

## Design

The calls in this section are the agent's own, made on 2026-11-26 under the
decision test, unless a line names a user decision. The options check of
2026-11-26 changed several of them. The lines say which.

### Shared contract

- **Names:** keep the `Judgment` names of the boundary, such as
  `JudgmentModel`, `JudgmentRequest`, and `JudgmentAnswer`. The boundary is
  provider-neutral, and a rename would change every caller without a change
  in behaviour.
- **Content:** `JudgmentContent` becomes an ordered list of typed parts. A
  part is `Text`, `Json`, or `Image`. `Json` holds an object or an array.
  `Image` holds a MIME type and base64 data, as the chat
  `ConversationPart::Image` does. `From<&str>` and `From<String>` stay, so
  that text-only callers keep short code. The input and the instructions
  use this type.
- **Descriptions (changed by the options check):** option and level
  descriptions use a separate text-or-JSON type with no image part. No
  provider takes an image in a description, and Decisions takes only a
  string there.
- **Input (under decision 3):** the `state` field becomes
  `input: JudgmentInput`. `JudgmentInput` is `Content(JudgmentContent)` or
  `Conversation { system, messages }`. `messages` reuses
  `ConversationMessage`, and `system` is an optional developer text. The
  check's option E, a judgment-owned message type, stays out: hosts keep one
  conversation type, and a JSON part can go in a text part.
- **Images:** inline base64 only. The check's option H, a hosted image URL
  part, stays out. The chat parts have no URL form, and the part can come
  later without a breaking change.
- **Options (decision 4):** a choice holds `Vec<JudgmentOption>`, where an
  option has a `label` and an optional description. A new
  `DuplicateOptionLabel` problem rejects a repeated label. The answer keeps
  its probability map keyed by label.
- **Levels:** a score holds `Vec<JudgmentLevel>`, where a level has a
  required `label` and an optional description. Decisions requires a label
  for each level.
- **Refusal (decision 2):** `JudgmentAnswer::Refused` has no fields.
  Decisions sends no reason. `validate_against` accepts it for any question
  kind. `MockJudgmentModel` never refuses.
- **Answer kind (added after the options check):**
  `JudgmentAnswer::kind()` returns `Option<JudgmentQuestionKind>`, with
  `None` for a refusal. The response check skips the kind check for a
  refusal.
- **Support check (changed by the options check, under decision 3):** each
  adapter declares a `JudgmentSupport` value: the content part kinds, the
  input forms, and the conversation part kinds that its provider takes. One
  shared function walks the whole request, including the input and the
  instructions. It returns the new `JudgmentError::UnsupportedInput` with
  the provider, the model id, and the unsupported feature. Its error kind is
  `UnsupportedInput`. The chat side made the same choice for its controls in
  `docs/protocol/model-capabilities.md`.
- **Image support (changed by the options check):** image support is part
  of the `JudgmentSupport` of each adapter, not a spec feature gate.
  TypeSafe takes no images. The OpenAI adapter takes images for every spec.
  `ModelFeature::Vision` on a spec stays routing data.
- **Choice values:** labels stay strings. Decisions accepts boolean values
  too, but the condition question already covers yes and no.

### TypeSafe mapping

- **Support:** TypeSafe takes text and JSON content only. Image parts and
  conversation input fail the support check.
- **Content:** content with one text part, or one JSON part, sends the same
  JSON as today. Content with several parts sends a JSON array of the parts.
  The TypeSafe protocol doc gives the exact form.
- **Options (decision 5):** the `criteria` object holds the options in the
  caller's order, through the ordered-object value of json-http.
- **Levels (changed by the options check):** a level without a description
  sends its label as text. A level with a description sends the JSON object
  `{"label": ..., "description": ...}`. TypeSafe and OpenAI then read the
  same rubric text.
- **Question ids:** TypeSafe sends the caller's ids as the keys of its
  `questions` object. The judgment boundary doc wrongly says that ids never
  reach the model. Milestone 1 corrects the sentence.
- **Refusal:** TypeSafe never returns a refusal. The adapter keeps its
  current response mapping.

### OpenAI Decisions mapping

- **Adapter:** `OpenAiJudgmentModel` in a new `decisions/` module of
  `ai-models-openai`. It posts to the configured base URL plus
  `/v1/decisions` with Bearer auth.
- **Spec:** `GPT_6_LUNA`, text id `openai/gpt-6-luna`, with the `Judgment`
  and `Vision` features, no effort, intelligence score 5, the very fast
  speed tier, and the low cost tier. Score 5 matches the TypeSafe Jev specs,
  because routing compares judgment specs with each other. The docs agent
  of Milestone 1 found that the spec needs a score. Prices stay in host config, as
  `docs/protocol/model-capabilities.md` requires. A catalog lists each
  model id once. If the library later adds chat support for `gpt-6-luna`,
  this one spec gains the chat features, and the chat suites stop skipping
  every spec with `Judgment`. That change is reversible, so it is not a
  user decision.
- **Input:** text-only content becomes the `input` string. Other content
  becomes one user message with `input_text`, `input_json`, and
  `input_image` parts. A conversation becomes developer, user, and
  assistant messages, `function_call` items, and `function_call_output`
  items. Reasoning parts and provider state are not sent. Video parts fail
  the support check.
- **JSON:** an object, or an array of objects, becomes an `input_json`
  part. Other arrays become compact JSON text, because `input_json` accepts
  only objects. A JSON description becomes compact JSON text, because
  Decisions takes only a string description.
- **Questions:** each question gets an opaque name such as `q0`, so that
  caller ids never reach the model. Answers map back by their echoed name.
  Two answers with the same name are invalid output. A condition without
  criteria becomes a `predicate`. A condition with criteria becomes a
  `choice` between the boolean values `true` and `false`, with the two
  descriptions. The probability of `true` becomes the condition
  probability. Milestone 5 checks this mapping with a live comparison.
- **Answers:** `predicate`, `choice`, and `score` map to the shared answers.
  `refusal` maps to `JudgmentAnswer::Refused`. Score probabilities use the
  level `value` as the index. A duplicate option or level in a
  distribution is invalid output.
- **Usage (changed by the options check):** OpenAI counts cached and
  cache-write tokens inside `input_tokens`. The adapter subtracts
  `cached_tokens` from the input count and reports them as cached input. It
  subtracts `reasoning_tokens` from the output count. This is what the
  Responses adapter does in
  `crates/ai-models-openai/src/openai/response/mod.rs`. Cache-write tokens
  stay in the plain input count, as in the Anthropic adapter.
  `plans/follow-ups/cache-write-token-pricing.md` records the price gap.
- **Limits:** no local checks for the OpenAI limits, such as 400 questions
  or 10 levels. OpenAI rejects them, and the rejection maps to the
  `InvalidRequest` error kind.
- **Prompt caching:** the adapter sends no cache controls. OpenAI caches
  the shared input by default.
- **Timeout and retries:** a 60-second default total timeout that the
  provider config can replace. One request per call, with no retry inside
  the adapter. A transport timeout keeps the `Transient` error kind. The
  protocol doc warns that a host retry can bill twice, and links
  `plans/follow-ups/xai-deferred-timeout-rebills.md`,
  `plans/follow-ups/deepseek-resource-error-rebills.md`, and
  `plans/follow-ups/google-video-poll-failure-rebills.md`, which cover the
  same risk.
- **Errors:** parse the OpenAI `error` envelope, map its codes through the
  `native_kind` code table of the OpenAI chat adapter, and fall back to the
  HTTP status. Keep `x-request-id`. The API key, the auth headers, and the
  base64 data of each request image never reach error text. Other request
  text can appear in a provider message, as in the other OpenAI adapters.
  Redacting all request text would remove too much of the provider message.
  This narrows the first wording after Milestone 1 found the gap.
- **Construction:** `from_config` only, for the registry, as
  `plans/follow-ups/remove-provider-constructors.md` proposes.

Replaced ways:

- `JudgmentContent` as an untagged text, object, or array → an ordered list
  of typed parts, plus a text-or-JSON description type. Consumers:
  `crates/ai-interface/src/judgment/` and its tests,
  `crates/ai-interface/src/_tests_/mock_judgment_model_tests.rs`, the
  TypeSafe request and its tests, `xtask/tests/live_judgments/`, and the
  docs below. Moved and deleted in Milestone 3.
- `JudgmentRequest::state` → `JudgmentRequest::input`. Consumers: the
  judgment request and response tests, the mock tests, the TypeSafe request
  and its tests, `crates/ai-models-registry/src/_tests_/judgment_tests.rs`,
  and `xtask/tests/live_judgments/`. Moved and deleted in Milestone 3.
- Choice options as `BTreeMap<String, Option<JudgmentContent>>` → an ordered
  `Vec<JudgmentOption>`. Consumers: the judgment question, request, and
  response code and their tests, the mock, the TypeSafe request and its
  tests, and `xtask/tests/live_judgments/runner_tests.rs`. Moved and deleted
  in Milestone 2.
- Score levels as `Vec<JudgmentContent>` → `Vec<JudgmentLevel>`. Consumers:
  the same files as the options. Moved and deleted in Milestone 2.

No converter stays. The project is not in production, so callers outside
this repository move to the new types.

Contract owners:

- [Judgment boundary protocol](../docs/protocol/judgment-boundary.md).
- [TypeSafe judgment provider protocol](../docs/protocol/typesafe-judgment-provider.md).
- [Judgment responses protocol](../docs/protocol/judgment-responses.md).
- [TypeSafe judgment responses protocol](../docs/protocol/typesafe-judgment-responses.md).
- [OpenAI judgment provider protocol](../docs/protocol/openai-judgment-provider.md).
- [OpenAI judgment responses protocol](../docs/protocol/openai-judgment-responses.md).
- [Live judgment API test protocol](../docs/protocol/live-judgment-api-tests.md).
- [Provider registry protocol](../docs/protocol/provider-registry.md).
- [Model capabilities protocol](../docs/protocol/model-capabilities.md).
- The [ai-interface README](../crates/ai-interface/README.md), the
  [ai-models-openai README](../crates/ai-models-openai/README.md), the
  [ai-models-typesafe README](../crates/ai-models-typesafe/README.md), the
  [json-http README](../crates/json-http/README.md), and the
  [workspace README](../README.md).

## Milestone 1: Documentation and protocol

Depends on: none

The protocol docs state the shared contract, the TypeSafe mapping, and the
OpenAI mapping before code changes.

- [x] Update `docs/protocol/judgment-boundary.md` with typed content parts,
      the description type, `JudgmentInput`, ordered options, labelled
      levels, the refusal answer and its kind, the `DuplicateOptionLabel`
      problem, the shared support check, and the `UnsupportedInput` error.
      Correct the sentence that says question ids never reach the model.
- [x] Update `docs/protocol/typesafe-judgment-provider.md` with its
      support, the content mapping, the level mapping, and the criteria
      order in the caller's order.
- [x] Update the json-http README with the opt-in ordered-object value and
      its exception to the sorted key order.
- [x] Write `docs/protocol/openai-judgment-provider.md`: purpose, scope,
      endpoint, spec, support, request mapping, response mapping, usage,
      errors, timeout, and billing notes with the links to the three
      rebill follow-up items.
- [x] Update `docs/protocol/provider-registry.md`,
      `docs/protocol/model-capabilities.md`, and
      `docs/protocol/live-judgment-api-tests.md` for the OpenAI judgment
      spec and adapter.
- [x] Link the new protocol doc from the workspace README and the
      `ai-models-openai` README.

## Milestone 2: Ordered options, labelled levels, and refusals

Depends on: 1

The shared answer and question types gain the refusal, the ordered options,
and the labelled levels. json-http gains the ordered-object value for the
TypeSafe criteria. TypeSafe, the mock, and every test move with them.

- [x] Add `JudgmentOption`, `JudgmentLevel`, and the `DuplicateOptionLabel`
      problem in `crates/ai-interface/src/judgment/`, with tests first.
- [x] Add `JudgmentAnswer::Refused`, change `kind()` to return an option,
      and accept a refusal in `validate_against` for any question kind.
- [x] Move `MockJudgmentModel` to the new option and level types.
- [x] Move the TypeSafe request mapping to the new options and the level
      mapping.
- [x] Add the opt-in ordered-object value to json-http, with a test of its
      insertion-order bytes and a test that other bodies keep sorted keys.
- [x] Send the TypeSafe `criteria` object through the ordered-object value,
      with a request test that proves the caller's order on the wire.
- [x] Move every consumer that the replaced-ways list names for options and
      levels. Delete the old map and list types.
- [x] Update the `ai-interface` and `ai-models-typesafe` READMEs and
      `crates/ai-interface/docs/examples.md`. Link the new judgment and
      TypeSafe response pages from the two READMEs.

## Milestone 3: Typed content, conversation input, and the support check

Depends on: 2

Requests take typed content parts or a conversation. The shared support
check rejects what an adapter cannot send.

- [x] Replace `JudgmentContent` with typed text, JSON, and image parts, and
      add the text-or-JSON description type. Keep the `From<&str>` and
      `From<String>` conversions.
- [x] Add `JudgmentInput` and rename `state` to `input`. Move the empty-input
      validation to the new type.
- [x] Add `JudgmentSupport`, the shared support check, and
      `JudgmentError::UnsupportedInput`, with tests for each place that can
      hold an unsupported part.
- [x] Give the TypeSafe adapter its support value, with tests that prove an
      image or a conversation sends no request.
- [x] Move every consumer that the replaced-ways list names for content and
      state. Delete the old content type.
- [x] Update the READMEs and `crates/ai-interface/docs/examples.md`.

## Milestone 4: OpenAI Decisions adapter

Depends on: 3

`ai-models-openai` calls `/v1/decisions` behind `JudgmentModel`. Every file
stays at or below 300 lines.

- [x] Add the `GPT_6_LUNA` spec to the OpenAI catalog.
- [x] Add the request mapping for content, conversations, descriptions, and
      the three question kinds, with request tests.
- [x] Add the response mapping for the four answer kinds and the usage, with
      response tests. Cover a refusal, a repeated answer name, a duplicate
      distribution entry, and the subtraction of cached and reasoning
      tokens.
- [x] Add the error mapping, the timeout, and the redaction of the API key
      and request content, with tests.
- [x] Give the adapter its support value, with a test that proves a video
      part sends no request.
- [x] Update the `ai-models-openai` README.

## Milestone 5: Registry, smoke test, and live tests

Depends on: 4

Hosts build the OpenAI judgment adapter through the registry. The smoke test
and the ignored live suite cover it.

- [x] Add the `openai` arm to the registry judgment factory, with a
      registry test.
- [x] Add the OpenAI judgment adapter to `xtask/src/smoke/providers.rs`.
- [x] Register `openai` in the live judgment suite under
      `xtask/tests/live_judgments/`. Narrow the catalog guards to the
      judgment specs, and remove `openai` from the non-judgment provider
      list. Keep the live tests ignored and out of `cargo xtask check`.
- [x] Add the `OPENAI_API_KEY` secret to `.github/workflows/live-judgments.yml`.
- [x] Update `docs/protocol/model-policies.md` with the judgment
      `UnsupportedInput` error, `crates/ai-models-registry/README.md` with
      the OpenAI judgment adapter, and `docs/development.md` and
      `xtask/README.md` with the OpenAI live judgment secret and command.
- [ ] Run the live judgment test against OpenAI once with credentials.
      Check that `choice` is the most probable option, that each
      distribution lists every option, and that the error envelope matches
      the protocol doc. Save the output under
      `.context/shared-judgment-and-openai-decisions/`. Raise a decision
      when a check fails.
- [ ] Compare one condition with criteria, sent as a boolean `choice`,
      against the same condition sent as a `predicate`, in one paid call.
      Save the output in the same folder. Raise a decision when the two
      probabilities disagree by more than the response check tolerance.

## Milestone 6: Verification, commit, review, and pull request

Depends on: every other milestone

- [x] Run the targeted tests and the complete gate, `cargo xtask check`.
- [x] Commit with Conventional Commits and push the branch.
- [x] Review the complete local diff against `origin/main` with the
      `implementation-review` skill after the push. Report the findings,
      then apply the fix rule: fix the `fix now` findings, record
      follow-up items under `plans/follow-ups/` for the rest, re-review
      once, and report.
- [ ] Delete this plan when the `plans` skill says, under "Deleting the
      plan".
- [ ] Open the pull request. Its description gives the decisions and the
      design calls of this plan with their dates, a permalink to this plan,
      and the follow-up items that the change adds.
