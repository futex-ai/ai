# Apply the MCP crate decisions

- **Severity:** Medium. **Effort:** Medium. **Confidence:** High. **Harder later:** No.
- **Kind:** Needs decision.
- **Added:** 2026-11-05, from branch `calummoore/redesign-8-conformance`. The user moved this work out of the AI library redesign with the message "add as follow ups if applicable".
- **Location:** `crates/ai-mcp`, `crates/ai-mcp-oauth`, `docs/protocol/ai-mcp.md`, `docs/protocol/mcp-oauth.md`
- **Reason:** The work waits for four open decision items: `plans/follow-ups/mcp-crate-direction.md`, `plans/follow-ups/mcp-empty-sse-data.md`, `plans/follow-ups/mcp-client-revision-target.md`, and `plans/follow-ups/mcp-oauth-newer-rules.md`.

- **Terms:**
  - **rmcp:** the official Rust SDK for the Model Context Protocol (MCP), crate `rmcp`.
  - **`json-http`:** the shared HTTP crate of this workspace, which every provider adapter uses.
- **Background:**
  - `ai-mcp` is the MCP client of this workspace, and `McpToolSet` gives MCP server tools to the agent loop. `ai-mcp-oauth` runs OAuth for MCP servers that need it. Both crates call reqwest directly, and both meet their protocol docs.
  - On 2026-11-02, an evaluation compared rmcp 3.8.0 with both crates: rmcp misses 23 rules of `ai-mcp.md` and 24 of `mcp-oauth.md`. It also found that the client fails on the empty SSE priming events of rmcp servers, and that both crates target only MCP revision 2025-06-18.
  - The redesign moved every provider adapter onto `json-http`. Since then, `json-http` exposes response headers on buffered responses, streams, and status errors, and it follows redirects with credentials scoped to the original origin. It has no option to turn redirects off, and MCP forbids redirects.
- **Task:** Apply the answer to `mcp-crate-direction.md` (where the MCP protocol logic and its HTTP code live). Apply the answer to `mcp-empty-sse-data.md` (how the client treats an SSE event without a JSON-RPC message). Apply the answers to `mcp-client-revision-target.md` and `mcp-oauth-newer-rules.md` when they fall inside this work. Update `ai-mcp.md`, `mcp-oauth.md`, `docs/architecture.md`, and the crate READMEs with the changes.
- **Impact if no action:** The MCP crates keep a second HTTP stack, and hosts cannot use rmcp servers in their default mode.
- **Needs:** The user's answers to the four decision items.
- **Done when:** The answered options are applied, `cargo xtask check` passes, and the four decision items and this item are deleted.
- **Related:** the four decision items above. This item holds their shared work.
- **Action:** ask the user, because the work waits for the four open decision items.
