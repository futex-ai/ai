# xAI deferred polling abandons jobs on network errors

- **Severity:** High. **Likelihood:** Rare. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/redesign-3-engine-specs`. The review of the Chat Completions engine found it outside its scope.
- **Location:** `crates/ai-models-compat/src/xai/deferred.rs:117`, `docs/protocol/provider-call-controls.md:146`
- **Reason:** The redesign moved this code without a change, and the fix changes behaviour, so it waits for a branch that changes the xAI error handling.

- **Terms:**
  - **Deferred completion:** an xAI lifecycle that submits a job once, then polls for its result. xAI bills the job also when nobody reads the result.
  - **`ReqwestTransport`:** the `json_http::Error` variant that the production reqwest transport returns for a network failure.
- **Background:**
  - The xAI dialect of `ai-models-compat` submits a deferred job once, then polls the deferred-completion endpoint with the job id until the result arrives or the total timeout ends.
  - `docs/protocol/provider-call-controls.md`, "XAI Deferred Lifecycle", says that polling treats 202, 429, transient transport failures, and 5xx retrieval responses as retryable retrieval states. It also says that a retrieval retry never becomes another submission.
  - `retrieve` in `crates/ai-models-compat/src/xai/deferred.rs` re-polls on `json_http::Error::Transport` and `json_http::Error::Auth`. It maps every other client error to `ModelError::internal`.
  - The production transport, `ReqwestJsonHttpTransport` in `crates/json-http`, reports a network failure as `ReqwestTransport`, not as `Transport`. The test transports report `Transport`, so the tests pass.
- **Problem:** In production, a network failure during one poll fails the whole call with an internal error, although xAI accepted the job and keeps running it.
- **Impact if no action:** xAI bills the job, and the caller loses its result. The retry layer does not retry internal errors. A caller who retries by hand submits a second billable job.
- **Root question:** How should deferred polling treat a network failure of the production transport?
- **Options:**
  - **A.** Treat `ReqwestTransport` like `Transport` in `retrieve`, so that polling continues within the total timeout. Add a polling test that fails one poll with `ReqwestTransport`. Effort: Small.
  - **B.** Add one shared helper that says whether a `json_http::Error` is a transport failure, and use it in the deferred submission, the polling, and the engine's error mapping. Add the polling test. Effort: Small.
  - **C.** Make the reqwest transport report `Transport`, so that one variant covers every network failure. This changes the error text of every adapter. Effort: Medium.
- **Left out:** Change nothing. It breaks the polling rule of `provider-call-controls.md`.
- **Wider fix:** B. One helper keeps every caller in step when `json-http` adds or splits a transport variant.
- **Previous examples:** The `#[cfg(test)]` `request_error` functions of the DeepSeek, Kimi, and Qwen dialects list `Transport` and `ReqwestTransport` as separate arms (`crates/ai-models-compat/src/deepseek/client.rs:69`).
- **Recommendation:** B. It fixes the polling defect and removes the variant drift that caused it.
- **Action:** fix after merge, apply B, because the code moved without a change and a later branch rewrites the error classification.
