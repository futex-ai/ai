# MiniMax retired the M2.7 catalog models

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-27, from branch `calummoore/adding-openai-decisions-api-9ler`, the CI of pull request #31.
- **Location:** `crates/ai-models-compat/src/minimax/catalog.rs:67`, `crates/ai-models-compat/src/minimax/catalog.rs:94`, `crates/ai-models-compat/src/minimax/catalog.rs:122`
- **Reason:** The fix deletes two public catalog spec constants that are on `main`, and the deletion of functionality needs the user's approval.

- **Terms:**
  - **Live model APIs:** the GitHub Actions workflow that sends one paid request to each chat catalog model of each provider. It runs on a schedule and on pull requests.
  - **Catalog spec:** a `ModelSpec` constant that a provider crate exports and lists in its `CATALOG` slice.
- **Background:**
  - The MiniMax catalog in `crates/ai-models-compat/src/minimax/catalog.rs` lists `MINIMAX_M3`, `MINIMAX_M2_7`, and `MINIMAX_M2_7_HIGHSPEED`.
  - Since at least 2026-11-26, the "MiniMax catalog" job of the Live model APIs workflow fails on `main` and on pull request branches.
  - The failure text for both M2.7 models is: `UnknownModel for minimax model MiniMax-M2.7: HTTP 404: The model "MiniMax-M2.7" is no longer available. (2069)`. The highspeed model gives the same text with its own id.
  - Scheduled runs on `main` failed this way on 2026-11-26 (run 45761263481) and 2026-11-27 (run 45945535982). The run of pull request #31 failed the same way (run 45982315095).
  - `MINIMAX_M3` still passes.
- **Problem:** MiniMax no longer serves `MiniMax-M2.7` and `MiniMax-M2.7-highspeed`, but the catalog still lists them.
- **Impact if no action:** The Live model APIs workflow stays red on every run. A host that selects either spec gets an `UnknownModel` error from MiniMax.
- **Root question:** What should the MiniMax catalog list now that MiniMax retired the M2.7 models?
- **Options:**
  - **A.** Delete `MINIMAX_M2_7` and `MINIMAX_M2_7_HIGHSPEED`, remove them from `CATALOG`, and move or remove the tests that name them, under the removal rule of the `testing` skill. Regenerate the control matrix with `cargo xtask capability-matrix`, and update `docs/protocol/minimax-model-provider.md`, `docs/protocol/minimax-responses.md`, and the compat README. Effort: Small.
  - **B.** Keep the constants for hosts that pinned them, but remove them from `CATALOG`, so the live suite and the registry catalog skip them. A host that still uses them gets the provider's `UnknownModel` error. Effort: Small.
  - **C.** Replace them with the current MiniMax models, if MiniMax lists successors. This needs the current MiniMax model list. Effort: Medium.
- **Left out:** Skipping the two models in the live suite only. It hides a catalog entry that cannot work.
- **Wider fix:** None. The live suite already detects retired models, as it did here.
- **Previous examples:** Commit `9923879`, "fix(models): remove retired video models".
- **Recommendation:** A. A spec that the provider no longer serves cannot work, and commit `9923879` removed retired models the same way.
- **Action:** ask the user, because A deletes public catalog specs that are on `main`.
