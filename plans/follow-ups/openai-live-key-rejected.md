# OpenAI live checks fail with HTTP 401

- **Severity:** High. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found while checking the live-test decisions of a plan.
- **Location:** `.github/workflows/live-models.yml`, `.github/workflows/live-images.yml`, `.github/workflows/live-videos.yml`
- **Reason:** The user replaced the key on 2026-11-01. The project and alert parts of options B and C still need the user's choice.

- **Terms:**
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI workflows. Each call costs money.
  - **Repository secret:** an encrypted GitHub Actions value that a workflow reads, such as `OPENAI_API_KEY`.
- **Background:** The live model, image, and video workflows read the repository secret `OPENAI_API_KEY`. Their scheduled runs on `main` call OpenAI through the production adapters. The live model workflow last passed on 2026-10-20. The live image workflow last passed on 2026-10-23. On 2026-11-01 at 23:15 UTC, the user's new key replaced the secret, after an OpenAI call with it returned HTTP 200.
- **Problem:** Until the replacement, OpenAI rejected the secret. The scheduled runs of 2026-11-01 failed with "HTTP 401: Incorrect API key provided" for `gpt-5.6-sol` and "Incorrect API key provided" for `gpt-image-2`. No step told a person, so the failure lasted about 12 days.
- **Impact if no action:** The next rejected key or retired model fails without notice for days again.
- **Root question:** How do the OpenAI live checks get a working credential, and how does the next failure reach a person?
- **Options:**
  - **A.** Create a new OpenAI API key, and replace the `OPENAI_API_KEY` repository secret. Effort: Small.
  - **B.** Create a dedicated OpenAI project for CI with a spend limit, and store its key as the secret. This also caps the cost of a leaked key or a runaway run. Effort: Small.
  - **C.** Do B, and make each scheduled live workflow open a GitHub issue after two failed runs in a row. Effort: Small.
- **Left out:** Pausing the OpenAI live jobs weakens a gate and hides OpenAI regressions. Mocked OpenAI checks fail the goal, because a mock cannot show a provider change.
- **Wider fix:** C. A failing schedule then reaches a person within two days.
- **Previous examples:** The live video workflow failed every scheduled run of the last 30, because Google retired its Veo preview model. Pull request #22 removes that model.
- **Related:** `plans/follow-ups/openai-video-errors-drop-status.md`: after the key works, a re-run shows whether OpenAI's video API still exists.
- **Recommendation:** C. It caps the cost of the paid checks and makes the next silent failure visible. The user applied option A on 2026-11-01.
- **Action:** ask the user, because options B and C need a new OpenAI project and a new workflow step that the user has not chosen.
