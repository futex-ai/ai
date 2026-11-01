# OpenAI live checks fail with HTTP 401

- **Severity:** High. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found while checking the live-test decisions of a plan.
- **Location:** `.github/workflows/live-models.yml`, `.github/workflows/live-images.yml`, `.github/workflows/live-videos.yml`
- **Reason:** The fix needs a new OpenAI credential that only the user can create and store as a repository secret.

- **Terms:**
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI workflows. Each call costs money.
  - **Repository secret:** an encrypted GitHub Actions value that a workflow reads, such as `OPENAI_API_KEY`.
- **Background:** The live model, image, and video workflows read the repository secret `OPENAI_API_KEY`. Their scheduled runs on `main` call OpenAI through the production adapters. The live model workflow last passed on 2026-10-20. The live image workflow last passed on 2026-10-23.
- **Problem:** OpenAI rejects the secret. The scheduled runs of 2026-11-01 fail with "HTTP 401: Incorrect API key provided" for `gpt-5.6-sol` and "Incorrect API key provided" for `gpt-image-2`. The live checks of the other seven chat providers pass.
- **Impact if no action:** No live check covers OpenAI. Every scheduled run and every pull request shows failed live checks, so a real OpenAI break looks like the known failure.
- **Root question:** How do the OpenAI live checks get a working credential, and how does the next failure reach a person?
- **Options:**
  - **A.** Create a new OpenAI API key, and replace the `OPENAI_API_KEY` repository secret. Effort: Small.
  - **B.** Create a dedicated OpenAI project for CI with a spend limit, and store its key as the secret. This also caps the cost of a leaked key or a runaway run. Effort: Small.
  - **C.** Do B, and make each scheduled live workflow open a GitHub issue after two failed runs in a row. Effort: Small.
- **Left out:** Pausing the OpenAI live jobs weakens a gate and hides OpenAI regressions. Mocked OpenAI checks fail the goal, because a mock cannot show a provider change.
- **Wider fix:** C. A failing schedule then reaches a person within two days.
- **Previous examples:** The live video workflow failed every scheduled run of the last 30. See `plans/follow-ups/veo-preview-model-retired.md`.
- **Related:** `plans/follow-ups/veo-preview-model-retired.md` has the same class of unnoticed scheduled failures. `plans/follow-ups/openai-video-errors-drop-status.md`: after the key works, a re-run shows whether OpenAI's video API still exists.
- **Recommendation:** C. It restores the checks, caps their cost, and makes the next silent failure visible.
- **Action:** ask the user, because the fix needs a credential that only the user can create.
