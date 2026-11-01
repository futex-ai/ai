# OpenAI's video API is gone

- **Severity:** High. **Likelihood:** Common. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Needs decision.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, item 9.
- **Location:** `crates/ai-models-openai/src/catalog.rs:51`, `crates/ai-models-openai/src/openai/video_generation/client.rs`
- **Reason:** Every option deletes functionality on `main` or adds a provider, so the user decides.

- **Terms:**
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI workflows. Each call costs money.
- **Background:** `OpenAiVideoGenerator` calls OpenAI's video API for the `sora-2` catalog entry. `docs/protocol/video-generation.md` says that OpenAI planned to shut that API down on 2026-09-24. Pull request #22 removes the retired Google Veo model, so `sora-2` becomes the only video model in any catalog.
- **Problem:** OpenAI's video endpoint returns HTTP 404 with an empty body. On 2026-11-01, `GET https://api.openai.com/v1/videos?limit=1` returned 404 with a working key, while `GET /v1/models` returned 200. The live Sora job therefore fails with the empty message `""`.
- **Impact if no action:** The live video workflow fails on every run, and every `sora-2` call fails with an empty error.
- **Root question:** Which video model, if any, should the library still offer?
- **Options:**
  - **A.** Put the HTTP status into OpenAI video error messages, re-run the video workflow once, and then decide. Effort: Small.
  - **B.** Delete the `sora-2` catalog entry and the live video workflow. Keep both video adapters for caller-named models. Effort: Small.
  - **E.** Delete the OpenAI video adapter, the `sora-2` entry, and the live video workflow. Keep the `VideoGenerator` trait and the Google adapter for caller-named models. Effort: Small.
  - **C.** Delete video generation from the library: both adapters, the `VideoGenerator` trait, its protocol docs, and the live suite. Effort: Medium.
  - **D.** Add a video model from another provider, such as the Gemini Omni model that Google named, after a live check. Its API is unknown. Effort: Large.
- **Left out:** Changing nothing keeps a paid check that always fails.
- **Wider fix:** None. A retired provider endpoint has no structural prevention, and the live workflow already detects it.
- **Previous examples:** Google retired its Veo preview model, and pull request #22 removes it.
- **Related:** `plans/follow-ups/openai-video-errors-drop-status.md`: options E and C delete the code that it fixes.
- **Recommendation:** E. The OpenAI endpoint is gone, so its adapter cannot work, and the Google adapter keeps the shared video boundary for a future model. The first recommendation was A, and the 404 made A unnecessary.
- **Action:** ask the user, because every option deletes functionality on `main` or adds a provider.
