# Anthropic truncation inside tool arguments fails

- **Severity:** Low. **Likelihood:** Rare. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-02, from branch `calummoore/redesign-4-stream-parts`. A check of review finding 40 found it outside its scope.
- **Location:** `crates/ai-models-anthropic/src/anthropic/stream/block.rs:156`
- **Reason:** The adapter behaved the same way before the redesign, so the fix waits for a branch that works on Anthropic truncation.

- **Terms:**
  - **Truncated finish:** the finish reason of a response that reached its output limit.
  - **Interrupted kind:** the error kind of a stream that failed after progress. The retry layer never retries it.
- **Background:** The Anthropic adapter parses the argument JSON of each `tool_use` block when the block ends. When `max_tokens` stops the answer inside a block's argument JSON, the JSON is incomplete. `docs/protocol/model-stream.md` says that only a finish reason of tool calls needs complete argument JSON, and that a consumer acts on a tool call only after a tool-calls finish.
- **Problem:** The adapter fails the attempt with the interrupted kind instead of finishing with the truncated reason.
- **Impact if no action:** A caller gets an error for a normal truncated answer, and loses the text and reasoning of that answer.
- **Root question:** How should the adapter end an answer that its output limit cut inside a tool call?
- **Options:**
  - **A.** Do not parse the arguments of a block that the stop cut. Finish with the truncated reason, so that the fold drops the incomplete call. Add a regression test. Effort: Small.
  - **B.** Parse argument JSON only when the finish reason is tool calls, in the shared fold, for every adapter. Effort: Medium.
- **Left out:** Keep the error. It fails the goal.
- **Wider fix:** B. One rule for every adapter.
- **Previous examples:** None found.
- **Related:** `plans/follow-ups/truncated-call-replay.md`, the same truncation path.
- **Recommendation:** A. It fixes the defect at its source with the smallest change.
- **Action:** fix after merge, apply A, because the defect was on `main` before the redesign.
