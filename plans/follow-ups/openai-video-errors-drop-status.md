# OpenAI video errors drop the HTTP status

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, found while checking the live-test decisions of a plan.
- **Location:** `crates/ai-models-openai/src/openai/video_generation/error.rs:29`
- **Reason:** The fix is outside the scope of the plan that found it, so it waits for its own branch.

- **Terms:**
  - **Live tests:** tests in `xtask/tests/` that call real provider APIs from CI workflows. Each call costs money.
- **Background:** The OpenAI video adapter builds each error from the HTTP status and the response body. The message comes from the body only. When the body has no error message, the adapter uses the raw body text. The chat path puts "HTTP <status>:" in front of each message. The Google image and video adapters fall back to "HTTP <status>". `docs/protocol/live-video-api-tests.md` says that OpenAI's video API was scheduled to shut down on 2026-09-24.
- **Problem:** An OpenAI video error leaves out the HTTP status. An empty body gives an empty message. The live run of 2026-11-01 failed with "provider failure for `openai` model `sora-2`: """, so nobody can tell a rejected key from a shut-down API.
- **Impact if no action:** Operators cannot tell why OpenAI video calls fail.
- **Root question:** How does a caller learn why an OpenAI video call failed?
- **Options:**
  - **A.** Fall back to "HTTP <status>" when the body has no error message, as the Google adapters do. Effort: Small.
  - **B.** Put the HTTP status in front of every OpenAI video error message, as the chat path does. Effort: Small.
  - **C.** Add a typed status field to a shared error core that every modality uses, so that no adapter can drop it. Effort: Medium.
- **Left out:** None.
- **Wider fix:** C. A typed field cannot be left out of a message.
- **Previous examples:** The chat path adds the status in `crates/ai-models-core/src/errors.rs`. The Google fallbacks are in `crates/ai-models-google/src/google/image_generation/error.rs` and `crates/ai-models-google/src/google/video_generation/error.rs`.
- **Related:** `plans/follow-ups/openai-video-api-gone.md`: OpenAI's video endpoint now returns HTTP 404 with an empty body, and some of its options delete this adapter.
- **Recommendation:** B. Every message then carries the status, as on the chat path. C best protects the codebase.
- **Action:** fix after merge, apply B, because the fix is outside the scope of the plan that found it.
