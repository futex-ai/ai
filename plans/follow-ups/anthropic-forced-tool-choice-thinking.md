# Anthropic forces tool use while thinking is on

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Small. **Confidence:** Medium. **Harder later:** No.
- **Kind:** Fix after merge.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, review item 3.
- **Location:** `crates/ai-models-anthropic/src/anthropic/request.rs:150`
- **Reason:** The user chose option A on 2026-11-01, and a later branch applies it.

- **Terms:**
  - **Adapter:** the crate that maps the shared interface to one provider API, such as `ai-models-anthropic`.
  - **Thinking:** reasoning that the provider does before the answer. A catalog entry in each adapter's `catalog.rs` sets its effort.
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI. Each call costs money.
  - **Firna:** a downstream system that uses this workspace for model routing and benchmarks.
- **Background:** `ModelToolChoice::Required` must force a tool call, or fail with `UnsupportedControl` before transport. `RequiredOrAuto` must force a tool call when the model can, and use auto when it cannot. Anthropic documents that a forced tool choice with thinking on is an error.
- **Problem:** With thinking on, the adapter still sends Anthropic `any` for both modes, and `tool` for `Function`. Nobody confirmed the failure against the live API. The Qwen and DeepSeek adapters already apply this rule to their thinking modes.
- **Impact if no action:** Forced tool calls on thinking Claude models will probably fail with HTTP 400. `RequiredOrAuto` callers, such as Firna benchmarks, get an error instead of the documented fallback.
- **Root question:** Which tool-choice modes can Anthropic honor while thinking is on?
- **Options:**
  - **A.** With thinking on, return `UnsupportedControl` for `Required` and `Function`, and send `auto` for `RequiredOrAuto`. Effort: Small.
  - **B.** Do A, and add a live forced-tool probe for one thinking Claude model. The probe needs CI credentials. Effort: Small.
  - **C.** Put tool-choice support for each thinking state in catalog data, and check it in shared code. Effort: Medium.
- **Left out:** Turning thinking off when a tool is forced. It changes model behavior without telling the caller.
- **Wider fix:** C. One shared check covers every provider.
- **Previous examples:** The Qwen and DeepSeek thinking rows in `docs/protocol/provider-call-controls.md`.
- **Related:** `plans/follow-ups/claude-gemini-drop-replay-data.md` has the same cause and shares the live-probe detection fix.
- **Recommendation:** B. It follows the Qwen and DeepSeek pattern and proves the provider rule. C best protects the codebase.
- **Action:** fix after merge, apply A, because the user chose A on 2026-11-01.
