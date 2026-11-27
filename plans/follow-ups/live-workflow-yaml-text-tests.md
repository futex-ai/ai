# Live workflow tests assert YAML text

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found by the options check of a plan.
- **Location:** `xtask/tests/live_models.rs:116`, `xtask/tests/live_images/workflow_tests.rs:24-71`, `xtask/tests/live_judgments/workflow_tests.rs:24-75`
- **Reason:** The tests exist on `main` outside the scope of the plan that found them, so their change waits for its own branch.

- **Terms:**
  - **Live workflows:** the GitHub Actions workflows in `.github/workflows/live-*.yml` that run paid provider tests.
- **Background:** Several tests in `xtask/tests/` read the live workflow files with `include_str!` and assert their text. They check the trigger blocks, the read-only permissions, and the conditions that keep secrets from fork and Dependabot pull requests. Some also check the job limits, the test command, and that ordinary CI holds no live secret. The shared testing rules of this workspace's agents say that workflow YAML literals never get a test. The matrix text tests `workflow_covers_every_registered_provider` of the image and judgment workflows belong to `plans/follow-ups/live-workflow-matrix-sync.md`, because they check the matrix rows.
- **Problem:** These tests check YAML text, not behavior. A correct workflow change, such as a path filter, can fail them.
- **Impact if no action:** Each workflow edit needs a matching test edit. The secret conditions are guarded only by text matches.
- **Root question:** How does the workspace keep the triggers, the secret conditions, and the job settings of its live workflows correct without tests of YAML text?
- **Options:**
  - **A.** Remove these YAML text tests, and rely on the workflow runs and on review. Effort: Small.
  - **B.** Move the shared job conditions, including the fork and Dependabot rule, into one reusable workflow that all three live workflows call. Then remove these YAML text tests. Effort: Medium.
  - **C.** Keep the tests. Effort: Small.
- **Left out:** None.
- **Wider fix:** B. One reusable workflow holds the secret rule, so a new live workflow cannot drift from it.
- **Previous examples:** Branch `ci-fix/live-model-apis-qwencloud-catalog` (2026-11-27) removed the matrix text test `workflow_covers_every_live_provider` of the chat workflow under this rule.
- **Related:**
  - `plans/follow-ups/live-workflow-matrix-sync.md` is a separate question about the same workflows and the same rule: how the matrix rows stay in step with the live tests. The two answers do not depend on each other.
  - `plans/follow-ups/restore-qwencloud-live-job.md` adds the QwenCloud row back to `.github/workflows/live-models.yml`. It changes no trigger and no secret condition, so it changes no test of this item.
- **Recommendation:** B. It removes the text tests and keeps the secret rule in one place.
- **Action:** fix after merge, apply B, because the change is outside the scope of the plan that found it.
