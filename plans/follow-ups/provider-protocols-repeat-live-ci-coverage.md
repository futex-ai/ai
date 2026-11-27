# Provider protocols repeat the live CI coverage

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-27, from branch `ci-fix/live-model-apis-qwencloud-catalog`, found by the finding check of review finding 1.
- **Location:** `docs/protocol/deepseek-model-provider.md:207`, `docs/protocol/kimi-model-provider.md:195`, `docs/protocol/minimax-model-provider.md:235`
- **Reason:** The branch that found it removed only the QwenCloud job. These three docs are true today, and the branch did not change them.

- **Terms:**
  - **Live model APIs workflow:** `.github/workflows/live-models.yml`. It runs one billable matrix job for each listed chat provider.
  - **Live model API test protocol:** `docs/protocol/live-model-api-tests.md`. It owns the scope of the credentialed chat suite, its probes, and the CI coverage of each provider.
- **Background:** The DeepSeek, Kimi, and MiniMax provider protocols each have a "Required Verification" section. Each section names the test file `xtask/tests/live_models.rs` and says that GitHub Actions runs the provider's live suite from the Live model APIs workflow. The Qwen provider protocol had the same paragraph. When the QwenCloud job left the workflow, that paragraph became false. The fix replaced it with a link to the live model API test protocol.
- **Problem:** The three paragraphs copy a fact that the live model API test protocol owns, and they name a source file. The `documentation` skill says that each fact lives in one place and that a protocol doc names no source files.
- **Impact if no action:** The next change to the workflow matrix for one of these providers makes its protocol false, as it did for Qwen.
- **Root question:** How does each provider protocol state its live verification without a copy of the CI coverage?
- **Options:**
  - **A.** Replace each live paragraph with one sentence that links the live model API test protocol, as the Qwen protocol now does. Keep the MiniMax-M3 tool-choice requirement in the MiniMax protocol, because it is a MiniMax requirement. Effort: Small.
  - **B.** Remove each live paragraph with no link. Effort: Small.
- **Left out:** A test that compares the provider protocols with the workflow. The `testing` skill gives docs and workflow YAML no test.
- **Wider fix:** None. The `documentation` skill already says that each fact lives in one place.
- **Previous examples:** `docs/protocol/qwen-model-provider.md`, fixed on branch `ci-fix/live-model-apis-qwencloud-catalog`.
- **Related:** `plans/follow-ups/restore-qwencloud-live-job.md`.
- **Recommendation:** A. It keeps a pointer from each provider protocol to the one place that owns the live coverage.
- **Action:** fix after merge, apply A, because the three docs are true today and are outside the files of the branch that found them.
