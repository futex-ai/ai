# Truncated answers end the turn as completed

- **Severity:** Medium. **Likelihood:** Common. **Effort:** Small. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-01, from branch `calummoore/ai-repo-overview`, review item 4.
- **Location:** `crates/ai-tool-calling/src/turn/active_turn.rs:202`
- **Reason:** No requirement covers truncation in the tool loop, so the fix adds a requirement that the user must approve.

- **Terms:**
  - **Tool loop:** the agent runtime in `ai-tool-calling` that alternates model calls and tool calls until the model finishes.
- **Background:** Each model response has a finish reason. `Truncated` means that a token limit or a context limit cut the answer off. The tool loop maps `Stop` and `Truncated` to the same `Completed` outcome.
- **Problem:** The outcome does not carry the finish reason. A caller cannot tell a cut-off answer from a finished one. If the limit cuts off a tool call, the call disappears without a signal.
- **Impact if no action:** Hosts show or store partial answers as final answers.
- **Root question:** How does the caller learn that the model's answer was cut off?
- **Options:**
  - **A.** Add the finish reason to the `Completed` outcomes. Callers can still ignore it. Effort: Small.
  - **B.** Add a `Truncated` variant to `StepOutcome` and `RunOutcome`. Every caller must handle it. Effort: Small.
  - **C.** Ask the model to continue after truncation, up to a limit. Report truncation only if the model still does not finish. Effort: Medium.
- **Left out:** Treating truncation as an error throws away a partial answer that still has value. Changing nothing fails the goal.
- **Wider fix:** B. The outcome type cannot hide truncation.
- **Previous examples:** None found.
- **Related:** `plans/follow-ups/anthropic-output-cap-4096.md` makes truncation common on Anthropic.
- **Recommendation:** B. A new variant makes every caller decide, and breaking changes are acceptable before production.
- **Action:** ask the user, because no requirement covers truncation in the tool loop, so the fix adds one.
