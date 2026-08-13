//! Ignored live catalog tests for registered image providers.

use super::{provider_tests::LiveImageProvider, runner_tests::run_catalog};

#[tokio::test]
#[ignore = "requires a live Google API credential"]
async fn google_image_catalog() {
    run_catalog(LiveImageProvider::Google).await;
}

#[tokio::test]
#[ignore = "requires a live OpenAI API credential"]
async fn openai_image_catalog() {
    run_catalog(LiveImageProvider::OpenAi).await;
}
