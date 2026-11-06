# Firna adopts the redesigned API

- **Severity:** Medium. **Effort:** Medium. **Confidence:** Medium. **Harder later:** No.
- **Kind:** Post-merge task.
- **Added:** 2026-11-05, from branch `calummoore/redesign-8-conformance`: the unchecked post-merge task "Firna adopts the new API at a pinned revision" of the AI library redesign plan, which this branch deletes.
- **Location:** None in this repository. The work is in the Firna repository. `docs/development.md` (section "Firna adoption") and `docs/protocol/provider-call-controls.md` (section "Firna Migration") give the steps.
- **Reason:** Firna is a separate repository that pins Git revisions of these crates, so it can adopt the redesigned API only after the redesign merges to `main`.

- **Terms:**
  - **Firna:** a downstream host application, in its own repository, that depends on the crates of this workspace at a pinned Git revision.
  - **Model stream:** `Model::stream`, which returns the ordered model events of one call. The provided `Model::complete` folds those events into one response.
  - **Registry:** `Registry` in `crates/ai-models-registry`. It builds every model, image, video, transcription, and judgment adapter from a model spec and a provider config.
- **Background:**
  - The AI library redesign, which the user approved on 2026-11-01, changed the public API of this workspace. This workspace does not change Firna.
  - The redesign removed or replaced these names and behaviours:
    - `Model::complete_with_events`, `ModelCompletionEvent`, and `ModelCompletionEventSink`: replaced by `Model::stream`.
    - `ProviderConversationItem`, its per-provider structs, and the `content` and `content_parts` pair of `ConversationMessage`: replaced by ordered parts and provider state.
    - `ProviderKind`: replaced by `ProviderId` constants.
    - `KnownModelSpec`, `KnownModelCatalog`, `ThinkingLevel` as a construction input, `resolve_catalog_thinking_level`, and the 24 thinking-variant catalog ids, such as `claude-opus-5-thinking-max`: replaced by `ModelSpec` and `Effort`.
    - The crates `ai-models-deepseek`, `ai-models-kimi`, `ai-models-qwen`, `ai-models-minimax`, and `ai-models-xai`: merged into `ai-models-compat` with the same wire behaviour.
    - The crate `ai-models-multi` and the wrappers of `ai-models-core`: moved into `ai-models-policy`. Fallback now needs an explicit policy instead of falling through on any error.
    - The shared mutable conversation API of `ToolCallingRuntime`, such as `replace_conversation` and `push_user_message`: replaced by a caller-owned conversation. `Turn`, `ActiveTurn::step`, `StepOutcome`, and `Turn::successful_tool_calls`: replaced by the agent events and outcomes. A truncated answer ends with the `Truncated` outcome instead of `Completed`.
    - Anthropic requests use the max output of each catalog entry instead of a 4,096-token ceiling. With Anthropic thinking on, the tool choices `Required` and `Function` fail with `UnsupportedControl`, and `RequiredOrAuto` sends `auto`.
    - A spec can leave its effort at the provider default, which sends no thinking field. The disabled effort sends a real off value. A spec with no listed effort rejects a requested effort. Gemini 3.5 Flash-Lite defaults to the low effort, sent as `thinkingLevel: "minimal"`.
  - The per-provider `new`, `with_auth`, and `with_catalog_auth` constructors still exist. `plans/follow-ups/remove-provider-constructors.md` removes them after the user answers `plans/follow-ups/constructor-default-effort.md`.
  - `ModelRouter`, `activity_verb`, and `group_for_tool` stay, because Firna and Juno use them.
  - `docs/development.md` (section "Firna adoption") gives six steps that Firna must take before it pins a revision with the model stream:
    1. Implement `JsonHttpTransport::execute_sse` in its `BoundedModelHttpTransport`, and keep its bounded rewrite of the request body. Without it, Firna compiles through the default method, which returns `SseUnsupported`, and every model call fails at runtime.
    2. Update its URL matching: a pattern for Google's `:generateContent` does not match the `:streamGenerateContent` endpoint that the Google adapter calls.
    3. Read `error.core().kind` to handle a stream that failed after progress, which has the interrupted kind.
    4. Treat its per-call `total_timeout` override as the overall deadline of the model stream. The override does not change the idle timeout.
    5. Implement `Model::stream` in each of its own `Model` implementations.
    6. Read `Model::stream` at its platform event boundary for live output. Forward text, reasoning, and fallback events in order. On a `Fallback` event, discard every event of the failed attempt.
- **Task:** Pin Firna to a revision of `main` that holds the redesign. Take the six steps above. Move each use of a removed name to its replacement. Take the call-control steps of `docs/protocol/provider-call-controls.md` (section "Firna Migration") that Firna has not taken yet. Build models through the registry where the constructors may go.
- **Impact if no action:** Firna stays on a revision before the redesign, and it gets no later fix or feature of this workspace.
- **Needs:** The redesign merged to `main`, and access to the Firna repository.
- **Done when:** Firna pins a revision of `main` that holds the redesign, it compiles without the removed names, and its tests pass.
- **Related:** `plans/follow-ups/juno-adopts-redesigned-api.md` is the same task for Juno. `plans/follow-ups/remove-provider-constructors.md` and `plans/follow-ups/constructor-default-effort.md` decide the constructors that Firna may still call.
