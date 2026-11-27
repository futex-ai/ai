# Restore the QwenCloud live job

- **Severity:** Medium. **Effort:** Small. **Confidence:** High. **Harder later:** No.
- **Kind:** Post-merge task.
- **Added:** 2026-11-27, from branch `ci-fix/live-model-apis-qwencloud-catalog`.
- **Location:** `.github/workflows/live-models.yml:52`, `docs/protocol/live-model-api-tests.md:141`, `docs/protocol/live-model-api-tests.md:152`, `docs/protocol/live-model-api-tests.md:169`
- **Reason:** The job can come back only after an account with paid QwenCloud access exists, and on 2026-11-27 the user chose to leave the job out until then.

- **Terms:**
  - **Live model APIs workflow:** `.github/workflows/live-models.yml`. It runs one matrix job for each listed chat provider, named `<provider> catalog`. It runs for same-repository pull requests that target `main`, every day at 05:17 UTC, and on manual dispatch from `main`. Each job runs one ignored test of `xtask/tests/live_models.rs`, with the Actions secret of its provider as `LIVE_MODEL_API_KEY`. The calls are billable.
  - **QwenCloud:** the provider of the Qwen chat models. Its adapter is in `crates/ai-models-compat/src/qwen/`. Its catalog lists `qwen3.7-max`, `qwen3.7-plus`, and `qwen3.7-flash`, in that order.
  - **`RejectedKey`:** the `ErrorKind` in `crates/ai-interface/src/error_kind.rs` for a provider that rejects the key or the account.
- **Background:**
  - The `QwenCloud catalog` job failed on every run from 2026-11-26 or earlier. Runs 45945535982 (`main`, 2026-11-27) and 45982315095 (pull request #31) failed with this text:
    `RejectedKey for qwen model qwen3.7-max: HTTP 403: The free quota has been exhausted. To continue accessing the model on a paid basis, please complete your payment information (or disable the "use free tier only" mode in the management console if already completed).`
  - Nobody reran the job, because the free quota of the CI account is used up. The adapter maps the 403 to `RejectedKey` as designed, so the adapter has no bug.
  - On 2026-11-27 the user chose to remove the QwenCloud job from the workflow until an account with paid access exists. The user rejected two other options: turn on paid access for the account of the `QWEN_API_KEY` secret, or replace that secret with the key of an account that has paid access.
  - These parts stay: the Qwen adapter, its catalog, its offline tests, and the ignored live test `qwen_catalog` in `xtask/tests/live_models.rs`. A developer can run that test by hand. The call is billable:
    `LIVE_MODEL_API_KEY="$QWEN_API_KEY" cargo test --locked -p xtask --test live_models qwen_catalog -- --ignored --exact --nocapture`
  - The `QWEN_API_KEY` Actions secret stays, because `.github/workflows/record-provider-streams.yml` uses it. That workflow runs only on manual dispatch from `main`, or when a same-repository pull request gets the label `record-provider-streams`. It sends QwenCloud requests, but it is not a check: `cargo xtask record-streams` prints the number of scenarios that returned an error and then exits with success (`xtask/src/record_streams/command.rs:44-50`).
- **Task:**
  1. Get paid access in one of two ways. Then check it with the command above.
     - In the QwenCloud management console, for the account of the `QWEN_API_KEY` secret, complete the payment information, or turn off the "use free tier only" mode.
     - Replace the `QWEN_API_KEY` Actions secret with the key of an account that has paid access.
  2. In `.github/workflows/live-models.yml`, add this matrix row back between the OpenAI row and the xAI row, at the indentation of the other rows:

     ```yaml
     - provider: QwenCloud
       test: qwen_catalog
       api_key: QWEN_API_KEY
     ```

  3. Restore the docs that say CI leaves out QwenCloud:
     - `docs/protocol/live-model-api-tests.md`, section "CI And Credentials": change "one matrix job per provider except QwenCloud" to "one matrix job per provider". Remove the paragraph that starts "Decision: the workflow leaves out QwenCloud". Add the row for QwenCloud with the secret `QWEN_API_KEY` to the secrets table, between OpenAI and xAI. The other docs link this section for the providers that CI runs, so they need no change.
     - Search `docs/`, `plans/`, every `README.md`, and the code comments for `QwenCloud` and for the file name `restore-qwencloud-live-job.md`. Restore each other place that says CI leaves out QwenCloud, or that links this item.
  4. Confirm one passing `QwenCloud catalog` job. The pull request of the restore branch runs the job, because it is a same-repository pull request. After the merge, a manual dispatch from `main` runs it too: `gh workflow run live-models.yml --ref main`. A manual dispatch from any other branch skips every job of the workflow.
- **Impact if no action:** No scheduled or pull-request job runs the QwenCloud catalog. A break at QwenCloud shows only in a manual, billable run of `qwen_catalog` with a key that has paid access.
- **Needs:** An account with paid QwenCloud access. The payment step needs the owner of the QwenCloud account, and the secret step needs permission to edit the repository Actions secrets.
- **Done when:** The workflow has the QwenCloud row again, the docs of step 3 no longer say that CI leaves out QwenCloud, and one `QwenCloud catalog` job passed.
- **Related:**
  - `plans/follow-ups/live-provider-recordings.md` runs `.github/workflows/record-provider-streams.yml`. That workflow records Qwen with the same `QWEN_API_KEY` secret and the first Qwen spec, `qwen3.7-max`, which is the model in the failure text. Its Qwen run needs the paid access of step 1 too.
  - `plans/follow-ups/live-workflow-matrix-sync.md` decides how the live workflow matrices stay in step with the live tests. Its answer can change step 2. Under its option A, step 2 stays as it is. Under B, the restore removes `qwen_catalog` from a skip list. Under C, the restore changes the QwenCloud entry of one provider table and runs the generator.
  - No credential-free test checks the matrix rows of the workflow, because the `testing` skill gives workflow matrix values no test. So the restore changes no test. `plans/follow-ups/live-workflow-yaml-text-tests.md` covers the YAML text tests that remain, for the triggers and the secret conditions. The restore changes neither.
  - `plans/follow-ups/provider-protocols-repeat-live-ci-coverage.md` has the same cause: the removal of this job made the copy of the CI coverage in the Qwen protocol false. That item removes the same copy from the DeepSeek, Kimi, and MiniMax protocols. The two tasks do not depend on each other.
