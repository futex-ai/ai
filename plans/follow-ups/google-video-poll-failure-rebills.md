# Google video poll failure can bill twice

- **Severity:** High. **Likelihood:** Rare. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-03, from branch `calummoore/redesign-5-policy-registry`. A check of review finding 118 found it outside its scope.
- **Location:** `crates/ai-models-google/src/google/video_generation/lifecycle.rs:37`, `docs/protocol/video-generation.md` (polling)
- **Reason:** The adapter behaved this way before the AI library redesign, and the fix changes the polling contract of `video-generation.md`, so the user decides.

- **Terms:**
  - **Video job:** a Google long-running operation that the video adapter submits once, polls until it finishes, and then downloads. Google bills the job also when nobody reads the result.
  - **Retried kind:** an error kind that the retry layer retries: rate limited or transient.
- **Background:**
  - The Google video adapter submits a job, polls its operation, and downloads the sample within one total deadline.
  - When a poll or the download fails with a transport failure, HTTP 429, or HTTP 5xx before the deadline, the adapter ends the call at once with a transient or rate limited error.
  - The xAI deferred lifecycle polls again after such failures, and `docs/protocol/provider-call-controls.md` requires that for xAI.
  - On 2026-11-03, the fix of review finding 118 made the adapter report the timed out kind when the deadline expires in any phase.
- **Problem:** After Google accepted the job, one failed poll or download makes the call fail with a retried kind. A caller or a retry layer that retries then submits a second billable job.
- **Impact if no action:** A short network failure during a video job can bill the job twice.
- **Root question:** How should the video adapter handle a failed poll or download of an accepted job?
- **Options:**
  - **A.** Keep today's behaviour, and document that a failure after acceptance can bill again on retry. Effort: Small.
  - **B.** Make the poll and the download retry after transport failures, HTTP 429, and HTTP 5xx while time remains, as the xAI deferred lifecycle does. A pending job then ends only with its video, a terminal provider answer, or the timed out kind. Change the polling contract of `video-generation.md`. Effort: Medium.
  - **C.** Give a failure after acceptance a kind that the retry layer never retries, such as interrupted, so that the caller decides. Effort: Small.
- **Left out:** Cancel the job before a retry: the adapter has no cancel call.
- **Wider fix:** B, together with the xAI item `plans/follow-ups/xai-deferred-timeout-rebills.md`: one rule that no accepted job is resubmitted by a retry.
- **Previous examples:** `plans/follow-ups/xai-deferred-timeout-rebills.md` and `plans/follow-ups/deepseek-resource-error-rebills.md`.
- **Recommendation:** B. It keeps the job alive within its deadline and never invites a second billable job.
- **Action:** ask the user, because B changes the polling contract of `video-generation.md`.
