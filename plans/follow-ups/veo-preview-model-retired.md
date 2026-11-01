# Google retired the Veo 3.1 preview model

- **Severity:** High. **Likelihood:** Common. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found while checking the live-test decisions of a plan.
- **Location:** `crates/ai-models-google/src/catalog.rs:21`, `crates/ai-models-google/src/google/video_generation/client.rs`
- **Reason:** Replacing or removing a catalog entry changes functionality on `main`, and a check of the successor model needs live credentials.

- **Terms:**
  - **Catalog:** the known models in each adapter's `catalog.rs`, with their features.
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI workflows. Each call costs money.
- **Background:** The Google adapter offers video generation through the catalog entry `veo-3.1-generate-preview`. The live video workflow calls it on a daily schedule. Every scheduled run of the last 30 failed.
- **Problem:** Google answers "This model models/veo-3.1-generate-preview is no longer available. Please update your code to use models/gemini-omni-1.1". Every Google video request through this catalog entry fails.
- **Impact if no action:** A caller that selects the Google video entry gets an error on every call.
- **Root question:** Which Google model should serve video generation?
- **Options:**
  - **A.** Keep the entry, but mark it unavailable so that routing skips it. Effort: Small.
  - **B.** Remove the Veo entry from the Google catalog until a replacement passes a live check. This deletes functionality on `main`. Effort: Small.
  - **C.** Replace the entry with the model that Google names, `gemini-omni-1.1`, after a live check shows that it accepts the same requests and returns the same video shape. Effort: Medium.
- **Left out:** Changing nothing fails the goal, because every call fails.
- **Wider fix:** None. The live video workflow already detects a retired model. `plans/follow-ups/openai-live-key-rejected.md` covers how a failing schedule reaches a person.
- **Previous examples:** None found.
- **Related:** `plans/follow-ups/openai-live-key-rejected.md` has the same class of unnoticed scheduled failures.
- **Recommendation:** C. It restores Google video generation with the successor that Google names.
- **Action:** ask the user, because the change replaces a catalog entry on `main`, and the check needs live credentials.
