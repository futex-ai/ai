# Copy AI Crates From Juno

## Summary

Copy every Rust crate with an `ai-` prefix from
`/Users/calummoore/projects/futex/juno/crates` into this repository and make the
resulting workspace build, test, and document itself independently from Juno.

Source crates identified:

- `ai-interface`
- `ai-models-anthropic`
- `ai-models-core`
- `ai-models-google`
- `ai-models-multi`
- `ai-models-openai`
- `ai-models-xai`
- `ai-tool-calling`

Important source dependency: the provider crates currently depend on the Juno
workspace crate `json-http`, which does not have an `ai-` prefix. The migration
must either copy `json-http` as an explicit support crate or replace that
boundary before the provider crates can compile in this repository.

## Milestone 1: Workspace Bootstrap

Create a standalone Rust workspace that can own the copied AI crates and run the
required local checks. At the end of this milestone, the repository should have a
valid Cargo workspace even before provider behavior is changed.

- [ ] Add root `Cargo.toml` with workspace package metadata, resolver, members,
      and shared dependency declarations needed by the copied crates.
- [ ] Add or adapt workspace tooling so `cargo xtask check`,
      `cargo xtask review`, and `cargo xtask rust-file-length-lint --all` are
      available in this repository.
- [ ] Add root `.gitignore` entries for Rust build outputs and local tool
      artifacts.
- [ ] Confirm the root `README.md` explains the workspace purpose, developer
      entry points, and links to `plans/README.md`.
- [ ] Run `cargo metadata` to verify the workspace manifest parses.

## Milestone 2: Copy AI Crates

Bring the AI crates across without changing behavior. At the end of this
milestone, all `ai-*` crate source, tests, and crate READMEs should be present
under `crates/` and tracked by Git.

- [ ] Copy `crates/ai-interface` from Juno.
- [ ] Copy `crates/ai-models-core` from Juno.
- [ ] Copy `crates/ai-models-anthropic` from Juno.
- [ ] Copy `crates/ai-models-google` from Juno.
- [ ] Copy `crates/ai-models-openai` from Juno.
- [ ] Copy `crates/ai-models-xai` from Juno.
- [ ] Copy `crates/ai-models-multi` from Juno.
- [ ] Copy `crates/ai-tool-calling` from Juno.
- [ ] Preserve source-adjacent `_tests_` directories, crate `README.md` files,
      snapshot files, and fixtures from each copied crate.
- [ ] Add every copied crate to workspace members and workspace dependencies.
- [ ] Review each copied crate README against the repo's Rust crate README
      requirements and update any stale Juno-specific references.

## Milestone 3: Resolve Non-AI Support Dependencies

Make the copied crates independent from unpublished Juno-only workspace
assumptions. At the end of this milestone, dependency resolution should not rely
on the original Juno checkout.

- [ ] Inventory all copied manifests for `workspace = true`, path, git, and
      unpublished dependencies.
- [ ] Decide the `json-http` strategy:
      - [ ] Option A: copy `crates/json-http` as a clearly documented support
            crate because the AI provider crates already depend on it.
      - [ ] Option B: replace `json-http` usage in the provider crates with a
            local AI-owned HTTP boundary or external dependency.
- [ ] Apply the chosen `json-http` strategy and document the ownership boundary
      in the relevant README files.
- [ ] Convert inherited workspace dependency versions into this repository's
      root `Cargo.toml` without pinning unnecessary external versions by hand.
- [ ] Run `cargo metadata` again and fix dependency resolution errors.

## Milestone 4: Compile, Test, And Smoke Test

Validate the copied crates as a working product surface. At the end of this
milestone, all copied crates should pass formatting, linting, tests, and a
basic runtime-facing smoke check.

- [ ] Run `cargo fmt --all -- --check`; if it fails, run `cargo fmt --all` and
      re-run the check.
- [ ] Run `cargo clippy --workspace --all-targets --all-features`.
- [ ] Run `cargo test --workspace --all-features`.
- [ ] Run `cargo xtask rust-file-length-lint --all`.
- [ ] Run a smoke check that exercises model-provider construction and
      tool-calling registration without requiring live provider API calls.
- [ ] Run `cargo xtask check` and fix any failures until it passes.

## Milestone 5: Documentation And Review

Finish the migration with documentation, a committed diff, and reviewer
feedback. Do not auto-fix review findings without explicit user direction.

- [ ] Update the root `README.md` with the final workspace summary, developer
      setup, crate map, public interfaces, and key code jumping-in points.
- [ ] Update `plans/README.md` to move this plan from active to completed after
      all previous milestones are done.
- [ ] Review `git diff origin/main...` for accidental unrelated changes,
      generated artifacts, missing files, and stale Juno paths.
- [ ] Run `git add -A`.
- [ ] Commit the completed work with a Conventional Commit message.
- [ ] Push the current branch.
- [ ] Run `cargo xtask review` after pushing so the AI reviewer checks the diff
      against `origin/main`.
- [ ] Report each review finding in the final message with severity, context,
      impact, solution options, and a recommended option.
