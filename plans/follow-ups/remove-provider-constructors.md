# Remove the per-provider model constructors

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-05, from branch `calummoore/redesign-8-conformance`. The user moved this work out of the AI library redesign with the message "add as follow ups if applicable".
- **Location:** the `new`, `with_auth`, and `with_catalog_auth` constructors of `crates/ai-models-anthropic`, `crates/ai-models-openai`, `crates/ai-models-google`, `crates/ai-models-typesafe`, and the dialects of `crates/ai-models-compat`
- **Reason:** The work waits for the open decision item `plans/follow-ups/constructor-default-effort.md`, because one of its options keeps a constructor path.

- **Terms:**
  - **Registry:** `Registry` in `crates/ai-models-registry`. It builds every model, image, video, transcription, and judgment adapter from a model spec and a provider config.
  - **Per-provider constructors:** the old `new`, `with_auth`, and `with_catalog_auth` functions of each provider crate.
- **Background:**
  - The user approved the AI library redesign on 2026-11-01. Its list of replaced functionality says that the registry replaces the per-provider constructors.
  - The registry exists and builds every adapter. The constructors stay, with their current behaviour, because the open item `constructor-default-effort.md` asks how callers that never chose an effort keep their old requests, and one of its options keeps a constructor path.
- **Task:** After the user answers `constructor-default-effort.md`, delete the per-provider constructors, unless the answer keeps a path. Move every caller, test, and README example to the registry. Update the provider protocol docs that still name the constructors.
- **Impact if no action:** Two construction paths stay, with different default efforts for the same model.
- **Needs:** The user's answer to `plans/follow-ups/constructor-default-effort.md`.
- **Done when:** The constructors are gone or kept as the answer says, every caller builds through the registry, and `cargo xtask check` passes.
- **Related:** `plans/follow-ups/constructor-default-effort.md`, the decision that this work waits for.
- **Action:** ask the user, because the work waits for that decision.
