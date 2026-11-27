# Live workflow tests assert YAML text

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found by the options check of a plan.
- **Location:** `xtask/tests/live_models.rs:116` (`workflow_covers_every_live_provider`), `xtask/tests/live_models.rs:134` (`workflow_runs_for_trusted_pull_requests`), `xtask/tests/live_images/workflow_tests.rs`, `xtask/tests/live_judgments/workflow_tests.rs`, `.github/workflows/live-models.yml`, `.github/workflows/live-images.yml`, `.github/workflows/live-judgments.yml`, `.github/workflows/record-provider-streams.yml:19-25`
- **Reason:** The 14 tests have been on `main` since 2026-09-21 or earlier, and their fix changes four workflows that run paid jobs.

- **Terms:**
  - **Live workflows:** the GitHub Actions workflows in `.github/workflows/live-*.yml` that run paid provider tests.
- **Background:**
  - Fourteen tests read workflow files with `include_str!` and assert their text: 2 in `xtask/tests/live_models.rs`, 6 in `xtask/tests/live_images/workflow_tests.rs`, and 6 in `xtask/tests/live_judgments/workflow_tests.rs`.
  - They check the trigger blocks, the matrix entries, and the conditions that keep secrets from fork and Dependabot pull requests. The image and judgment tests also check parallelism, timeouts, the order of the credential step, the exact `cargo test` command, and that no artifact is uploaded. They also read `.github/workflows/ci.yml` to check that ordinary CI holds no secret.
  - The workflows differ. `max-parallel` is 2 in `live-models.yml` and 1 in the others. `timeout-minutes` is 30 in `live-models.yml` and `record-provider-streams.yml`, and 20 in the other two. `live-judgments.yml` also needs the opt-in `vars.LIVE_JUDGMENT_API_TESTS_ENABLED`.
  - `.github/workflows/record-provider-streams.yml:19-25` has the same fork and Dependabot condition. No test reads that workflow.
  - The shared testing rules of this workspace's agents say that workflow YAML literals never get a test.
- **Problem:** These tests check YAML text, not behaviour. For example, `xtask/tests/live_models.rs:138` requires the literal `"  pull_request:\n    branches:\n      - main"`, so a correct edit, such as `branches: [main]`, fails it.
- **Impact if no action:** Each workflow edit needs a matching test edit. The secret conditions are guarded only by text matches.
- **Root question:** How does the workspace keep its live workflows safe and correct without tests of YAML text?
- **Options:**
  - **A.** Remove the YAML text tests, and rely on the workflow runs and on review. Risk: nothing then checks the fork and Dependabot condition, so review alone stops an edit that exposes secrets to a fork pull request. Effort: Small.
  - **B.** Keep the tests. Cost: each workflow edit still needs a matching test edit, and a correct edit can fail them. Effort: Small.
  - **C.** Move the shared job condition, with the fork and Dependabot rule, into one reusable workflow. `live-models.yml`, `live-images.yml`, `live-judgments.yml`, and `record-provider-streams.yml` call it. Give it inputs for the settings that differ: `max-parallel`, `timeout-minutes`, the judgment opt-in, and the label check and artifact upload of the recording workflow. Then remove the YAML text tests. Cost: four workflows change at once, and each must run once to prove the change. Effort: Medium.
- **Left out:** None.
- **Wider fix:** C. One reusable workflow holds the secret rule, so a new live workflow cannot drift from it.
- **Previous examples:** None found.
- **Related:** `plans/follow-ups/minimax-video-live-assertion-claim.md`: the same file. Its option B changes the upload step of `record-provider-streams.yml`, and option C of this item moves the job of that workflow into a reusable workflow.
- **Recommendation:** C. It removes the text tests and keeps the secret rule in one place.
- **Action:** fix after merge, apply C, because the change reaches four workflows and three test files, and each changed workflow must run once to prove it.
