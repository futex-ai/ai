# xAI deferred timeout can bill twice

- **Severity:** High. **Likelihood:** Rare. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/ai-repo-overview`, item 10. The Milestone 1 docs implementor of `plans/ai-library-redesign.md` found it.
- **Location:** `crates/ai-models-xai/src/xai/deferred.rs:208`, `docs/protocol/provider-call-controls.md:172`
- **Reason:** The fix changes a requirement that `docs/protocol/provider-call-controls.md` states, so the user decides.

- **Terms:**
  - **Deferred completion:** an xAI lifecycle that submits a job once, then polls for its result. The provider can bill the job also when nobody reads the result.
  - **Retry layer:** `RetryingModel` in `ai-models-policy` calls a model again after rate limited or transient failures.
- **Background:** The xAI adapter submits a deferred job, gets a request id, and polls until the result arrives or the total timeout ends. When the total timeout ends after the job was accepted, the adapter returns a transient provider error. `docs/protocol/provider-call-controls.md` requires this classification. The retry layer retries transient errors.
- **Problem:** After a timeout, the retry layer calls the adapter again, and the adapter submits a second deferred job. The first job can still finish and bill. AGENTS.md lists "no duplicate billable submission" in its review focus.
- **Impact if no action:** A slow deferred job can bill two or three times, once for each retry attempt.
- **Root question:** What may happen after a deferred job outlives its total timeout?
- **Options:**
  - **A.** Classify a timeout after the job was accepted as an interrupted error. The retry layer never retries it, and a fallback can still move on. Effort: Small.
  - **B.** Classify it as the timed out kind of the redesign, which is never retried and lets another model try. Until the redesign lands, map it to interrupted. Effort: Small.
  - **C.** Keep the transient kind, and make the retry layer skip calls that use the deferred lifecycle. The retry layer must then read the completion mode. Effort: Medium.
- **Left out:** Changing nothing fails the goal, because a retry can submit a second billable job. Cancelling the first job before a retry fails the goal for now, because the adapter has no cancel call.
- **Wider fix:** B. One error kind for jobs that outlive their timeout covers xAI deferred calls and the video jobs alike.
- **Previous examples:** The rule that an interrupted stream is never retried, in `docs/protocol/model-stream.md`, exists for the same billing reason.
- **Related:** None.
- **Recommendation:** B. It names the case exactly and keeps the retry layer free of lifecycle checks.
- **Action:** ask the user, because the fix changes the timeout classification that `docs/protocol/provider-call-controls.md` requires.
