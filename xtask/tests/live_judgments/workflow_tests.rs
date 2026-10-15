//! Credentialed live-judgment workflow guard tests.

use super::provider_tests::LiveJudgmentProvider;

const WORKFLOW: &str = include_str!("../../../.github/workflows/live-judgments.yml");
const ORDINARY_CI: &str = include_str!("../../../.github/workflows/ci.yml");
const CREDENTIAL_GUARD: &str = "if: steps.credential.outputs.present == 'true'";
const GUARDED_STEPS: [&str; 4] = [
    "Checkout",
    "Install Rust",
    "Cache Cargo",
    "Test every judgment catalog model",
];

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
fn workflow_has_trusted_events_no_opt_in_variable_and_read_only_permissions() {
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
    assert!(!WORKFLOW.contains("LIVE_JUDGMENT_API_TESTS_ENABLED"));
}

#[test]
fn workflow_bounds_parallelism_duration_and_superseded_runs() {
    assert!(WORKFLOW.contains("fail-fast: false"));
    assert!(WORKFLOW.contains("max-parallel: 1"));
    assert!(WORKFLOW.contains("timeout-minutes: 20"));
    assert!(WORKFLOW.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"));
}

#[test]
fn workflow_detects_only_the_current_provider_secret_before_checkout() {
    let credential = "LIVE_JUDGMENT_API_KEY: ${{ secrets[matrix.api_key] }}";
    assert_eq!(WORKFLOW.matches("secrets[matrix.api_key]").count(), 2);
    assert_eq!(WORKFLOW.matches(credential).count(), 2);
    assert!(WORKFLOW.contains("- name: Detect provider credential\n        id: credential\n"));
    assert!(WORKFLOW.contains("if [[ -z \"$LIVE_JUDGMENT_API_KEY\" ]]; then"));
    assert!(
        WORKFLOW.find("- name: Detect provider credential") < WORKFLOW.find("- name: Checkout")
    );
}

#[test]
fn workflow_skips_an_entry_without_its_secret_with_a_warning() {
    assert!(WORKFLOW.contains(
        "echo \"::warning::Skipping the ${{ matrix.provider }} judgment catalog: \
         Actions secret ${{ matrix.api_key }} is not set\""
    ));
    assert!(WORKFLOW.contains("echo \"present=false\" >> \"$GITHUB_OUTPUT\""));
    assert!(WORKFLOW.contains("echo \"present=true\" >> \"$GITHUB_OUTPUT\""));
    assert!(!WORKFLOW.contains("exit 1"));
}

#[test]
fn workflow_guards_every_step_after_detection_with_the_credential_output() {
    assert_eq!(
        WORKFLOW.matches(CREDENTIAL_GUARD).count(),
        GUARDED_STEPS.len()
    );
    for step in GUARDED_STEPS {
        assert!(
            step_block(step).contains(CREDENTIAL_GUARD),
            "step `{step}` must run only when the credential is present"
        );
    }
    assert!(!step_block("Detect provider credential").contains(CREDENTIAL_GUARD));
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

/// Returns the workflow text from one named step up to the next step.
fn step_block(name: &str) -> &'static str {
    let marker = format!("- name: {name}\n");
    let start = WORKFLOW
        .find(&marker)
        .unwrap_or_else(|| panic!("workflow is missing step `{name}`"));
    let rest = &WORKFLOW[start + marker.len()..];
    let end = rest.find("- name: ").unwrap_or(rest.len());
    &rest[..end]
}
