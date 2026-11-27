# The Qwen live test account has no paid access

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-27, from branch `calummoore/adding-openai-decisions-api-9ler`, the CI of pull request #31.
- **Location:** `.github/workflows/live-models.yml`, `xtask/tests/live_models/runner_tests.rs:108`
- **Reason:** The fix is a billing change in the QwenCloud account that owns the CI secret, which only the user can make.

- **Terms:**
  - **Live model APIs:** the GitHub Actions workflow that sends one paid request to each chat catalog model of each provider. It runs on a schedule and on pull requests.
- **Background:**
  - Since at least 2026-11-26, the "QwenCloud catalog" job of the Live model APIs workflow fails on `main` and on pull request branches.
  - The failure text is: `RejectedKey for qwen model qwen3.7-max: HTTP 403: The free quota has been exhausted. To continue accessing the model on a paid basis, please complete your payment information (or disable the "use free tier only" mode in the management console if already completed).`
  - Scheduled runs on `main` failed this way on 2026-11-26 (run 45761263481) and 2026-11-27 (run 45945535982). The run of pull request #31 failed the same way (run 45982315095).
  - The adapter maps the 403 to the `RejectedKey` error kind as designed, so the code works.
- **Problem:** The QwenCloud account of the CI secret runs on the free tier only, and that quota is used up.
- **Impact if no action:** The Live model APIs workflow stays red, and the Qwen adapters get no live coverage.
- **Root question:** How does the Qwen live job get an account that can call the catalog models?
- **Options:**
  - **A.** In the QwenCloud management console, add payment information, or turn off the "use free tier only" mode. Then rerun the job. Effort: Small.
  - **B.** Replace the CI secret with the key of another QwenCloud account that has paid access. Effort: Small.
  - **C.** Remove the Qwen job from the workflow until an account exists. The Qwen adapters then have no live coverage. Effort: Small.
- **Left out:** A retry, because the quota does not reset during a run. Skipping only `qwen3.7-max`, because the quota covers every model of the account.
- **Wider fix:** None. The failure is an account state, not a defect class.
- **Previous examples:** None found.
- **Recommendation:** A. It restores the live coverage with no code change.
- **Action:** ask the user, because the fix needs access to the QwenCloud account that owns the CI secret.
