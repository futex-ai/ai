# Native effort wire forms live only in READMEs

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/redesign-3-engine-specs`. A check of review finding 25 found it outside its scope.
- **Location:** `crates/ai-models-openai/README.md:49`, `crates/ai-models-anthropic/README.md:75`, `crates/ai-models-google/README.md:61`, `docs/protocol/provider-call-controls.md`
- **Reason:** The gap was already on `main`, so it waits for a branch that works on the native adapter docs.

- **Terms:**
  - **Effort:** the portable reasoning level of a request: disabled, low, medium, high, extra high, or max.
  - **Native adapter:** the OpenAI, Anthropic, or Google adapter, which each speak their own provider protocol.
- **Background:** Each native adapter maps each effort to the wire fields of its provider, for example the OpenAI `reasoning.effort` value, the Anthropic thinking block and effort, and the Google `thinkingConfig`. The crate READMEs of the three adapters list these forms. No protocol doc lists them. The `documentation` skill counts a wire format as a requirement, and requirements live in protocol docs.
- **Problem:** The effort wire forms of the native adapters are requirements without a protocol home. A reviewer cannot check a change of them against a committed requirement.
- **Impact if no action:** A change of an effort wire form can pass review unnoticed, because only a README describes it.
- **Root question:** Where should the effort wire forms of the native adapters be required?
- **Options:**
  - **A.** Add the effort wire forms to the native field table of `docs/protocol/provider-call-controls.md`, next to the output-limit and stop fields. Keep the README text as usage notes. Effort: Small.
  - **B.** Give each native adapter its own protocol page with its wire mapping, and link them from `provider-call-controls.md`. Effort: Medium.
- **Left out:** Leave the forms in the READMEs only. It fails the goal, because a README holds usage, not requirements.
- **Wider fix:** None. The `documentation` skill already puts requirements in protocol docs.
- **Previous examples:** None found.
- **Recommendation:** A. It puts the forms next to the other native fields at the smallest cost.
- **Action:** fix after merge, apply A, because the gap was already on `main`.
