//! Ignored live catalog tests for registered judgment providers.

use super::{provider_tests::LiveJudgmentProvider, runner_tests::run_catalog};

#[tokio::test]
#[ignore = "requires a live TypeSafe API credential"]
async fn typesafe_judgment_catalog() {
    run_catalog(LiveJudgmentProvider::TypeSafe).await;
}
