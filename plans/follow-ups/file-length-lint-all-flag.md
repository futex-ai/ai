# File-length lint ignores its --all flag

- **Severity:** Low. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Outside the scope.
- **Added:** 2026-11-01, from the branch `calummoore/agentsmd-documentation`.
- **Location:** `xtask/src/file_length.rs:11`, `xtask/src/cli.rs:19`, `xtask/src/main.rs:32`, `xtask/src/_tests_/main_tests.rs:15`, `.github/workflows/ci.yml:46`, `README.md:138`, `xtask/README.md:93`
- **Reason:** The fix changes `xtask` code, CI, and two READMEs, and the branch that found the problem changed only the agent rules.

- **Terms:**
  - **File-length lint:** The command `cargo xtask rust-file-length-lint`. It fails when a Rust file under `crates/` or `xtask/` has more than 300 lines.
- **Background:** The lint takes an `--all` flag. The help text of the flag says "Audit every Rust file under crates/ and xtask/." The root `README.md`, `xtask/README.md`, and the CI workflow run the lint with `--all`. `cargo xtask check` calls the lint with the flag set. A parser test checks that `--all` parses.
- **Problem:** The lint ignores the flag. Its `run` function names the parameter `_all` and never reads it, so the lint always audits every Rust file. The help text suggests that a run without the flag audits fewer files.
- **Impact if no action:** A reader expects a smaller audit without `--all`. The lint stays correct, because it always checks every file.
- **Root question:** What does the file-length lint audit, and how does its interface show it?
- **Options:**
  - **A.** Keep the flag and the behaviour. Change the help text and the two READMEs to say that the lint always audits every file and that `--all` has no effect. Effort: Small.
  - **B.** Remove the flag. Update the CI workflow, the two READMEs, and the parser test. Effort: Small.
  - **C.** Make a run without `--all` audit only the Rust files that differ from `origin/main`. Effort: Medium.
- **Left out:** None.
- **Wider fix:** None. The class is one unused parameter, and the compiler does not warn about a parameter with a leading underscore.
- **Previous examples:** None found.
- **Recommendation:** B. A flag without an effect only misleads, and the lint is simpler without it.
- **Action:** ask the user, because option B removes a command-line option, and a command interface on `main` is functionality.
