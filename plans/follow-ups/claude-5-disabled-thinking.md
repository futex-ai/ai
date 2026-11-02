# Claude 5 models cannot turn thinking off

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Fix after merge.
- **Added:** 2026-11-02, from branch `calummoore/redesign-3-engine-specs`. The fix of review finding 23 split it off.
- **Location:** `crates/ai-models-anthropic/src/catalog.rs`, the Anthropic effort mapping in `crates/ai-models-anthropic/src/anthropic/request.rs`
- **Reason:** The fix adds a new wire mapping, and the fix of finding 23 only restored efforts whose old request reached the provider unchanged.

- **Terms:**
  - **Effort:** the portable reasoning level of a request: disabled, low, medium, high, extra high, or max.
  - **Model spec:** a `ModelSpec` constant that lists the efforts that one model supports.
- **Background:**
  - Each model spec lists its efforts, and the shared capability check rejects an effort that the spec does not list and that no lower listed effort covers.
  - The Claude Sonnet 5, Opus 5, and Fable 5 specs list low, medium, high, and max. They do not list disabled.
  - Before the redesign, a disabled request to these models left out the `thinking` field. Anthropic's current docs say that Claude 5 models then use adaptive thinking, so that request did not turn thinking off.
  - Anthropic documents `thinking: {"type": "disabled"}` to turn thinking off for Sonnet 5 and Opus 5. Fable 5 cannot turn thinking off.
- **Problem:** A caller cannot turn thinking off on Sonnet 5 or Opus 5. A disabled request fails with `UnsupportedControl` for effort.
- **Impact if no action:** Callers who want the lowest latency and cost on Sonnet 5 or Opus 5 must accept low effort, or build a custom spec with their own mapping.
- **Root question:** How can a caller turn thinking off on the Claude 5 models that support it?
- **Options:**
  - **A.** Change nothing. A disabled request keeps failing before transport. Effort: Small.
  - **B.** List disabled on the Sonnet 5 and Opus 5 specs, and map it to `thinking: {"type": "disabled"}`. Keep Fable 5 without disabled. Add one wire test for each model and regenerate the control matrix. Each listed effort adds one paid live probe. Effort: Small.
- **Left out:** List disabled with the old mapping that leaves out `thinking`. The provider then still thinks, so the response would report an effort that the provider never applied.
- **Wider fix:** None. The capability check already rejects efforts that a spec does not list.
- **Previous examples:** None found.
- **Recommendation:** B. It gives callers a real off switch on the two models that support one.
- **Action:** fix after merge, apply B, because it adds a new wire mapping.
