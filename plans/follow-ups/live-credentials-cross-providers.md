# Live chat and image credentials can cross providers

- **Severity:** Medium. **Effort:** Medium. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Fix after merge.
- **Added:** 2026-11-26, from branch `calummoore/adding-openai-decisions-api-9ler`, review round 1, finding 5.
- **Location:** `xtask/tests/live_models/runner_tests.rs:24`, `xtask/tests/live_images/runner_tests.rs:24`, `.github/workflows/live-models.yml:61`, `.github/workflows/live-images.yml:43`.
- **Reason:** The chat and image suites and their credential contracts predate the judgment change.

- **Terms:**
  - **Live suites:** ignored tests that call provider APIs for chat models or image generation.
  - **Workflow matrix:** the GitHub Actions list that selects a provider's test and Actions secret for each job.
- **Background:**
  - The chat suite reads `LIVE_MODEL_API_KEY` for eight providers. The image suite reads `LIVE_IMAGE_API_KEY` for Google and OpenAI.
  - Each workflow gives the chosen test the secret from its matrix row through that suite's shared variable.
  - On 2026-11-27, the judgment suite adopted variable names derived from provider IDs. For example, it reads `LIVE_JUDGMENT_OPENAI_API_KEY` for OpenAI.
  - The judgment test fails before transport if its variable is absent or blank. The workflow exports the secret under `LIVE_JUDGMENT_` followed by the selected Actions secret name.
- **Problem:** The chat and image runners still select their keys from one variable per suite. A local run of all ignored tests can pass the same key to every provider in the suite.
- **Impact if no action:** A local run without a provider filter or a swapped workflow row can send a provider's key to another provider.
- **Root question:** How can each live chat or image test receive only the key assigned to its provider?
- **Options:**
  - **C.** Derive a variable from the suite and provider ID. Use `LIVE_MODEL_<PROVIDER>_API_KEY` for chat and `LIVE_IMAGE_<PROVIDER>_API_KEY` for images. Use uppercase provider IDs. Export the selected Actions secret under the suite prefix plus the selected secret name. Fail before the first request if the required variable is missing or blank. Name the variable in the error and omit its value. Add credential-free lookup tests. Update the workflows, `docs/protocol/live-model-api-tests.md`, `docs/protocol/live-image-api-tests.md`, `docs/development.md`, and `xtask/README.md`. Effort: Medium.
- **Left out:** A shared variable still permits cross-provider use during a local run. Generated workflow matrices add a separate change to provider registration.
- **Wider fix:** C. Every provider gets its credential name from the same naming rule.
- **Previous examples:** The judgment suite implements the same rule in `xtask/tests/live_judgments/provider_tests.rs` and `.github/workflows/live-judgments.yml`.
- **Related:** `plans/follow-ups/live-workflow-yaml-text-tests.md` covers the workflow text guards. Provider-specific lookup prevents a credential swap before a request.
- **Recommendation:** C. It checks credential selection in the test runner for both local runs and CI.
- **Action:** fix after merge, apply C, because the change affects the existing chat and image suites.
