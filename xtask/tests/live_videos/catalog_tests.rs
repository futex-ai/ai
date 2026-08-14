//! Ignored live catalog tests for registered video providers.

use super::{provider_tests::LiveVideoProvider, runner_tests::run_catalog};

#[tokio::test]
#[ignore = "requires a live Google API credential"]
async fn google_video_catalog() {
    run_catalog(LiveVideoProvider::Google).await;
}

#[tokio::test]
#[ignore = "requires a live OpenAI API credential"]
async fn openai_video_catalog() {
    run_catalog(LiveVideoProvider::OpenAi).await;
}
