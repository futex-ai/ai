//! Credentialed live-video workflow guard tests.

use super::provider_tests::LiveVideoProvider;

const WORKFLOW: &str = include_str!("../../../.github/workflows/live-videos.yml");
const ORDINARY_CI: &str = include_str!("../../../.github/workflows/ci.yml");

#[test]
fn workflow_covers_every_registered_provider() {
    for provider in LiveVideoProvider::ALL {
        assert!(WORKFLOW.contains(&format!("test: {}", provider.workflow_test())));
        assert!(WORKFLOW.contains(&format!("api_key: {}", provider.workflow_secret())));
    }
}

#[test]
fn workflow_is_trusted_bounded_and_artifact_free() {
    assert!(WORKFLOW.contains("  pull_request:\n    branches:\n      - main"));
    assert!(WORKFLOW.contains("  schedule:"));
    assert!(WORKFLOW.contains("  workflow_dispatch:"));
    assert!(
        WORKFLOW.contains("github.event.pull_request.head.repo.full_name == github.repository")
    );
    assert!(WORKFLOW.contains("github.event.pull_request.user.login != 'dependabot[bot]'"));
    assert!(WORKFLOW.contains("permissions:\n  contents: read"));
    assert!(WORKFLOW.contains("max-parallel: 1"));
    assert!(WORKFLOW.contains("timeout-minutes: 30"));
    assert!(WORKFLOW.contains("github.ref_name == github.event.repository.default_branch"));
    assert!(WORKFLOW.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"));
    assert!(!WORKFLOW.contains("pull_request_target"));
    assert!(!WORKFLOW.contains("upload-artifact"));
}

#[test]
fn workflow_runs_exact_ignored_test_and_ordinary_ci_is_credential_free() {
    assert!(WORKFLOW.contains(
        "cargo test --locked -p xtask --test live_videos \"${{ matrix.test }}\" -- --ignored --exact --nocapture"
    ));
    assert!(!ORDINARY_CI.contains("LIVE_VIDEO_API_KEY"));
    assert!(!ORDINARY_CI.contains("--test live_videos"));
    assert!(!ORDINARY_CI.contains("secrets."));
}

#[test]
fn workflow_exposes_only_the_current_secret_and_checks_it_before_checkout() {
    let credential = "LIVE_VIDEO_API_KEY: ${{ secrets[matrix.api_key] }}";
    assert_eq!(WORKFLOW.matches(credential).count(), 2);
    assert!(WORKFLOW.contains("if [[ -z \"$LIVE_VIDEO_API_KEY\" ]]; then"));
    assert!(WORKFLOW.contains("exit 1"));
    assert!(
        WORKFLOW.find("- name: Require provider credential") < WORKFLOW.find("- name: Checkout")
    );
}
