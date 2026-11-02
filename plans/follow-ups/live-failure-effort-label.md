# Live probe failures do not name the effort

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/redesign-4-stream-parts`. A check of review finding 77 found it outside its scope.
- **Location:** `xtask/tests/live_models/runner_tests.rs:126`
- **Reason:** The failure lines named only the text id before the AI library redesign too, so the fix waits for a branch that works on the live tests.

- **Terms:**
  - **Live suite:** the paid tests under `xtask/tests/live_models`, which run one probe for each effort that a model spec lists.
  - **Text id:** the `provider/model` id of a model spec.
- **Background:** Each failure line of a live probe has the form `<text id>: <text>`. The retry rule reads the part after the last `": "`. One spec runs one probe for each listed effort.
- **Problem:** When one spec runs at several efforts, a failure line does not say which effort failed.
- **Impact if no action:** A failed live run needs a log search to find the effort that failed.
- **Root question:** How should a live failure name the probe that failed?
- **Options:**
  - **A.** Build one label for each probe in the runner from the text id, the effort, and the resolved lifecycle, and print it in place of the text id. The label must not contain `": "`. Update the expected failure lines. Effort: Small.
  - **B.** Return typed failure values with the probe facts, and let the retry rule match a variant instead of parsing text. Effort: Medium.
- **Left out:** None.
- **Wider fix:** B. Typed values remove the text parsing of the retry rule.
- **Previous examples:** None found.
- **Recommendation:** A. It names the failed probe at the smallest cost.
- **Action:** fix after merge, apply A, because the gap was on `main` before the redesign.
