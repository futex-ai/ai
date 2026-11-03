# Stopped steps leave calls without results

- **Severity:** Medium. **Likelihood:** Rare. **Effort:** Medium. **Confidence:** High. **Harder later:** Yes.
- **Kind:** Needs decision.
- **Added:** 2026-11-02, from branch `calummoore/ai-repo-overview`, review finding 11 (the blocked wider fix).
- **Location:** `docs/protocol/agent-loop.md`
- **Reason:** Options B and D add new tool outcomes and a new public function at the agent loop boundary that no requirement covers, so the user decides.

- **Terms:**
  - **Agent loop:** the runtime in `ai-tool-calling` that alternates model calls and tool calls over a conversation that the caller owns.
  - **Step:** one model response and the tool calls that it asks for.
  - **Checkpoint:** a caller callback that can stop a turn between two tool calls.
- **Background:** The target agent loop in `docs/protocol/agent-loop.md` appends the assistant message of a step before it runs the tool calls, and it appends each tool result when the call completes. When a turn stops in the middle of a step, the completed results stay in the conversation, as they do on `main` today. Calls that did not complete get no result, and the host must close them before the next turn. OpenAI and Anthropic reject a request in which a tool call has no result.
- **Problem:** A host that does not close the unfinished calls sends a request that the provider rejects. A call that was running when the caller dropped the stream may already have changed an external system, and the conversation does not say so.
- **Impact if no action:** Each host writes its own closing logic, and a host that forgets it gets provider errors on the next turn.
- **Root question:** When a turn stops in the middle of a step, how does the next turn learn what each tool call of that step did?
- **Options:**
  - **A.** Keep completed results, and leave unfinished calls to the host. This is in force since review finding 11 was fixed. Effort: Small.
  - **B.** Use one finalization path for success, error, checkpoint stop, and a dropped stream. Keep every completed result. Give each unfinished call an explicit unstarted or unknown result, and state that an unknown result does not read as a failure. Effort: Medium.
  - **D.** Do B, and also let a host rebuild a closed step after a process end. The response event and each tool-finished event carry all the data that the closed step needs, and one library function builds the closed step from saved events. Effort: Medium.
- **Left out:** Hosts rebuilding the step from events alone fails after a dropped stream, because a result can complete after the last event the caller read. Waiting for every call before a checkpoint stop breaks the rule that no new tool call starts after a checkpoint error.
- **Wider fix:** D. It covers every exit inside the process and a process end.
- **Previous examples:** `docs/protocol/tool-output-management.md` requires that a successful tool result survives a storage failure, for the same reason: the side effect has already happened.
- **Related:** None.
- **Recommendation:** B. It closes every call of a stopped step inside the library, so no host can forget it. D best protects hosts that resume after a restart.
- **Action:** ask the user, because B and D add outcomes and a function at the agent loop boundary that no requirement covers.
