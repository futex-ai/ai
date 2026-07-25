# OAuth Transport Status-Aware JSON

Align OAuth response decoding with endpoint status semantics in
[`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md) without changing
the public transport trait or response DTO.

## Milestone 1: Preserve Status Before JSON Requirements

Ensure successful revocation bodies are ignored as required by RFC 7009 and
non-success responses retain their HTTP status even when the body is not JSON.
At the end of this milestone, successful metadata, registration, and token
responses remain strictly JSON.

- [x] Have Claude Code Fable 5 independently validate the review finding,
      severity, RFC behavior, compatibility boundary, and implementation
      direction.
- [x] Add failing production-transport coverage for a non-JSON successful
      revocation response and an end-to-end disconnect regression proving local
      deletion completes without a spurious revocation failure.
- [x] Cover non-JSON error statuses for metadata, registration, token, and
      revocation endpoints; retain parsed JSON OAuth errors and reject non-JSON
      successful metadata, registration, and token responses.
- [x] Make bounded response decoding endpoint- and status-aware while
      preserving response limits, redirect handling, public transport types,
      and existing endpoint-specific consumers.
- [x] Align public API documentation, the OAuth protocol, and the crate README
      with status-aware JSON requirements.
- [x] Run focused and full repository gates, then have Claude Code Fable 5
      validate the completed solution.
- [ ] Run `git add -A`, commit the green fix with a descriptive Conventional
      Commit whose title is at most 50 characters, and push the current branch.
- [x] Record that this finding came from the tenth and final authorized
      `cargo xtask review` cycle; do not start an eleventh cycle.
