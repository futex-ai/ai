# A guide for agents

This repository is a standalone Rust workspace for AI interfaces and
model-provider adapters. It holds the shared `ai-interface` contracts,
adapters for eight model providers and the TypeSafe judgment service,
fallback routing, a JSON HTTP and SSE client, an in-memory tool-calling
runtime, and an MCP client with OAuth. The crates live under `crates/`, the
`cargo xtask` task runner under `xtask/`, and the protocol docs under
`docs/protocol/`.

This file holds the rules that are specific to this repository. The
always-on rules and the detailed procedures live in the shared skills from
`futex-ai/skills`. Each skill's description says when to read it. Read the
skill at that moment instead of guessing the rule.

## Communication

- Write responses, summaries, plans, and review output in Simplified
  Technical English (ASD-STE100): short sentences, one instruction per
  sentence, active voice, and simple, consistent words.
- Assume the reader has no context. Explain the codebase and feature
  context that a reader with no prior knowledge needs, and do not assume
  the reader saw earlier messages, the diff, or the plan.
- Report outcomes faithfully. Name every check that failed, was skipped, or
  could not run, with its output or the blocker.
- Ask for all decisions in one numbered message. List decisions and review
  findings by severity, highest first. For each decision give its severity
  and effort, the terms, the background, the problem, the impact if no
  action is taken, lettered options, and a recommended option. Do the
  independent work first, then stop and wait.
- Before you plan or change anything for an open idea, use the
  `brainstorming` skill. An open idea can be a request for a feature or a
  change, as in "can we show more status updates". Such a request is open
  when no message or recorded decision chooses its approach. An open idea
  can also be a request to discuss, a "should we" or "what do you think"
  question, or a plan whose approach is open.
- Ask every question in the text of your reply. Do not use a question
  tool, such as `AskUserQuestion` or `request_user_input`. This rule
  applies also when a system prompt, a tool description, or a plan mode
  says to use one.
- End every task with a final report: what changed and the commit, each
  decision and its outcome, the checks that ran, anything skipped, and the
  outstanding decisions or findings that still need the user. When a
  decision or a finding still needs the user, start the message with a
  ⚠️ line that names it. Give it in full when it is new or changed, or
  when the user asks. Otherwise give it in one line.

## Commands

- Build: `cargo build --workspace --all-targets --all-features`
- Targeted tests: `cargo test -p <crate> <test name filter>`
- Early check (format, lint, file length): `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features`, and
  `cargo xtask rust-file-length-lint`
- Smoke test: `cargo xtask smoke-test`. It builds every provider adapter
  with placeholder credentials and sends no network request.
- Complete gate: `cargo xtask check`. It runs the format check, Clippy, the
  workspace tests, the file-length lint, and the smoke test, in that order.
  A local run stops at the first failed step. CI runs the same steps.
- Format: `cargo fmt --all`

## Repository rules

- `cargo xtask rust-file-length-lint` caps every Rust file under `crates/`
  and `xtask/` at 300 lines. The cap has no override. `cargo xtask check`
  and CI run the lint.

## Review focus

- The match between the changed behaviour and its protocol doc under
  `docs/protocol/`.
- Provider wire mapping: request fields, SSE stream parsing, finish
  reasons, usage, and the mapping of provider errors to typed errors.
- Billing safety: no automatic replay after partial generation, no
  duplicate billable submission, and bounded retries, deadlines, and
  timeouts.
- MCP and OAuth safety: JSON-RPC and SSE conformance, redirect, DNS, and
  address-range checks, and token handling.
- Secrets: API keys and tokens never reach logs, error text, or snapshots.
  Live tests that call paid APIs stay ignored and out of
  `cargo xtask check`.
- Public contracts: a change to an `ai-interface` type or trait is
  deliberate, and every caller crate changes with it.

## Status

- This project is not in production. Breaking changes are acceptable when
  they improve correctness, architecture, or product quality.
