# Dialect tests check a copy of error mapping

- **Severity:** Medium. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/redesign-3-engine-specs`. The review of the Chat Completions engine found it outside its scope.
- **Location:** `crates/ai-models-compat/src/deepseek/client.rs:69`, `crates/ai-models-compat/src/kimi/client.rs:69`, `crates/ai-models-compat/src/qwen/client.rs:69`, `crates/ai-models-compat/src/deepseek/_tests_/client_tests.rs:44`, `crates/ai-models-compat/src/deepseek/_tests_/error_tests.rs:96`, `crates/ai-models-compat/src/kimi/_tests_/client_tests.rs:106`, `crates/ai-models-compat/src/qwen/_tests_/error_tests.rs:108`
- **Reason:** The review counted these functions as unchanged code, so the fix waits for a branch that works on the dialect tests.

- **Terms:**
  - **`ai-models-compat`:** the crate with one Chat Completions engine for the DeepSeek, Kimi, Qwen, MiniMax, and xAI dialects.
- **Background:** The DeepSeek, Kimi, and Qwen adapters were separate crates before. Each had a production `request_error` function that mapped a `json_http::Error` to a `ModelError`. The engine of `ai-models-compat` now owns that mapping. Each dialect client keeps a `#[cfg(test)]` copy of its old `request_error`, so that the moved tests compile. The tests at the listed lines call these copies.
- **Problem:** The tests check a test-only copy of the old mapping, not the engine's production error classification. A change of the engine's classification passes them.
- **Impact if no action:** The error mapping of three dialects has less test coverage than the test count suggests. A regression in the engine's mapping reaches callers.
- **Root question:** How should the dialect error tests exercise the production error path?
- **Options:**
  - **A.** Point the tests at the engine's production error mapping, and delete the three test-only copies. Keep every assertion. Effort: Small.
  - **B.** Test the engine's error classification once, with a table of `json_http::Error` values and dialect settings. Delete the copies and the dialect cases that the table covers. Effort: Medium.
- **Left out:** Keep the copies. It fails the goal, because the tests then never reach the production path.
- **Wider fix:** None. The `testing` skill already says that a test must exercise the production code, not a copy.
- **Previous examples:** None found.
- **Related:** `plans/follow-ups/xai-deferred-poll-transport-error.md`, the same class of variant drift: the copies list `Transport` and `ReqwestTransport` as separate arms.
- **Recommendation:** A. It moves the existing assertions onto the production path at the smallest cost.
- **Action:** fix after merge, apply A, because the review counted the code as unchanged.
