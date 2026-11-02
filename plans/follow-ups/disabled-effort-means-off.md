# Disabled effort mixes "no control" and "off"

- **Severity:** Medium. **Effort:** Large. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/redesign-3-engine-specs`, item 27: the blocked wider fix of review finding 26.
- **Location:** `crates/ai-interface/src/` (`ModelSpec` and `Effort`), `crates/ai-models-core/src/capabilities.rs`, the custom defaults in `crates/ai-models-google/src/catalog.rs:23`, `crates/ai-models-anthropic/src/catalog.rs:31`, `crates/ai-models-openai/src/catalog.rs:23`, `crates/ai-models-compat/src/xai/catalog.rs:23`, `docs/protocol/model-capabilities.md`, `docs/protocol/provider-call-controls.md:99`
- **Reason:** The fix changes a public contract type and the effort rules of `docs/protocol/model-capabilities.md`, so the user decides.

- **Terms:**
  - **Effort:** the portable reasoning level of a request: disabled, low, medium, high, extra high, or max. `docs/protocol/model-capabilities.md` says that thinking is on when the resolved effort is not disabled.
  - **Custom defaults:** the per-provider values that `ModelSpec::custom` gives to a model that no catalog lists.
  - **Provider default:** the reasoning that a provider applies when a request sends no thinking field.
- **Background:**
  - Before the redesign, the thinking level "disabled" meant "send no explicit thinking control". The redesign made disabled a portable effort that means "thinking off". Each adapter still sends no thinking field for disabled.
  - For many reasoning models, no thinking field means the provider default, not off. Google documents that Gemini 3 Flash, Flash-Lite, and 2.5 Pro cannot turn thinking off. Anthropic documents that Claude 5 models use adaptive thinking when the request has no `thinking` field. OpenAI GPT-5.5 and xAI Grok 4.5 reason at a provider default when no effort is sent.
  - On 2026-11-02, the custom defaults of Google, Anthropic, OpenAI, and xAI default to disabled. `docs/protocol/provider-call-controls.md:99` says that the Anthropic custom defaults keep thinking off.
- **Problem:** The contracts can say "send no thinking control" only as the disabled effort, so specs, custom specs, responses, and the generated matrix claim "thinking off" where the provider reasons. Three known places show it: the Flash-Lite spec, the custom defaults of four providers, and the Claude 5 disabled requests before the redesign.
- **Impact if no action:** Each new model or custom spec can repeat the false off claim. Callers pay for reasoning that the response says did not happen.
- **Root question:** How do the contracts tell "the provider decides the reasoning" apart from "reasoning is off"?
- **Options:**
  - **A.** Keep the contracts. Fix each place by hand: give each affected spec and custom default a real listed effort, and map disabled only to a real off value. New specs can repeat the class. Effort: Medium.
  - **B.** Add a provider-default state to `ModelSpec` and the custom defaults. A call in that state sends no thinking field and reports no effort. Each adapter sends disabled only as an explicit off value, and `model-capabilities.md` records the rule. The sampling and tool-choice rules get a rule for an unknown thinking state. Effort: Large.
- **Left out:** Document that disabled can still think. It breaks the sampling and tool-choice rules, which treat disabled as thinking off.
- **Wider fix:** B. One contract state removes the false off claim for every provider and every new spec.
- **Previous examples:** `plans/follow-ups/flash-lite-disabled-effort.md` and `plans/follow-ups/claude-5-disabled-thinking.md`.
- **Related:** `plans/follow-ups/flash-lite-disabled-effort.md` (its option D is this item) and `plans/follow-ups/constructor-default-effort.md` (its answer sets the default effort that a migrated call uses).
- **Recommendation:** B. It removes the class at the contract, before more specs copy it.
- **Action:** ask the user, because it changes a public contract and the effort rules of `model-capabilities.md`.
