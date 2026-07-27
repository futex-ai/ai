# MCP JSON-RPC Identifier Validation

## Summary

Distinguish an omitted JSON-RPC identifier from explicit null and invalid
identifier values before classifying inbound MCP messages. Only a message
without an `id` member can be a notification, while malformed side messages
must fail before applying notification or request side effects.

The source of truth is
[`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).

## Milestone 1: Make Identifier Classification Presence-Aware

At the end of this milestone, inbound responses, server requests, and
notifications enforce the approved identifier presence/type contract while
retaining explicit-null JSON-RPC error handling.

- [x] Record the Codex cycle-9 finding and independently inspect the shared
      classifier, JSON and SSE callers, side effects, identifier types,
      existing regressions, and JSON-RPC request/response requirements.
- [x] Have Claude Code Fable 5 validate the finding, severity, null-ID nuance,
      implementation boundary, regression matrix, and documentation scope
      before editing implementation code.
- [x] Add failing classifier regressions for absent, string, integer,
      fractional, null, Boolean, object, and array identifiers across
      method-bearing messages, success responses, and error responses.
- [x] Add a failing SSE client regression proving a null-ID tool-list
      notification returns the typed malformed-message error without marking
      tools stale or posting a reply.
- [x] Introduce one private presence-aware identifier state and classify
      notifications, requests, success responses, and errors without changing
      public identifier or message-kind types.
- [x] Preserve valid string/number request echoing, fractional numeric IDs,
      explicit-null error responses, member precedence, and existing JSON-RPC
      version validation.
- [x] Update the MCP protocol and crate README with identifier presence/type
      validation and no-side-effect malformed-message behavior.
- [x] Run focused classifier, JSON client, SSE side-message, and MCP crate
      tests.
- [x] Have Claude Code Fable 5 audit the implemented solution, regressions,
      and documentation; resolve every valid issue before repository gates.
- [x] Run formatting, workspace Clippy with warnings denied, all workspace
      tests, Rust file-length lint, smoke tests, and `cargo xtask check`.
- [x] Review the complete diff for omissions, unrelated changes, Markdown
      errors, and untracked files.
- [x] Run `git add -A`, commit with a Conventional Commit title no longer than
      50 characters, and push the current branch.
- [ ] After the push, run the tenth and final authorized `cargo xtask review`
      cycle against `origin/main` and investigate every finding.
- [ ] Once the final review has no valid findings, mark this and all remaining
      review-fix plans completed in `plans/README.md`, then commit and push the
      final bookkeeping without starting an eleventh review cycle.
