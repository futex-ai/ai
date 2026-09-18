//! Credentialed live-judgment workflow guard tests.

use super::provider_tests::LiveJudgmentProvider;

const WORKFLOW: &str = include_str!("../../../.github/workflows/live-judgments.yml");
const ORDINARY_CI: &str = include_str!("../../../.github/workflows/ci.yml");

#[test]
fn workflow_covers_every_registered_provider() {
    for provider in LiveJudgmentProvider::ALL {
        assert!(
            WORKFLOW.contains(&format!("test: {}", provider.workflow_test())),
            "workflow omitted the {} test",
            provider.kind()
        );
        assert!(
            WORKFLOW.contains(&format!("api_key: {}", provider.workflow_secret())),
            "workflow omitted the {} credential",
            provider.kind()
        );
    }
}

#[test]
fn workflow_has_trusted_events_and_read_only_permissions() {
    assert!(WORKFLOW.contains("  pull_request:\n    branches:\n      - main"));
    assert!(WORKFLOW.contains("  schedule:"));
    assert!(WORKFLOW.contains("  workflow_dispatch:"));
    assert!(WORKFLOW.contains("permissions:\n  contents: read"));
    assert!(!WORKFLOW.contains("pull_request_target"));
    assert!(
        WORKFLOW.contains("github.event.pull_request.head.repo.full_name == github.repository")
    );
    assert!(WORKFLOW.contains("github.event.pull_request.user.login != 'dependabot[bot]'"));
    assert!(WORKFLOW.contains("github.ref_name == github.event.repository.default_branch"));
}

#[test]
fn workflow_bounds_parallelism_duration_and_superseded_runs() {
    assert!(WORKFLOW.contains("fail-fast: false"));
    assert!(WORKFLOW.contains("max-parallel: 1"));
    assert!(WORKFLOW.contains("timeout-minutes: 20"));
    assert!(WORKFLOW.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"));
}

#[test]
fn workflow_requires_only_the_current_provider_secret_before_checkout() {
    let credential = "LIVE_JUDGMENT_API_KEY: ${{ secrets[matrix.api_key] }}";
    assert_eq!(WORKFLOW.matches(credential).count(), 2);
    assert!(WORKFLOW.contains("if [[ -z \"$LIVE_JUDGMENT_API_KEY\" ]]; then"));
    assert!(WORKFLOW.contains("exit 1"));
    assert!(
        WORKFLOW.find("- name: Require provider credential") < WORKFLOW.find("- name: Checkout")
    );
}

#[test]
fn workflow_runs_the_exact_ignored_catalog_test_without_artifacts() {
    assert!(WORKFLOW.contains(
        "cargo test --locked -p xtask --test live_judgments \"${{ matrix.test }}\" -- --ignored --exact --nocapture"
    ));
    assert!(!WORKFLOW.contains("upload-artifact"));
    assert!(!WORKFLOW.contains("judgment response"));
}

#[test]
fn ordinary_ci_remains_credential_free() {
    assert!(!ORDINARY_CI.contains("LIVE_JUDGMENT_API_KEY"));
    assert!(!ORDINARY_CI.contains("--test live_judgments"));
    assert!(!ORDINARY_CI.contains("TYPESAFE_API_KEY"));
    assert!(!ORDINARY_CI.contains("secrets."));
}
