# Live workflow tests assert YAML text

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found by the options check of a plan.
- **Location:** `xtask/tests/live_models.rs:116`, `xtask/tests/live_images/workflow_tests.rs`, `xtask/tests/live_judgments/workflow_tests.rs`
- **Reason:** The tests exist on `main` outside the scope of the plan that found them, so their change waits for its own branch.

- **Terms:**
  - **Live workflows:** the GitHub Actions workflows in `.github/workflows/live-*.yml` that run paid provider tests.
- **Background:** Several tests in `xtask/tests/` read the live workflow files with `include_str!` and assert their text. They check the trigger blocks, the matrix entries, and the conditions that keep secrets from fork and Dependabot pull requests. The shared testing rules of this workspace's agents say that workflow YAML literals never get a test.
- **Problem:** These tests check YAML text, not behavior. A correct workflow change, such as a path filter, can fail them.
- **Impact if no action:** Each workflow edit needs a matching test edit. The secret conditions are guarded only by text matches.
- **Root question:** How does the workspace keep its live workflows safe and correct without tests of YAML text?
- **Options:**
  - **A.** Remove the YAML text tests, and rely on the workflow runs and on review. Effort: Small.
  - **B.** Move the shared job conditions, including the fork and Dependabot rule, into one reusable workflow that all three live workflows call. Then remove the YAML text tests. Effort: Medium.
  - **C.** Keep the tests. Effort: Small.
  - **D.** Generate the matrix rows of each live workflow from one provider table in `xtask`, as `cargo xtask capability-matrix` writes its doc section, and add a drift check to `cargo xtask check`. Then a matrix row that someone adds or removes by hand fails the gate without a test of YAML text. Blocked: it adds a gate step, which needs the user. `xtask` has no library target, so the live tests share the table only through a new library target. Effort: Medium.
- **Left out:** None.
- **Wider fix:** B. One reusable workflow holds the secret rule, so a new live workflow cannot drift from it.
- **Previous examples:** Branch `ci-fix/live-model-apis-qwencloud-catalog` (2026-11-27) removed the matrix text test `workflow_covers_every_live_provider` of the chat workflow under this rule, and its review raised option D as the wider fix.
- **Related:** None.
- **Recommendation:** B. It removes the text tests and keeps the secret rule in one place.
- **Action:** fix after merge, apply B, because the change is outside the scope of the plan that found it.
