//! Credentialed end-to-end checks for every image-capable catalog model.

#[path = "live_images/provider_tests.rs"]
mod provider_tests;
#[path = "live_images/retry_tests.rs"]
mod retry_tests;
#[path = "live_images/runner_tests.rs"]
mod runner_tests;
#[path = "live_images/validation_tests.rs"]
mod validation_tests;
#[path = "live_images/workflow_tests.rs"]
mod workflow_tests;

use self::provider_tests::LiveImageProvider;
use self::runner_tests::run_catalog;

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
