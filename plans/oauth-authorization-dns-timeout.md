# OAuth Authorization DNS Timeout

Align browser-handoff destination validation with the bounded OAuth network
contract in [`docs/protocol/mcp-oauth.md`](../docs/protocol/mcp-oauth.md).
Authorization-server selection remains a host-owned interaction without a
library timeout and is outside this follow-up.

## Milestone 1: Bound Authorization DNS Preflight

Ensure a stalled authorization-host resolver cannot block explicit
authorization indefinitely. At the end of this milestone, the preflight uses
the configured HTTP timeout, fails through the existing typed DNS error, and
never creates callback state or opens the user agent after timeout.

- [x] Add a deterministic paused-time regression with a pending DNS resolver,
      an outer watchdog, and strict empty clock/user-agent mocks. Prove the
      request fails at exactly `http_timeout` rather than hanging.
- [x] Bound only the authorization destination DNS lookup with
      `McpOAuthConfig::http_timeout`, preserve ordinary resolver errors, and
      map timeout expiry to `Error::Dns`.
- [x] Align the config field documentation, OAuth protocol, crate README, and
      verification coverage with the browser-handoff DNS timeout.
- [x] Run focused and full repository gates, then have Claude Code Fable 5
      validate the completed timeout solution.
- [ ] Run `git add -A`, commit the green timeout fix with a descriptive
      Conventional Commit whose title is at most 50 characters, and push the
      current branch.
- [ ] After the push, run the final authorized `cargo xtask review` cycle and
      record all findings or the clean result.
