# MCP SSE Line Endings

## Summary

Accept every WHATWG server-sent-event line terminator in the incremental MCP
streamable-HTTP decoder. The normative MCP contract remains
[`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md).

## Milestone 1: Incremental Terminator Compliance

Replace delimiter literals with a standards-aligned line scanner. At the end
of this milestone, CRLF, CR, LF, and every valid mixed event boundary dispatch
before EOF without mistaking one CRLF pair for a blank line.

- [x] Independently reproduce the cycle-8 P2 finding against the decoder and
      WHATWG grammar, then have Claude Code Fable 5 validate severity, parsing
      invariants, scope, and the test contract before implementation.
- [x] Add failure-first coverage for bare-CR framing and multiline data, every
      broken mixed boundary, immediate trailing-CR dispatch, a later orphan LF,
      and multiple events with mixed framing.
- [x] Add guard coverage proving one CRLF is not a blank line and a CR/LF pair
      split across chunks remains one line terminator.
- [x] Replace literal delimiter scans with a stateless line scanner that
      greedily recognizes CRLF, documents its trailing-CR/orphan-LF invariant,
      and preserves live dispatch, EOF, and byte-limit behavior.
- [x] Normalize CRLF and remaining CR line endings in that order before
      extracting and joining `data` fields.
- [x] Align the MCP protocol and crate README with all accepted WHATWG line
      endings, mixed framing, and chunk-boundary behavior.
- [x] Run formatting, targeted Clippy with warnings denied, both `ai-mcp`
      feature modes, integration/smoke tests, and the Rust file-length lint.
- [x] Have Claude Code Fable 5 independently inspect and test the completed
      parser, regressions, documentation, and out-of-scope boundaries.
- [x] Run `cargo xtask check` and resolve every failure.
- [x] Run `git add -A`, commit the checked work with a Conventional Commit
      title no longer than 50 characters, and push the current branch.
- [x] Run the final allowed `cargo xtask review` attempts against
      `origin/main`; cycle 9's finding and cycle 10's final finding were
      independently validated, fixed, and Fable-audited, so no further
      invocation is permitted in this ten-cycle workflow.
- [x] Once the authorized review loop has no valid finding left, move this and
      the AI MCP crate plan to Completed in `plans/README.md`, tick their final
      bookkeeping items, and commit and push those plan-only changes.
