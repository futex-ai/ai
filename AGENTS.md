# A guide for agents

`ai` is a standalone Rust library workspace for AI interfaces, model-provider
adapters, provider-agnostic model policy and fallback routing, JSON HTTP
transport, MCP and MCP OAuth clients, and an in-memory tool-calling runtime.
Library crates live under `crates/`, repository automation under `xtask/`
(run as `cargo xtask <command>`), protocol docs under `docs/protocol/`, and
plans under `plans/`.

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
- End every task with a final report: what changed and the commit, each
  decision and its outcome, the checks that ran, anything skipped, and the
  outstanding decisions or review items that still need the user.

## Commands

- Build: `cargo build --workspace --all-features`
- Targeted tests: `cargo test -p <crate> <test name pattern>`. For the
  credential-free guards of a live suite:
  `cargo test --locked -p xtask --test live_models` (or `live_images`,
  `live_videos`, `live_judgments`).
- Early check (format, lint, file length): `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets --all-features`, and
  `cargo xtask rust-file-length-lint`.
- Complete gate: `cargo xtask check`. It runs the format check, Clippy, the
  workspace tests, the file-length lint, and the smoke test in that order,
  and stops at the first failed step. CI runs the same steps.
- Format: `cargo fmt --all`
- Smoke test: `cargo xtask smoke-test`. It constructs every provider adapter
  with placeholder credentials and makes no network calls.

## Repository rules

- `cargo xtask rust-file-length-lint` fails when a Rust file under
  `crates/` or `xtask/` has more than 300 lines. It checks every file, with
  or without `--all`, and has no override. The gate and CI run it.

## Review focus

- Protocol alignment: the behaviour of code under `crates/` meets the
  requirements in its `docs/protocol/` doc. A change to that behaviour
  updates the protocol doc in the same change.
- Provider boundaries: `ai-interface` stays provider-neutral. Each provider
  adapter crate owns its wire mapping, usage normalization, and typed error
  translation. `ai-models-core` holds the shared provider-agnostic helpers,
  and `ai-models-multi` holds fallback routing.
- Credential-free checks: `cargo xtask check`, the smoke test, and tests
  that are not ignored send no request to a provider or another external
  host, and need no credentials. Integration tests can use loopback servers.
- Live workflows: forked and Dependabot pull requests get no secrets,
  retries stay bounded, and video submissions are never retried.

## Status

- This project is not in production. Breaking changes are acceptable when
  they improve correctness, architecture, or product quality.
