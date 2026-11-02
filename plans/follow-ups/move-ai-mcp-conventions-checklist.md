# Agent rules sit inside the MCP protocol doc

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from the branch `calummoore/agentsmd-documentation`.
- **Location:** `docs/protocol/ai-mcp.md:655`
- **Reason:** The fix touches a 700-line protocol doc, so the `documentation` skill also requires a split of that doc, which is larger than the agent-rules change that found it.

- **Terms:**
  - **Protocol doc:** A spec file under `docs/protocol/`. It holds the requirements and the decisions in force for one area.
- **Background:** `docs/protocol/ai-mcp.md` is the protocol doc of the `ai-mcp` crate, the MCP client. It has 700 lines. Its section "Conventions checklist (binding)" lists agent rules. Examples are `dyn` trait seams, no `unwrap`, the 300-line file cap, and the checks before completion. Its last rule is "add the plan to `plans/README.md`". The `documentation` skill of `futex-ai/skills` says that a protocol doc holds only requirements and decisions. It also says that a change that touches a doc over about 250 lines splits that doc into linked pages.
- **Problem:** The checklist is not a requirement of the MCP client. Each of its rules repeats a shared skill. Its last rule names the plan index, which the `plans` skill does not have. The user chose the `plans` skill on 2026-11-01.
- **Impact if no action:** A reader takes the checklist as a binding part of the MCP spec. Its plan-index rule contradicts the `plans` skill.
- **Root question:** How does the MCP protocol doc hold only requirements, within the length target?
- **Options:**
  - **A.** Delete the checklist, because the shared skills hold each of its rules. Split the rest of the doc into linked pages of about 250 lines each. Move any code names to `crates/ai-mcp/README.md`. Effort: Medium.
  - **B.** Delete only the plan-index rule of the checklist, and split the doc. Effort: Medium.
- **Left out:** Move the checklist to `crates/ai-mcp/README.md`. A crate README holds entry points and public behaviour, not agent rules, and the shared skills already hold each rule. Delete the checklist without a split. The `documentation` skill requires the split when a change touches a doc over the target.
- **Wider fix:** None. The `documentation` skill already keeps agent rules and mechanics out of protocol docs.
- **Previous examples:** None found.
- **Related:** `plans/follow-ups/remove-plan-index-and-completed-plans.md`. The deletion of the plan index waits for this fix, because this doc names the index.
- **Recommendation:** A. The shared skills hold every rule of the checklist, so the deletion loses no rule.
- **Action:** fix after merge, apply A, because the split of a 700-line doc is larger than the agent-rules change that found it.
