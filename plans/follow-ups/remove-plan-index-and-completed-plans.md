# Plan index and merged plans stay on main

- **Severity:** Low. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from the branch `calummoore/agentsmd-documentation`.
- **Location:** `plans/README.md`, `plans/model-completion-streaming.md:204`, and the 45 plan files that `plans/README.md` lists under "Completed"
- **Reason:** The fix deletes 47 docs from `main`, which needs the user's approval, and the branch that found the problem changed only the agent rules.

- **Terms:**
  - **Plan index:** The file `plans/README.md`. It lists each plan under "Active" or "Completed".
  - **Status paragraph:** The first paragraph of a plan under the `plans` skill of `futex-ai/skills`. It starts with `Status: Active` while the plan is open.
- **Background:** On 2026-11-01 the user chose the `plans` skill as the plan rule of this repository. It replaced the old `AGENTS.md` rule that kept a plan index. Under the skill, no index file exists, and no README holds a plan reference. The pull request of a change deletes its plan. `plans/` still holds the index and 46 plans whose work has merged. No plan has a status paragraph.
- **Problem:** The index and the 46 merged plans contradict the `plans` skill. The index is also wrong. It lists `plans/model-completion-streaming.md` as active, but PR #18 for that plan merged on 2026-08-22. [`docs/protocol/model-completion-streaming.md`](https://github.com/futex-ai/ai/blob/cbe8b41e199c56b2fe6f40f21f0417ad9b232192/docs/protocol/model-completion-streaming.md?plain=1#L3) says that the work was implemented on 2026-08-22. The plan still has two open TODOs: a `cargo xtask review` run and a move to "Completed" in the index.
- **Impact if no action:** Agents and readers trust a wrong list of active plans. `scripts/active-plans.sh` of the `plans` skill finds no active plan, so the two sources disagree.
- **Root question:** Which plan files stay in the repository after their work has merged?
- **Options:**
  - **A.** Delete the index and the 46 merged plans. Remove each link to them from `docs/`, `plans/`, and each README. Check that a protocol doc holds each decision in force of a deleted plan. Effort: Medium.
  - **B.** Delete only the index, and keep the plans as history. Effort: Small.
  - **C.** Keep all the files. Move `plans/model-completion-streaming.md` to "Completed" in the index. Effort: Small.
- **Left out:** Add status paragraphs to the merged plans and keep them. The `plans` skill deletes a plan when its work merges, so a status paragraph does not apply to these plans.
- **Wider fix:** None. The `plans` skill already prevents new index entries and new merged plans on `main`.
- **Previous examples:** Commit `bb9d0da` removed the README links to the plan index on 2026-11-01.
- **Related:** `plans/follow-ups/move-ai-mcp-conventions-checklist.md`. `docs/protocol/ai-mcp.md` names `plans/README.md`, so options A and B wait for that item or include its fix.
- **Recommendation:** A. It brings `plans/` in line with the plan rule that the user chose, and each decision in force keeps its home in a protocol doc.
- **Action:** ask the user, because options A and B delete docs that are on `main`.
