# MCP Lazy Result Serialization

Remove discarded content serialization from the result-precedence path defined
by [`docs/protocol/ai-mcp.md`](../docs/protocol/ai-mcp.md). This is a
performance-only cleanup: model-visible output and error precedence must remain
unchanged.

## Milestone 1: Serialize Only the Selected Result

Ensure successful structured and single-text results do not build an unused
JSON content tree. At the end of this milestone, content serialization occurs
only for remote errors and multi-block fallback results.

- [x] Have Claude Code Fable 5 independently validate the review finding,
      severity, implementation direction, and feasible verification strategy.
- [x] Preserve the existing branch-complete precedence regression; do not add
      an artificial serializer trait or allocator-counting seam solely to
      observe a private pure-function optimization.
- [x] Move content serialization into the remote-error and fallback branches
      without changing output precedence, truncation, or typed error behavior.
- [x] Run focused and full repository gates, then have Claude Code Fable 5
      validate the completed solution.
- [x] Run `git add -A`, commit the green fix with a descriptive Conventional
      Commit whose title is at most 50 characters, and push the current branch.
- [x] Record that this finding came from the tenth and final authorized
      `cargo xtask review` cycle; do not start an eleventh cycle.
