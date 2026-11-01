# Claude and Gemini drop replay data

- **Severity:** High. **Likelihood:** Common. **Effort:** Medium. **Confidence:** Medium. **Harder later:** Yes.
- **Kind:** Fix after merge.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, review item 1.
- **Location:** `crates/ai-models-anthropic/src/anthropic/response.rs:44`, `crates/ai-models-google/src/google/request.rs:39`
- **Reason:** The user chose option A on 2026-11-01, and a later branch applies it.

- **Terms:**
  - **Adapter:** the crate that maps the shared interface to one provider API, such as `ai-models-anthropic`.
  - **Thinking:** reasoning that the provider does before the answer. A catalog entry in each adapter's `catalog.rs` sets its effort.
  - **Replay data:** provider data that must go back to the provider on the next model call. It travels in `provider_context` on `ConversationMessage`.
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI. Each call costs money.
- **Background:** In a tool loop, the model calls a tool, gets the result, and continues in a new model call. Claude returns thinking blocks with a signature. Gemini 3 returns a `thoughtSignature` on function-call parts. Both providers document that the next call must send these back. Google documents an HTTP 400 when a Gemini 3 signature is missing.
- **Problem:** The Anthropic adapter parses thinking blocks and then drops them. The Google adapter never reads `thoughtSignature`. Nobody confirmed the failure against the live APIs.
- **Impact if no action:** Gemini 3 tool loops will probably fail on their second model call, and `MultiModel` can hide this by falling back. Claude tool loops lose their reasoning between steps, or fail.
- **Root question:** How does the library carry each provider's reasoning state across tool steps?
- **Options:**
  - **A.** Add Anthropic and Google variants to `ProviderConversationItem`, replay them, and add mock regression tests. Every other adapter must handle the new variants. Effort: Medium.
  - **B.** Do A, and add a live two-turn tool probe for each provider. The probe needs CI credentials. Effort: Medium.
  - **C.** Replace the closed enum with opaque provider state, then add Anthropic and Google replay. This changes the stored conversation format. Effort: Large.
- **Left out:** Turning thinking off in tool loops fails the goal, because the catalog offers thinking models for tool use. Changing nothing fails the goal if the providers enforce the rule.
- **Wider fix:** C. Opaque state removes the enum with a variant per provider. The live probe in B is the detection fix.
- **Previous examples:** The replay variants for OpenAI, DeepSeek, Kimi, Qwen, MiniMax, and xAI in `crates/ai-interface/src/messages/provider_context.rs`.
- **Related:** `plans/follow-ups/anthropic-forced-tool-choice-thinking.md` has the same cause: no live test runs a tool round trip. Both items share the live-probe detection fix.
- **Recommendation:** B. It fixes the bug in the current design and proves the fix against the real APIs. C best protects the codebase.
- **Action:** fix after merge, apply A, because the user chose A on 2026-11-01.
