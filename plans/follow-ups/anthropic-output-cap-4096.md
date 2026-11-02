# Anthropic output is capped at 4,096 tokens

- **Severity:** High. **Likelihood:** Common. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Fix after merge.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, review item 2.
- **Location:** `crates/ai-models-anthropic/src/anthropic/request.rs:11`
- **Reason:** The user chose option B on 2026-11-01, and a later branch applies it.

- **Terms:**
  - **Adapter:** the crate that maps the shared interface to one provider API, such as `ai-models-anthropic`.
  - **Catalog:** the known models in each adapter's `catalog.rs`, with their features and thinking level.
  - **Thinking:** reasoning that the provider does before the answer. A catalog entry sets its effort, from `Disabled` to `Max`.
- **Background:** Anthropic requires `max_tokens` on every request. The adapter sends the caller's limit, but never more than 4,096. With thinking on, thinking tokens count against the same limit.
- **Problem:** A long answer or a hard thinking step stops at 4,096 tokens. Recent Claude models accept much larger limits. The adapter always streams now, so a large limit does not risk an HTTP timeout.
- **Impact if no action:** Long Claude answers and large tool calls end early with `max_tokens`. The tool loop then reports them as completed.
- **Root question:** How large may an Anthropic response be?
- **Options:**
  - **A.** Raise the constant to a value that every catalog model supports. Effort: Small.
  - **B.** Add a max output for each model to the catalog. Use it as the default and the ceiling. Effort: Medium.
  - **C.** Send the caller's limit with no ceiling, and send a large default when the caller gives none. An old model can reject a default that is too large. Effort: Small.
- **Left out:** Changing nothing fails the goal, because long answers stay cut off.
- **Wider fix:** B. Catalog data gives each adapter a correct ceiling.
- **Previous examples:** None found.
- **Related:** `crates/ai-tool-calling/README.md`: the tool loop reports truncation as a distinct outcome.
- **Recommendation:** A. It is a one-line change that removes the common failure now. B best protects the codebase.
- **Action:** fix after merge, apply B, because the user chose B on 2026-11-01.
