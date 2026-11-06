# DeepSeek resource error can bill twice

- **Severity:** High. **Likelihood:** Rare. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/ai-repo-overview`, outside-scope problem of review round 2 of pull request 1.
- **Location:** `crates/ai-models-deepseek/src/deepseek/response.rs:142`, `docs/protocol/deepseek-model-provider.md:227`
- **Reason:** The fix changes a mapping that `docs/protocol/deepseek-model-provider.md` requires, so the user decides.

- **Terms:**
  - **Retry layer:** `RetryingModel` in `ai-models-policy` calls a model again after rate limited or transient failures.
  - **Progress:** a stream has progress after the adapter read its first provider event. A failure after progress means that the provider already generated tokens.
- **Background:** DeepSeek can end a stream with the finish reason `insufficient_system_resource`. The adapter maps it to a transient provider error, and `docs/protocol/deepseek-model-provider.md` requires that mapping. The finish reason arrives at the end of the stream, after the provider generated tokens.
- **Problem:** The retry layer retries transient errors, so it sends the whole request again after DeepSeek already billed a partial generation. The interruption rule of `docs/protocol/model-stream.md` exists to stop such retries: failures after progress are interrupted and never retried.
- **Impact if no action:** A DeepSeek call that hits this finish reason can bill twice or three times, once for each retry attempt.
- **Root question:** What may happen after DeepSeek reports insufficient resources at the end of a stream?
- **Options:**
  - **A.** Map the finish reason to the interrupted error, which the retry layer never retries. A fallback can still move on. Effort: Small.
  - **B.** Map it to interrupted when the stream had progress, and to transient when it is the first and only event. Effort: Small.
  - **C.** Keep transient, and make the retry layer skip errors that carry a progress flag. Every error then needs the flag. Effort: Medium.
- **Left out:** Changing nothing fails the goal, because a retry can bill a second generation.
- **Wider fix:** C, in the redesign form: the error core of `docs/protocol/model-policies.md` records progress, and the retry layer never retries after progress. The target docs already state that rule for stream failures.
- **Previous examples:** `plans/follow-ups/xai-deferred-timeout-rebills.md` is the same class: a transient error after billable work.
- **Related:** `plans/follow-ups/xai-deferred-timeout-rebills.md`: the same class and the same wider fix.
- **Recommendation:** B. It stops the double billing and keeps the retry for a resource error that arrives before any generation.
- **Action:** ask the user, because the fix changes the mapping that `docs/protocol/deepseek-model-provider.md` requires.
