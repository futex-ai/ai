# AI library redesign

Status: Active. [PR #24](https://github.com/futex-ai/ai/pull/24) is open for
the first pull request, [PR #25](https://github.com/futex-ai/ai/pull/25)
for the second, and [PR #26](https://github.com/futex-ai/ai/pull/26) for the
third. The work spans eight stacked pull requests. Each earlier pull request keeps
this plan. The last one deletes it, as the final milestone says.

The workspace puts nine AI providers behind one interface, with a
tool-calling loop and an MCP client. A review on 2026-11-01 found these
problems. The facts of one model live in its catalog, its request code, and
protocol prose. Five providers copy one Chat Completions implementation. The
shared crate names every provider. Streaming, replay, fallback, and
truncation behave in ways that callers do not expect. This plan replaces
those parts with one design. It keeps every provider wire mapping, the error
classes, the tool output windows, and the OAuth security rules.

User decision, 2026-11-01: the user chose the redesign from the review
conversation with the message "ok, create a plan around the new suggested
design". The alternative was to fix the review findings one by one in the
current design. The findings share causes that only a structural change
removes. The user answered every open decision of this plan on the same
day, as the "Decisions" section records. The user approved this plan on
2026-11-01 with the message "plan approved - impl all plan milestones and
tasks, tick off as you go". The approval covers the list under "Removed or
replaced functionality".

Not covered: prompt caching, native structured-output modes, new providers,
changes to the live-test triggers, and changes inside Firna or Juno. Firna
and Juno adopt the new API after the merge.

Contract owners:

- New in Milestone 1: `docs/architecture.md`,
  `docs/protocol/model-stream.md`, `docs/protocol/conversation-parts.md`,
  `docs/protocol/model-capabilities.md`,
  `docs/protocol/chat-completions-dialects.md`,
  `docs/protocol/provider-registry.md`, `docs/protocol/model-policies.md`,
  and `docs/protocol/agent-loop.md`.
- [Provider call controls](../docs/protocol/provider-call-controls.md) and
  [tool output management](../docs/protocol/tool-output-management.md).
- The [workspace README](../README.md) and each crate README that a
  milestone changes.

## Design

| Component | Crate | Responsibility |
| --- | --- | --- |
| Contracts | `ai-interface` | Model, tool, and modality traits, stream events, the fold from events to a response, conversation parts, provider state, model specs, efforts, and errors. It names no provider. |
| HTTP | `json-http` | Requests, SSE, response headers, typed bodies, and record-and-replay for tests. |
| Adapter helpers | `ai-models-core` | The capability check, error classification, schema checks, and synthetic tool-call ids. |
| Native adapters | `ai-models-anthropic`, `ai-models-openai`, `ai-models-google`, `ai-models-typesafe` | One adapter for each native protocol. |
| Compatible engine | `ai-models-compat` | One Chat Completions engine, with a dialect for DeepSeek, Kimi, Qwen, MiniMax, and xAI. |
| Policies | `ai-models-policy` | Retry, fallback, concurrency, and pricing layers. |
| Registry | `ai-models-registry` | Provider config, and one factory for every catalog entry. |
| Agent loop | `ai-tool-calling` | Turns, parallel tools, outcomes, observers, and tool output windows. |
| MCP | `ai-mcp`, `ai-mcp-oauth` | The MCP client and its OAuth flow, until Milestone 11 decides on `rmcp`. |
| Test kit | `ai-testkit` | Scenarios, recordings, and the conformance runner. |

- **Stream.** `Model::stream(&request)` returns a pull-based event stream.
  The events are text delta, reasoning delta, tool-call start, tool-call
  argument delta, tool-call end, usage, and finish. `Model::complete` is a
  provided method that folds the events into a `ModelResponse`. Adapters
  emit events from the provider stream and build no intermediate JSON.
- **Conversation parts.** A message is a role and ordered parts: text,
  image, video, tool call, tool result, and reasoning. A part can carry
  `ProviderState { provider, model, version, payload }`. Each adapter
  decodes its own payload into a private typed struct. It skips the state
  of other providers.
- **Model specs.** Each provider crate exports `ModelSpec` constants and a
  `CATALOG` slice. A spec holds the provider id, the provider model id, the
  context window, the max output, the efforts, the default effort, the
  forced-tool-choice rule, the replay need, the features, and the routing
  tiers. Text ids have the form `provider/model`. `catalog().find` turns
  them into specs at load time. `ModelSpec::custom` covers unlisted models.
- **Effort.** The request carries an `Effort`. The capability check lowers
  it to the highest effort that the spec supports. It never raises it.
- **Capability check.** One shared function checks each request against
  its spec before transport. It covers the output limit, the effort, tool
  choice with thinking on, and the other controls.
- **Dialects.** A dialect sets the endpoint, auth, model ids, reasoning
  field, replay rule, thinking parameter, usage fields, tool-choice limits,
  stream quirks, and an optional deferred lifecycle.
- **Registry.** `ProviderConfig` holds the provider, base URL, auth, extra
  headers, and timeouts. `Registry::model(&spec)` builds a model. Sibling
  methods build image, video, transcription, and judgment adapters. Every
  adapter honors the base URL.
- **Policies.** Every modality error shares one core: kind, provider,
  model, message, retry-after, and provider request id. Each error kind
  states whether another model can succeed. Retry backs off with jitter,
  honors retry-after, and covers rate limits. A fallback needs an explicit
  policy. Its presets are "any error" and "only when another model can
  succeed". Fallback reports every attempt.
- **Agent loop.** The caller owns the conversation and passes it to `run`.
  The loop streams agent events. It runs tool calls in parallel up to a
  limit and appends the results in call order. Observer errors go to
  `tracing` and never fail a turn. The outcomes are completed, truncated,
  and capped. A final answer can carry a schema. The loop offers
  `tool_output_read` only after a windowed output exists. `TypedTool`
  derives the JSON schema of a tool from its argument type.
- **Tests.** `ai-testkit` replays recorded provider streams through each
  adapter and snapshots the results. The live tests keep their single-turn
  probes and their current triggers.

The review conversation used example crate names. They map as follows:
`ai-core` is `ai-interface`, `ai-http` is `json-http`, `ai-provider-*` is
`ai-models-*`, `ai-policy` is `ai-models-policy`, `ai-registry` is
`ai-models-registry`, and `ai-agent` is `ai-tool-calling`.

## Removed or replaced functionality

The plan approval covers these removals and behavior changes. Each
milestone keeps all behavior that this list does not name.

- `Model::complete_with_events`, `ModelCompletionEvent`, and
  `ModelCompletionEventSink`: replaced by `Model::stream`.
- `ProviderConversationItem`, its per-provider structs, and the `content`
  and `content_parts` pair of `ConversationMessage`: replaced by parts and
  provider state.
- `ProviderKind`: replaced by `ProviderId` constants.
- `KnownModelSpec`, `KnownModelCatalog`, `ThinkingLevel` as a construction
  input, `resolve_catalog_thinking_level`, and the 24 thinking-variant
  catalog ids, such as `claude-opus-5-thinking-max`: replaced by
  `ModelSpec` and `Effort`.
- The crates `ai-models-deepseek`, `ai-models-kimi`, `ai-models-qwen`,
  `ai-models-minimax`, and `ai-models-xai`: merged into `ai-models-compat`
  with the same wire behavior.
- The crate `ai-models-multi` and the wrappers of `ai-models-core`: moved
  into `ai-models-policy`.
- The per-provider `new`, `with_auth`, and `with_catalog_auth`
  constructors: replaced by the registry.
- The shared mutable conversation API of `ToolCallingRuntime`, such as
  `replace_conversation` and `push_user_message`: replaced by the
  caller-owned conversation.
- `Turn`, `ActiveTurn::step`, `StepOutcome`, and
  `Turn::successful_tool_calls`: replaced by the agent events and outcomes.
- The 4,096-token ceiling of Anthropic requests: replaced by the max output
  of each catalog entry, as item 2 says.
- Forced tool choice for Anthropic with thinking on: replaced by
  `UnsupportedControl` and an automatic fallback, as item 3 says.
- The `Completed` outcome for truncated answers: replaced by a `Truncated`
  outcome, as item 4 says.
- The fall-through-on-any-error default of fallback: replaced by a required
  policy, as decision 5 says.

The MCP crates change only after the MCP decision of Milestone 11.

## Decisions

The user gave every answer on 2026-11-01. The user gave no reason for the
answers that differ from the recommendation.

- Item 1, Claude and Gemini drop replay data, applied in Milestone 3:
  option A. Add Anthropic thinking-block and Gemini thought-signature
  variants to `ProviderConversationItem`, replay them, and add mock
  regression tests. The alternatives were a live two-turn probe in
  addition, or opaque provider state at once. The recommendation was the
  live probe in addition.
- Item 2, Anthropic output is capped at 4,096 tokens, applied in Milestone 3:
  option B. Add a max output to each catalog entry, and use it as the
  default and the ceiling. The alternatives were a larger constant, or no
  ceiling. The recommendation was the larger constant.
- Item 3, Anthropic forces tool use while thinking is on, applied in Milestone 3:
  option A. With thinking on, return `UnsupportedControl` for `Required`
  and `Function`, and send `auto` for `RequiredOrAuto`. The alternatives
  were a live probe in addition, or the rule in catalog data at once. The
  recommendation was the live probe in addition.
- Item 4, Truncated answers end the turn as completed, applied in Milestone 3:
  option B. Add a `Truncated` variant to `StepOutcome` and `RunOutcome`.
  The alternatives were a finish reason on `Completed`, or an automatic
  continuation. B was the recommendation, because every caller must then
  decide.
- Decision 5, which failures move a call to the next model: option D. A
  fallback needs an explicit policy, with the presets "any error" and
  "only when another model can succeed". The provider error splits into an
  invalid request, a rejected key, an unknown model, an input that the
  model does not take, and invalid model output. Each kind states whether
  another model can succeed. The alternatives were "any error" as the
  default, "only when unavailable" as the default, a required policy with
  presets on today's error classes, or no fallback in the library. D was
  the recommendation. Benchmarks and products need different rules, and
  today's classes stop a chain on failures that the next model can take.
- Decision 6, live multi-step checks: dropped. The live tests keep their
  single-turn probes. The alternatives were probes for one thinking model
  per provider, probes for every catalog model, recordings only, the
  conformance scenarios run live, or live models chosen from the specs.
- Item 10, [xAI deferred timeout can bill twice](follow-ups/xai-deferred-timeout-rebills.md):
  open. It blocks the xAI deferred timeout rule of Milestones 4 and 8.
- Item 16, [Choose how the MCP crates change](follow-ups/mcp-crate-direction.md):
  open. Milestone 11 raised it on 2026-11-02. It blocks Milestone 12. The
  recommendation is option B, which moves the `ai-mcp` transport onto
  `json-http`.
- Item 17, [Empty SSE data breaks rmcp-based servers](follow-ups/mcp-empty-sse-data.md):
  open. Milestone 11 found this defect on `main` on 2026-11-02. Milestone 12
  applies the answer. The recommendation is option E, which decodes only
  message events with data.
- Item 18, [Choose the MCP revisions the client supports](follow-ups/mcp-client-revision-target.md):
  open. It decides between options B and G of item 16. The recommendation
  is option B, a follow-up plan for dual-era support.
- Item 19, [MCP OAuth misses the newer authorization rules](follow-ups/mcp-oauth-newer-rules.md):
  open. The recommendation is option C, a follow-up plan for the newer
  OAuth rules.
- Decision 7, how much paid live testing each change pays for: option A.
  The live suites keep their current triggers. The alternatives were path
  filters, a pull-request label, schedules only, a small pull-request set,
  one run for each merged change, or a run set from the Cargo dependency
  graph. The recommendation was path filters.

## Design calls

All calls are mine, made on 2026-11-01.

- One plan for eight pull requests. Each diff stays reviewable.
- Evolve the crates in place. Firna pins revisions and can migrate once,
  and the recordings give the regression safety.
- Build the Chat Completions engine before the interface changes, so the
  stream and parts milestones touch four adapters instead of eight.
- Keep the current crate names and the trait name `Model`. Renames add
  churn without a behavior gain.
- The event stream is a boxed trait object with `async fn next(&mut self)`,
  as `JsonHttpSseStream` is. It works with `dyn` dispatch and `unimock`.
- A provider state payload is a JSON object with a format version. Hosts
  store conversations as JSON, and each adapter types the payload at once.
- Prices stay out of `ModelSpec`. Deployment prices change without a
  release.
- The request keeps `system_prompt`. This keeps today's blank-system rule.
- A one-way converter reads the old stored conversation format, so stored
  conversations survive.
- Milestone 1 writes the target docs with the status "Approved, not
  implemented". Each milestone switches its doc when its code lands.
- `cargo xtask capability-matrix` generates the control matrix. No CI step
  compares it, because that step would be a new gate.
- Keep `ModelRouter`, `activity_verb`, and `group_for_tool`. Juno and Firna
  use them.
- The registry also builds image, video, transcription, and judgment
  adapters, so one factory covers every catalog entry.
- The MCP crates change only after Milestone 11 compares `rmcp` with the
  MCP protocol docs. A replacement deletes functionality on `main`.
- The eight pull requests form a stack. Each one targets the branch of the
  pull request before it, so that each shows only its own milestones.
- Pull request #23 aligned `AGENTS.md` with the shared skills on
  2026-11-01. Reviews therefore use the `implementation-review` skill and
  its fix rule, and the last pull request deletes this plan.
- Items 1 to 4 land in Milestone 3 on today's types. Their defects reach
  callers now, so they do not wait for the redesign milestones. The later
  milestones carry each fix into the new types.
- Milestone 5 moves the Anthropic rule of item 3 into spec data with the
  other control rules, because the chosen capability check holds every
  control rule. The behavior stays as option A of item 3.
- Milestone 7 moves the replay variants of item 1 into provider state.
- Milestone 1, 2026-11-02: the docs implementor made these calls where the
  plan was silent, and I accepted them:
  - The stream has an eighth event, the fallback event. It keeps the
    restart signal that `MultiModel` gives today and carries each failed
    attempt.
  - Content events carry a part position. Deltas and tool-call ends can
    carry provider state, so that the fold rebuilds ordered parts.
  - Output checks run inside the stream before the usage event, so that a
    fallback can act on invalid output.
  - The stored conversation format has `format_version` 2, and a converter
    reads version 1.
  - Own provider state with an unknown version or a payload that does not
    decode fails before transport with the invalid request kind.
  - Log copies drop all provider state and all reasoning text.
  - Model specs also hold the sampling rule, stop support, and the deferred
    lifecycle. The forced tool-choice rule has four values.
  - The error kinds add content policy and timed out for the image and
    video modalities. HTTP 401, 402, and 403 map to rejected key, and 404
    maps to unknown model.
  - Retry defaults to three attempts with base delays of 100 ms and 250 ms,
    waits from half to all of the base delay, and a maximum wait of 30 s.
  - The registry holds one config per provider. Adapter and auth headers
    win over extra headers.
  - The agent loop runs four tool calls at a time by default. A filtered
    step adds no message. The turn and response checkpoints stay.
  - The loop offers the output reader only when the output store holds a
    stored output. It still runs a reader call that it did not offer.
- Review round 1 of pull request 1, 2026-11-02:
  - Finding 11: completed tool results stay in the conversation, and the
    host closes unfinished calls. Options B and D, which close every call
    inside the library, wait for the user in
    `plans/follow-ups/stopped-step-calls.md`.
  - Finding 12: each provider crate converts its own version 1 items, and
    the registry composes the conversions.
  - Finding 13: the fold lives in the contracts, and the agent loop uses
    it.
  - The converter keeps the items of a provider that the registry does not
    include, and the first item's provider sets the order of a mixed
    message. Every tool event names its call id.
- The plan has about 520 lines, above the 300-line target, because it
  covers eight pull requests. A split would scatter the shared decisions.
- 2026-11-02, before Milestone 4:
  - Milestone 11 depends on Milestone 1 only. It reads and writes a
    comparison under `.context/`, so it can run beside the code
    milestones, and the MCP decision reaches the user sooner. Milestone 12
    depends on Milestones 8 and 11, because one of its options builds on
    the `json-http` work of Milestone 8.
  - Milestone 4 moves the Chat Completions types of `ai-models-core` into
    `ai-models-compat`. Only the five ported crates use them, and
    `docs/architecture.md` gives `ai-models-core` no Chat Completions
    role.
  - Milestone 9 marks `chat-completions-dialects.md` implemented. The doc
    describes the engine on the specs, events, parts, error kinds, and
    base URLs of Milestones 5 to 9.
- 2026-11-02, before Milestone 6:
  - Until Milestone 7, the finish event carries each adapter's message-level
    `provider_context` items as an interim field, so that the fold rebuilds
    today's response. Milestone 7 replaces it with per-part provider state.
  - Milestone 7 marks `model-stream.md` implemented, because the provider
    state rules of that doc need the parts of Milestone 7.

## Pull requests

| Pull request | Milestones |
| --- | --- |
| 1. Requirements | 1 |
| 2. Safety net and current fixes | 2, 3 |
| 3. Engine and specs | 4, 5 |
| 4. Stream and parts | 6, 7 |
| 5. HTTP, policies, and registry | 8, 9 |
| 6. Agent loop | 10 |
| 7. MCP | 11, 12 |
| 8. Conformance and close | 13, 14 |

Each milestone ends with `cargo xtask check`, a commit, and a push. Commit
and pull request titles use Conventional Commits. The last milestone of a
pull request then runs the ship steps:

1. Review the complete diff of the pull request with the
   `implementation-review` skill after the push.
2. Apply the fix rule: fix the `fix now` findings, record follow-up items
   under `plans/follow-ups/` for the rest, re-review once, and report.
3. Open the pull request into the branch of the pull request before it,
   or into `main` for the first one, and keep this plan.

## Milestone 1: Documentation and protocol

Depends on: none

Write the requirements and decisions of the target. New docs carry the
status "Approved, not implemented".

- [x] Add `docs/architecture.md` with the target components, their
      responsibilities, and their boundaries.
- [x] Add `docs/protocol/model-stream.md`: the stream method, the events
      and their order, the fold, deadlines, interruption, and wrappers.
- [x] Add `docs/protocol/conversation-parts.md`: roles, parts, provider
      state, replay and skip rules, stored-format versions, the converter,
      and log redaction.
- [x] Add `docs/protocol/model-capabilities.md`: specs, text ids, custom
      specs, effort, the capability check, and the control matrix. Include
      the max output of item 2 and the Anthropic rule of item 3.
- [x] Add `docs/protocol/chat-completions-dialects.md`. Link it from the
      DeepSeek, Kimi, Qwen, and MiniMax protocol docs.
- [x] Add `docs/protocol/provider-registry.md`: provider config, base URL,
      auth, headers, timeouts, and the adapters that a spec builds.
- [x] Add `docs/protocol/model-policies.md`: the error core, the scope of
      each error kind, retry, fallback, concurrency, and pricing. State the
      fallback rule of decision 5.
- [x] Add `docs/protocol/agent-loop.md`: turns, steps, the caller-owned
      conversation, parallel tools, outcomes, observers, the final-answer
      schema, and the output reader. Include the `Truncated` outcome of
      item 4. Align `tool-output-management.md`.
- [x] Update the protocol list of the workspace `README.md`.
- [x] Check every relative link in the changed Markdown.
- [x] Commit and push. Run the ship steps for pull request 1.

## Milestone 2: Recordings and conformance runner

Depends on: 1

Capture what each adapter does today, so that later milestones can prove
that they keep it.

- [x] Add a recording transport and a replay transport to `json-http`
      behind its `test-support` feature. A recording holds the request and
      the raw response bytes.
- [x] Add the `ai-testkit` crate with scenarios, recording storage, and a
      conformance runner. The runner snapshots each result with
      file-based `insta` snapshots.
- [x] Define the scenarios: one text turn, a two-turn tool loop with
      thinking on, a schema call, an image input, a rate-limit error, and a
      stream cut after progress.
- [x] Add a manual workflow that records the success scenarios for each
      provider. The user runs it, because it needs the provider secrets.
- [x] Build the rate-limit and stream-cut scenarios from documented
      provider error bodies, because a live provider does not produce them
      on demand.
- [x] Commit the recordings, and snapshot today's results, errors
      included.
- [x] Run `cargo xtask check`, then commit and push.

## Milestone 3: Fixes in the current design

Depends on: 2

Apply the answers to items 1 to 4 on today's types.

- [x] Item 1, option A: add Anthropic thinking-block and Gemini
      thought-signature variants to `ProviderConversationItem`. Replay them
      in their adapters. Handle the new variants in every other adapter, in
      the synthetic tool-call id hasher, and in the log redaction list. Add
      mock regression tests.
- [x] Item 2, option B: add a max output to each catalog entry. Take each
      value from the provider's model documentation. Use it as the default
      and the ceiling in the Anthropic adapter. Update
      `provider-call-controls.md`.
- [x] Item 3, option A: with thinking on, return `UnsupportedControl` for
      `Required` and `Function`, and send `auto` for `RequiredOrAuto`.
      Update the matrix in `provider-call-controls.md`.
- [x] Item 4, option B: add a `Truncated` variant to `StepOutcome` and
      `RunOutcome`. Mark that requirement implemented in `agent-loop.md`.
- [x] Delete each follow-up item in the commit that applies it.
- [x] Re-record the affected scenarios, and update their snapshots.
- [x] Run `cargo xtask check`, then commit and push.
- [x] Run the ship steps for pull request 2.

## Milestone 4: Chat Completions engine

Depends on: 3

Replace five copies of one protocol with one engine and five dialects.

- [x] Add `ai-models-compat` with the engine: request mapping, streaming,
      accumulation, response mapping, usage, and errors.
- [x] Port DeepSeek, Kimi, and Qwen as dialects. Move their tests into
      dialect test modules, and keep every assertion.
- [x] Port MiniMax with its stream normalizer and reasoning details.
- [x] Port xAI with deferred completion and legacy function calls.
- [x] Prove parity with the recordings and the moved tests.
- [x] Delete the five crates. Update the workspace members, the READMEs,
      the docs, `xtask`, and CI.
- [x] Run `cargo xtask check`, then commit and push.

## Milestone 5: Model specs and capabilities

Depends on: 4

Put the facts of each model in one constant, and check requests against it.

- [x] Add `ModelSpec`, `ProviderId`, `Effort`, and the capability types to
      `ai-interface`.
- [x] Convert every catalog to `ModelSpec` constants and a `CATALOG`
      slice. Remove the thinking-variant entries.
- [x] Add the `provider/model` lookup and `ModelSpec::custom`.
- [x] Add the request effort, lowered to the spec.
- [x] Add the capability check for the output limit, tool choice with
      thinking on, and the other controls. Carry the max output of item 2
      into each spec. Move the Anthropic rule of item 3 into spec data.
- [x] Add `cargo xtask capability-matrix`, and regenerate the matrix in
      `model-capabilities.md`. Mark that doc implemented.
- [x] Run `cargo xtask check`, then commit and push.
- [x] Run the ship steps for pull request 3.

## Milestone 6: One stream method

Depends on: 5

Make streaming the single model entry point.

- [x] Replace `complete` and `complete_with_events` on `Model` with
      `stream` and a provided `complete` fold.
- [x] Add the event types and the fold to `ai-interface`, beside `Model`,
      so that `complete` and the agent loop share one fold.
- [x] Emit events directly in the Anthropic, OpenAI, and Google adapters
      and in the engine. Remove the JSON round trip.
- [x] Move the retry, concurrency, pricing, and fallback wrappers to
      stream semantics.
- [x] Re-run the recordings, and migrate the snapshots to events.
- [x] Retire `model-completion-events.md` and
      `model-completion-streaming.md`.
- [x] Keep the current retry and fallback rules in force until Milestone 8.
      Move them from the retired docs into `model-policies.md` as the rules
      in force, next to the target rules.
- [x] Update `live-model-api-tests.md`: deferred and schema calls now emit
      model events, so the live checks expect events for them.
- [x] Run `cargo xtask check`, then commit. Do not push for this task.

Verification evidence: `.context/milestone-6/parity.md`,
`.context/milestone-6/snapshot-review-root.md`, and `.context/milestone-6/gate.log`.

## Milestone 7: Conversation parts and provider state

Depends on: 6

Replace the closed replay enum with opaque provider state.

- [ ] Replace the message content fields and `ProviderConversationItem`
      with ordered parts and provider state.
- [ ] Move the replay payload of each provider into its adapter as a
      private typed struct. This includes the Anthropic and Gemini variants
      of item 1.
- [ ] Add each provider crate's conversion of its version 1 items.
- [ ] Redact provider state in log copies.
- [ ] Replace the interim `provider_context` field of the finish event
      with per-part provider state.
- [ ] Mark `conversation-parts.md` and `model-stream.md` implemented.
- [ ] Run `cargo xtask check`, then commit and push.
- [ ] Run the ship steps for pull request 4.

## Milestone 8: HTTP facts and policy layers

Depends on: 7

Give the policies the response facts that they need, and make retry and
fallback explicit.

- [ ] Expose response headers on buffered responses, streams, and status
      errors in `json-http`.
- [ ] Parse `Retry-After` and the provider request id into typed fields.
- [ ] Serialize typed request bodies straight to bytes.
- [ ] Add the shared error core, and use it in every modality error.
- [ ] Split the provider error into the kinds of decision 5. Make each kind
      state whether another model can succeed.
- [ ] Add `ai-models-policy`. Move the retry, concurrency, and pricing
      wrappers from `ai-models-core`. Merge `ai-models-multi` into it.
- [ ] Make retry back off with jitter, honor retry-after, and cover rate
      limits.
- [ ] Make fallback require a policy, with the presets "any error" and
      "only when another model can succeed". Report every attempt.
- [ ] Keep the provider message of every rate-limit error in the error
      core. The MiniMax and xAI adapters lose it today, as their
      `rate_limit` snapshots show.
- [ ] Apply `plans/follow-ups/xai-deferred-poll-transport-error.md`, so
      that deferred polling continues after a network failure of the
      production transport.
- [ ] Mark `model-policies.md` implemented.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 9: Provider registry

Depends on: 8

Build any catalog entry from one config shape.

- [ ] Add `ai-models-registry` with one Cargo feature for each provider.
- [ ] Add `ProviderConfig`. Make every adapter honor its base URL.
- [ ] Build chat, image, video, transcription, and judgment adapters from
      specs.
- [ ] Compose the provider conversions into the converter that the
      registry exposes.
- [ ] Replace the provider match in
      `xtask/tests/live_models/provider_tests.rs` and in the smoke test
      with the registry.
- [ ] Mark `provider-registry.md` and `chat-completions-dialects.md`
      implemented.
- [ ] Run `cargo xtask check` and `cargo xtask smoke-test`, then commit
      and push.
- [ ] Run the ship steps for pull request 5.

## Milestone 10: Agent loop

Depends on: 9

Rebuild the tool loop on the stream and the caller-owned conversation.

- [ ] Change `run` to take the caller's conversation and the request.
- [ ] Stream agent events, including tool and fallback events. The tool
      finished event carries the tool execution record, and the step
      finished event carries the messages that the step appended.
- [ ] Build each step response with the shared fold of `ai-interface`,
      the same fold that `Model::complete` uses.
- [ ] Run tool calls in parallel up to a limit, and append the results in
      call order.
- [ ] Append the assistant message before the tool calls of its step run,
      and each tool result as its call completes, so that a stopped turn
      keeps every completed tool result.
- [ ] Send observer errors to `tracing`. Never fail a turn on them.
- [ ] Keep the `Truncated` outcome of item 4 in the new loop.
- [ ] Accept a schema for the final answer.
- [ ] Offer `tool_output_read` only after a windowed output exists.
- [ ] Add `TypedTool`. Add `schemars` with `cargo add`.
- [ ] Mark `agent-loop.md` implemented.
- [ ] Run `cargo xtask check` and `cargo xtask smoke-test`, then commit
      and push.
- [ ] Run the ship steps for pull request 6.

## Milestone 11: MCP evaluation

Depends on: 1

Decide with evidence whether `rmcp` replaces the custom MCP client.

- [x] Compare `rmcp` with `docs/protocol/ai-mcp.md` and
      `docs/protocol/mcp-oauth.md`. Cover protocol versions, sessions, SSE
      limits, auth challenges, discovery, registration, PKCE, refresh, and
      URL safety.
- [x] Save the comparison under `.context/ai-library-redesign/`.
- [x] Add the MCP decision to this plan, run the options check, and ask
      the user.

## Milestone 12: MCP change

Depends on: 8, 11

Apply the MCP decision.

- [ ] Apply the answer to item 16.
- [ ] Apply the answer to item 17, and the answers to items 18 and 19
      when they fall inside this plan.
- [ ] Run `cargo xtask check`, then commit and push.
- [ ] Run the ship steps for pull request 7.

## Milestone 13: Conformance

Depends on: 10, 12

Finish the safety net. `AGENTS.md` already follows the shared skills since
pull request #23.

- [ ] Run every adapter through every conformance scenario.
- [ ] Check replay for each spec that has the replay need, as
      `model-capabilities.md` requires.
- [ ] Update the workspace `README.md`.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 14: Verification, review, and close

Depends on: every other milestone

- [ ] Run `cargo xtask check` and `cargo xtask smoke-test`.
- [ ] Commit with Conventional Commits and push the branch.
- [ ] Review the complete local diff against `origin/main` with the
      `implementation-review` skill after the push. Report the findings,
      then apply the fix rule: fix the `fix now` findings, record
      follow-up items under `plans/follow-ups/` for the rest, re-review
      once, and report.
- [ ] Delete this plan when the `plans` skill says, under "Deleting the
      plan".
- [ ] Open pull request 8. Its description gives the decisions and the
      design calls of this plan with their dates, a permalink to this
      plan, and the follow-up items that the change adds.

## Post-merge follow-up (non-blocking)

- [ ] Firna adopts the new API at a pinned revision.
- [ ] Juno adopts the new API and converts its stored conversations.
