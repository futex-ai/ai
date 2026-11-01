# AI library redesign

Status: Active. On branch `calummoore/ai-repo-overview`, no pull request yet.
The work spans eight pull requests. Each earlier pull request keeps this plan.
The last one closes it, as the final milestone says.

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
removes. Plan approval: pending. Implementation starts after the user
approves this plan. The approval covers the list under "Removed or replaced
functionality".

Not covered: prompt caching, native structured-output modes, new providers,
and changes inside Firna or Juno. They adopt the new API after the merge.

Contract owners:

- New in Milestone 1: `docs/architecture.md`,
  `docs/protocol/model-stream.md`, `docs/protocol/conversation-parts.md`,
  `docs/protocol/model-capabilities.md`,
  `docs/protocol/chat-completions-dialects.md`,
  `docs/protocol/provider-registry.md`, `docs/protocol/model-policies.md`,
  and `docs/protocol/agent-loop.md`.
- [Provider call controls](../docs/protocol/provider-call-controls.md),
  [tool output management](../docs/protocol/tool-output-management.md), and
  [live model API tests](../docs/protocol/live-model-api-tests.md).
- The [workspace README](../README.md) and each crate README that a
  milestone changes.

## Design

| Component | Crate | Responsibility |
| --- | --- | --- |
| Contracts | `ai-interface` | Model, tool, and modality traits, stream events, conversation parts, provider state, model specs, efforts, and errors. It names no provider. |
| HTTP | `json-http` | Requests, SSE, response headers, typed bodies, and record-and-replay for tests. |
| Adapter helpers | `ai-models-core` | The fold from events to a response, the capability check, error classification, schema checks, and synthetic tool-call ids. |
| Native adapters | `ai-models-anthropic`, `ai-models-openai`, `ai-models-google`, `ai-models-typesafe` | One adapter for each native protocol. |
| Compatible engine | `ai-models-compat` | One Chat Completions engine, with a dialect for DeepSeek, Kimi, Qwen, MiniMax, and xAI. |
| Policies | `ai-models-policy` | Retry, fallback, concurrency, and pricing layers. |
| Registry | `ai-models-registry` | Provider config, and one factory for every catalog entry. |
| Agent loop | `ai-tool-calling` | Turns, parallel tools, outcomes, observers, and tool output windows. |
| MCP | `ai-mcp`, `ai-mcp-oauth` | The MCP client and its OAuth flow, until Milestone 12 decides on `rmcp`. |
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
  model, message, retry-after, and provider request id. Retry backs off
  with jitter, honors retry-after, and covers rate limits. Fallback follows
  decision 5 and reports every attempt.
- **Agent loop.** The caller owns the conversation and passes it to `run`.
  The loop streams agent events. It runs tool calls in parallel up to a
  limit and appends the results in call order. Observer errors go to
  `tracing` and never fail a turn. Outcomes follow item 4. A final answer
  can carry a schema. The loop offers `tool_output_read` only after a
  windowed output exists. `TypedTool` derives the JSON schema of a tool
  from its argument type.
- **Tests.** `ai-testkit` replays recorded provider streams through each
  adapter and snapshots the results. Live checks follow decision 6. The
  live schedule follows decision 7.

The review conversation used example crate names. They map as follows:
`ai-core` is `ai-interface`, `ai-http` is `json-http`, `ai-provider-*` is
`ai-models-*`, `ai-policy` is `ai-models-policy`, `ai-registry` is
`ai-models-registry`, and `ai-agent` is `ai-tool-calling`.

## Removed or replaced functionality

The plan approval covers these removals. Each milestone keeps all behavior
that this list does not name.

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
- The `AGENTS.md` sections that describe Juno only: the Expo app, mockups,
  Diesel, and seed migrations.

Decisions 5 and 7 and items 1 to 4 decide the fallback default, the live
triggers, the Anthropic output cap, and the replay of Anthropic and Google
state. The MCP crates change only after the MCP decision of Milestone 12.

## Decisions

Items 1 to 4 are open review findings, recorded as follow-up items. The
milestones that depend on them name them.

- Item 1: [Claude and Gemini drop replay data](follow-ups/claude-gemini-drop-replay-data.md).
- Item 2: [Anthropic output is capped at 4,096 tokens](follow-ups/anthropic-output-cap-4096.md).
- Item 3: [Anthropic forces tool use while thinking is on](follow-ups/anthropic-forced-tool-choice-thinking.md).
- Item 4: [Truncated answers end the turn as completed](follow-ups/tool-loop-hides-truncation.md).

Decisions 5 to 7 are open.

### 5. Fallback moves on after every error

- **Severity:** Medium. **Effort:** Medium.
- **Source:** Design.
- **Category:** Docs or spec.
- **Location:** `crates/ai-models-multi/src/lib.rs:43`, `docs/protocol/model-completion-streaming.md:147`
- **Terms:**
  - **Firna:** a downstream system that uses this workspace for model routing and benchmarks. It pins revisions of this repository.
- **Background:** `MultiModel` tries models in order. Today it moves to the next model after any error and returns only the last error. `docs/protocol/model-completion-streaming.md` states this rule and warns that it can bill two providers. The redesign replaces `MultiModel` with a fallback layer that takes a policy and reports every attempt. Firna uses fallback lanes in its benchmarks.
- **Problem:** The redesign must state which failures move a call to the next model. Today's rule treats a malformed request like an outage. Today's error classes also put failures that only one model has, such as an unsupported control or a rejected key, in the same class as malformed requests.
- **Impact if no action:** A malformed request runs on every model in the chain. This costs time and can bill several providers.
- **Root question:** Which failures should move a call to the next model?
- **Options:**
  - **A.** Keep "any error" as the default, and add the other policies as an opt-in. No caller changes behavior. Effort: Small.
  - **B.** Make "only when unavailable" the default: rate limits, overload, transient errors, interruptions, and context-limit errors. Firna's lanes change behavior when Firna migrates, without a visible choice. Effort: Small.
  - **C.** Require a policy when a caller builds a fallback, and ship the presets "any error" and "only when unavailable". Every caller states its choice in code. Effort: Small.
  - **D.** Do C, and base the second preset on the scope of each failure. In the error core of Milestone 9, each error kind states whether another model can succeed. Split today's provider error into an invalid request, a rejected key, an unknown model, an input that the model does not take, and invalid model output. The preset moves on after a kind that belongs to one model or provider, and stops after an invalid request or an internal error. The error mapping of every adapter changes. Effort: Medium.
  - **E.** Remove fallback from the library. Each caller owns its ordered attempts, its rule, and its reports. This deletes `MultiModel` and its restart events, which are functionality on `main`, and each caller must rebuild them. Effort: Medium.
- **Left out:** A custom predicate with no presets is worse than C and D on every point, because both also accept a custom predicate. All options except E assume that fallback stays a library layer. The source is the design text, and the user decision of 2026-11-01 does not cover each sentence of it.
- **Wider fix:** D. Each error kind states its scope in one exhaustive match, so a new kind cannot reach a preset without a scope.
- **Previous examples:** `RetryingModel` already retries only transient errors. `docs/protocol/model-completion-streaming.md` records the double-billing warning.
- **Related:** Item 1: the "any error" rule can hide its HTTP 400 failures behind a fallback.
- **Recommendation:** D. It keeps the explicit choice of C and makes the "only when unavailable" preset correct, because today's classes stop a chain on failures that the next model can take. The first recommendation was C, and the options check changed it.
- **Action:** ask the user, because the change replaces the fall-through rule that `docs/protocol/model-completion-streaming.md` states, and the design choice did not settle the default.

### 6. No live test checks multi-step behavior

- **Severity:** Medium. **Effort:** Medium.
- **Source:** Design.
- **Category:** Test.
- **Location:** `xtask/tests/live_models/runner_tests.rs:198`
- **Background:** The live tests call every chat catalog model with one text-only turn and tool choice "none". One MiniMax probe sends a tool and checks that the model calls it. No live test runs a tool round trip, a schema call, or a continuation with thinking on. The redesign changes how every adapter carries replay data. Milestone 2 defines conformance scenarios and records real streams, which tests replay without credentials.
- **Problem:** No test checks multi-step behavior against the rules that the providers apply today. Providers change those rules without notice, and a recording does not show the change.
- **Impact if no action:** Replay and tool-choice bugs reach callers, as items 1 and 3 suggest.
- **Root question:** How do the tests prove multi-step behavior against the real providers?
- **Options:**
  - **A.** Add a two-turn tool probe and a schema probe for one thinking model per provider. This adds about 24 paid calls to each run. Effort: Medium.
  - **B.** Add both probes for every chat catalog model. This gives the most coverage and makes each run about four times as expensive. Effort: Medium.
  - **C.** Rely on the recordings only, and re-record them on a schedule. Tests need no credentials, but they miss a rule change until the next recording. Effort: Medium.
  - **D.** Run the Milestone 2 conformance scenarios live, with one thinking model per provider, in place of separate probes. One scenario definition then serves the replay tests, the recordings, and the live check. Milestone 3 must wait for Milestone 2, and the live mode needs structural checks, because live text never matches a snapshot. Effort: Medium.
  - **E.** Choose the live models from the specs: one model for each distinct replay need and forced-tool-choice rule. This costs more calls than A and far fewer than B. It starts with the models of A and switches when Milestone 6 adds the rule fields. Effort: Medium.
- **Left out:** Changing nothing fails the goal, because a mock cannot show a provider rule. Watching provider changelogs fails the goal, because it does not run the requests that the adapters send. Error reports from Firna or Juno fail the goal, because callers meet the failure first. All options assume that live calls run in CI with repository secrets. The source is `docs/protocol/live-model-api-tests.md`. All options also assume that a prompt can make a thinking model call the tool, with one bounded retry for a miss.
- **Wider fix:** E. It reads the rule fields of each spec, so a new model family with new rules gets a live check without a hand edit.
- **Previous examples:** The MiniMax-M3 required-tool probe in `xtask/tests/live_models/tool_choice_tests.rs`. The MiniMax clean-EOF fix that a live run found, recorded in `plans/model-completion-streaming.md`.
- **Related:** Items 1 and 3: their option B adds the Anthropic and Google probes that this decision covers for every provider. Decision 7 sets how often the live checks run.
- **Recommendation:** D. It makes the same paid calls as A and removes a second definition of the same scenarios. The first recommendation was A, and the options check changed it. E best protects the codebase.
- **Action:** ask the user, because the option adds assertions to a paid CI gate, and the always-on rules reserve new gates for the user.

### 7. Every pull request pays for live suites

- **Severity:** Medium. **Effort:** Small.
- **Source:** Design.
- **Category:** Process.
- **Location:** `.github/workflows/live-models.yml:3`
- **Background:** Four workflows make paid provider calls: chat models, images, videos, and judgments. Each runs daily, on manual dispatch, and on every same-repository pull request to `main`. The judgment workflow runs only when the repository variable `LIVE_JUDGMENT_API_TESTS_ENABLED` is `true`. No workflow has a path filter. On 2026-11-01, a docs-only pull request on branch `conductor/readme-plans-link-ai` ran all four suites. `docs/protocol/live-model-api-tests.md` and its image, video, and judgment siblings state these triggers.
- **Problem:** A docs-only or plan-only pull request runs every paid suite in full. This plan opens about eight pull requests, and several of them change no provider code.
- **Impact if no action:** Each push to such a pull request pays for full catalog runs and video generation.
- **Root question:** How much paid live testing should each change and each day pay for? The options check widened the first question, "When should the paid live suites run?", because the cost also depends on how much of a suite runs.
- **Options:**
  - **A.** Keep today's triggers. Effort: Small.
  - **B.** Run a suite on a pull request only when its provider, HTTP, or test code changes. Keep the daily runs for chat, images, and judgments. Run video weekly and on manual dispatch. Effort: Small.
  - **C.** Run a suite on a pull request only when the pull request has a `live-tests` label. Keep the schedules. An author can forget the label. Effort: Small.
  - **D.** Run the paid suites only on schedule and on manual dispatch. A provider break can merge and wait for the next scheduled run. Effort: Small.
  - **F.** Run a small set on pull requests: the synchronous probe and one catalog model per provider. The daily run keeps the full catalog. A break in one catalog entry then shows only on the next day. Effort: Small.
  - **G.** Run the paid suites once for each merged change, through a merge queue or a push to `main`. A merge queue needs a repository setting and makes the suites a required check, which is a new gate. Effort: Small.
  - **E.** Derive the run set from the Cargo dependency graph. An xtask command maps the changed files to crates and runs a provider job when its crate or a crate that it depends on changed. A change to the test files of a suite runs the whole suite. This needs the command, its tests, and a selection job in four workflows. Effort: Medium.
- **Left out:** None. All options assume that fork and Dependabot pull requests never get secrets. The source is the four workflow files. G weakens that assumption, because a merge queue runs queued fork changes with the repository secrets.
- **Wider fix:** E. It reads the build graph, so it stays correct when this plan adds and deletes crates. B must list the paths of each suite by hand.
- **Previous examples:** The judgment workflow gates on `LIVE_JUDGMENT_API_TESTS_ENABLED`. The video workflow skips automatic retries to limit cost.
- **Related:** Decision 6 adds live checks whose cost this schedule controls.
- **Recommendation:** B. It is the smallest option that stops the payment for docs-only and plan-only pull requests. Milestone 1 applies it, so the later pull requests of this plan pay only for what they change. E best protects the codebase.
- **Action:** ask the user, because the options run a paid gate less often, and the live-test protocol docs state today's triggers.

## Design calls

All calls are mine, made on 2026-11-01.

- One plan for eight pull requests. Each diff stays reviewable.
- Evolve the crates in place. Firna pins revisions and can migrate once,
  and the recordings give the regression safety.
- Build the Chat Completions engine first, so the stream and parts
  milestones touch four adapters instead of eight.
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
- The MCP crates change only after Milestone 12 compares `rmcp` with the
  MCP protocol docs. A replacement deletes functionality on `main`.
- Reviews follow `AGENTS.md`: report findings without fixing them
  automatically, until Milestone 14 aligns `AGENTS.md` with the shared
  skills.

## Pull requests

| Pull request | Milestones |
| --- | --- |
| 1. Requirements | 1 |
| 2. Safety net and current fixes | 2, 3, 4 |
| 3. Engine and specs | 5, 6 |
| 4. Stream and parts | 7, 8 |
| 5. HTTP, policies, and registry | 9, 10 |
| 6. Agent loop | 11 |
| 7. MCP | 12, 13 |
| 8. Conformance, CI, rules, and close | 14, 15 |

Each milestone ends with `cargo xtask check`, a commit, and a push. Commit
titles use Conventional Commits with 50 characters or fewer. The last
milestone of a pull request then runs the ship steps:

1. Run `cargo xtask review` after the push.
2. Report each finding with a recommendation.
3. Open the pull request into `main`, and keep this plan.

## Milestone 1: Documentation and protocol

Depends on: none

Write the requirements and decisions of the target. New docs carry the
status "Approved, not implemented". A TODO that names a decision or an item
waits for its answer.

- [ ] Add `docs/architecture.md` with the target components, their
      responsibilities, and their boundaries.
- [ ] Add `docs/protocol/model-stream.md`: the stream method, the events
      and their order, the fold, deadlines, interruption, and wrappers.
- [ ] Add `docs/protocol/conversation-parts.md`: roles, parts, provider
      state, replay and skip rules, stored-format versions, the converter,
      and log redaction.
- [ ] Add `docs/protocol/model-capabilities.md`: specs, text ids, custom
      specs, effort, the capability check, and the control matrix. Apply
      the answers to items 2 and 3.
- [ ] Add `docs/protocol/chat-completions-dialects.md`. Link it from the
      DeepSeek, Kimi, Qwen, and MiniMax protocol docs.
- [ ] Add `docs/protocol/provider-registry.md`: provider config, base URL,
      auth, headers, timeouts, and the adapters that a spec builds.
- [ ] Add `docs/protocol/model-policies.md`: the error core, retry,
      fallback, concurrency, and pricing. Apply decision 5.
- [ ] Add `docs/protocol/agent-loop.md`: turns, steps, the caller-owned
      conversation, parallel tools, outcomes, observers, the final-answer
      schema, and the output reader. Apply item 4. Align
      `tool-output-management.md`.
- [ ] Update `live-model-api-tests.md` and its image, video, and judgment
      siblings for decisions 6 and 7.
- [ ] Apply decision 7 to the four live workflows, so that the later pull
      requests of this plan pay only for what they change.
- [ ] Update the protocol list of the workspace `README.md`.
- [ ] Check every relative link in the changed Markdown.
- [ ] Commit and push. Run the ship steps for pull request 1.

## Milestone 2: Recordings and conformance runner

Depends on: 1

Capture what each adapter does today, so that later milestones can prove
that they keep it.

- [ ] Add a recording transport and a replay transport to `json-http`
      behind its `test-support` feature. A recording holds the request and
      the raw response bytes.
- [ ] Add the `ai-testkit` crate with scenarios, recording storage, and a
      conformance runner. The runner snapshots each result with
      file-based `insta` snapshots.
- [ ] Define the scenarios: one text turn, a two-turn tool loop with
      thinking on, a schema call, an image input, a rate-limit error, and a
      stream cut after progress.
- [ ] Add a manual workflow that records the success scenarios for each
      provider. The user runs it, because it needs the provider secrets.
      The OpenAI recordings wait for
      `plans/follow-ups/openai-live-key-rejected.md`.
- [ ] Build the rate-limit and stream-cut scenarios from documented
      provider error bodies, because a live provider does not produce them
      on demand.
- [ ] Commit the recordings, and snapshot today's results, errors
      included.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 3: Live multi-step checks

Depends on: 2

Prove multi-step behavior against the real providers, as decision 6 says.

- [ ] Add the live checks that decision 6 chooses to
      `xtask/tests/live_models/`.
- [ ] Extend the credential-free guards to cover the new live checks.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 4: Fixes in the current design

Depends on: 2

Apply the direct options that the user chooses for items 1 to 4. An item
whose answer is its wider fix moves to the milestone that builds it.

- [ ] Apply the chosen option of each answered item.
- [ ] Delete the follow-up item of each applied option in the same commit.
- [ ] Re-record the affected scenarios, and update their snapshots.
- [ ] Run `cargo xtask check`, then commit and push.
- [ ] Run the ship steps for pull request 2.

## Milestone 5: Chat Completions engine

Depends on: 2

Replace five copies of one protocol with one engine and five dialects.

- [ ] Add `ai-models-compat` with the engine: request mapping, streaming,
      accumulation, response mapping, usage, and errors.
- [ ] Port DeepSeek, Kimi, and Qwen as dialects. Move their tests into
      dialect test modules, and keep every assertion.
- [ ] Port MiniMax with its stream normalizer and reasoning details.
- [ ] Port xAI with deferred completion and legacy function calls.
- [ ] Prove parity with the recordings and the moved tests.
- [ ] Delete the five crates. Update the workspace members, the READMEs,
      the docs, `xtask`, and CI.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 6: Model specs and capabilities

Depends on: 5

Put the facts of each model in one constant, and check requests against it.

- [ ] Add `ModelSpec`, `ProviderId`, `Effort`, and the capability types to
      `ai-interface`.
- [ ] Convert every catalog to `ModelSpec` constants and a `CATALOG`
      slice. Remove the thinking-variant entries.
- [ ] Add the `provider/model` lookup and `ModelSpec::custom`.
- [ ] Add the request effort, lowered to the spec.
- [ ] Add the capability check for the output limit, tool choice with
      thinking on, and the other controls. Apply items 2 and 3 when their
      answer is the wider fix.
- [ ] Add `cargo xtask capability-matrix`, and regenerate the matrix in
      `model-capabilities.md`. Mark that doc implemented.
- [ ] Run `cargo xtask check`, then commit and push.
- [ ] Run the ship steps for pull request 3.

## Milestone 7: One stream method

Depends on: 6

Make streaming the single model entry point.

- [ ] Replace `complete` and `complete_with_events` on `Model` with
      `stream` and a provided `complete` fold.
- [ ] Add the event types to `ai-interface` and the fold to
      `ai-models-core`.
- [ ] Emit events directly in the Anthropic, OpenAI, and Google adapters
      and in the engine. Remove the JSON round trip.
- [ ] Move the retry, concurrency, pricing, and fallback wrappers to
      stream semantics.
- [ ] Re-run the recordings, and migrate the snapshots to events.
- [ ] Mark `model-stream.md` implemented. Retire
      `model-completion-events.md` and `model-completion-streaming.md`.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 8: Conversation parts and provider state

Depends on: 7

Replace the closed replay enum with opaque provider state.

- [ ] Replace the message content fields and `ProviderConversationItem`
      with ordered parts and provider state.
- [ ] Move the replay payload of each provider into its adapter as a
      private typed struct.
- [ ] Add Anthropic thinking-block replay and Gemini thought-signature
      replay, as item 1 decides.
- [ ] Add the converter for the old stored format.
- [ ] Redact provider state in log copies.
- [ ] Mark `conversation-parts.md` implemented.
- [ ] Run `cargo xtask check`, then commit and push.
- [ ] Run the ship steps for pull request 4.

## Milestone 9: HTTP facts and policy layers

Depends on: 8

Give the policies the response facts that they need, and make retry and
fallback explicit.

- [ ] Expose response headers on buffered responses, streams, and status
      errors in `json-http`.
- [ ] Parse `Retry-After` and the provider request id into typed fields.
- [ ] Serialize typed request bodies straight to bytes.
- [ ] Add the shared error core, and use it in every modality error.
- [ ] Add `ai-models-policy`. Move the retry, concurrency, and pricing
      wrappers from `ai-models-core`. Merge `ai-models-multi` into it.
- [ ] Make retry back off with jitter, honor retry-after, and cover rate
      limits.
- [ ] Apply decision 5 to fallback, and report every attempt.
- [ ] Mark `model-policies.md` implemented.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 10: Provider registry

Depends on: 9

Build any catalog entry from one config shape.

- [ ] Add `ai-models-registry` with one Cargo feature for each provider.
- [ ] Add `ProviderConfig`. Make every adapter honor its base URL.
- [ ] Build chat, image, video, transcription, and judgment adapters from
      specs.
- [ ] Replace the provider match in
      `xtask/tests/live_models/provider_tests.rs` and in the smoke test
      with the registry.
- [ ] Mark `provider-registry.md` implemented.
- [ ] Run `cargo xtask check` and `cargo xtask smoke-test`, then commit
      and push.
- [ ] Run the ship steps for pull request 5.

## Milestone 11: Agent loop

Depends on: 10

Rebuild the tool loop on the stream and the caller-owned conversation.

- [ ] Change `run` to take the caller's conversation and the request.
- [ ] Stream agent events, including tool and fallback events.
- [ ] Run tool calls in parallel up to a limit, and append the results in
      call order.
- [ ] Send observer errors to `tracing`. Never fail a turn on them.
- [ ] Add the outcomes that item 4 decides.
- [ ] Accept a schema for the final answer.
- [ ] Offer `tool_output_read` only after a windowed output exists.
- [ ] Add `TypedTool`. Add `schemars` with `cargo add`.
- [ ] Mark `agent-loop.md` implemented.
- [ ] Run `cargo xtask check` and `cargo xtask smoke-test`, then commit
      and push.
- [ ] Run the ship steps for pull request 6.

## Milestone 12: MCP evaluation

Depends on: 9

Decide with evidence whether `rmcp` replaces the custom MCP client.

- [ ] Compare `rmcp` with `docs/protocol/ai-mcp.md` and
      `docs/protocol/mcp-oauth.md`. Cover protocol versions, sessions, SSE
      limits, auth challenges, discovery, registration, PKCE, refresh, and
      URL safety.
- [ ] Save the comparison under `.context/ai-library-redesign/`.
- [ ] Add the MCP decision to this plan, run the options check, and ask
      the user.

## Milestone 13: MCP change

Depends on: 12

Apply the MCP decision.

- [ ] Apply the chosen option: move to `rmcp` behind `McpToolSet`, or move
      the custom client onto `json-http`.
- [ ] Run `cargo xtask check`, then commit and push.
- [ ] Run the ship steps for pull request 7.

## Milestone 14: Conformance, CI, and repository rules

Depends on: 11, 13

Finish the safety net and align the repository rules.

- [ ] Run every adapter through every conformance scenario.
- [ ] Align `AGENTS.md` with the shared skills with the `agents-md`
      skill. Remove the sections that describe Juno only.
- [ ] Update the workspace `README.md`.
- [ ] Run `cargo xtask check`, then commit and push.

## Milestone 15: Verification, review, and close

Depends on: every other milestone

- [ ] Run `cargo xtask check` and `cargo xtask smoke-test`.
- [ ] Commit with Conventional Commits and push the branch.
- [ ] Run `cargo xtask review` after the push. Report each finding with a
      recommendation.
- [ ] Close this plan as the plan rules of the repository say at that
      time: move it to Completed in `plans/README.md`, or delete it.
- [ ] Open pull request 8. Its description gives the decisions and the
      design calls of this plan with their dates, a permalink to this
      plan, and the follow-up items that the change adds.

## Post-merge follow-up (non-blocking)

- [ ] Firna adopts the new API at a pinned revision.
- [ ] Juno adopts the new API and converts its stored conversations.
