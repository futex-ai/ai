//! Credentialed end-to-end checks for every chat-capable catalog model.

#[path = "live_models/provider_tests.rs"]
mod provider_tests;
#[path = "live_models/runner_tests.rs"]
mod runner_tests;

use std::collections::BTreeSet;
use std::sync::Arc;

use ai_interface::ProviderKind;
use ai_models_core::ModelFeature;
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

use self::provider_tests::LiveProvider;
use self::runner_tests::run_catalog;

#[test]
fn covers_every_real_provider() {
    assert_eq!(LiveProvider::from_kind(ProviderKind::Mock), None);
    let expected = BTreeSet::from([
        ProviderKind::Anthropic,
        ProviderKind::DeepSeek,
        ProviderKind::Google,
        ProviderKind::Kimi,
        ProviderKind::MiniMax,
        ProviderKind::OpenAi,
        ProviderKind::Qwen,
        ProviderKind::Xai,
    ]);
    let actual = LiveProvider::ALL
        .iter()
        .map(|provider| provider.kind())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);

    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    for provider in LiveProvider::ALL {
        assert_eq!(LiveProvider::from_kind(provider.kind()), Some(provider));
        let catalog = provider.catalog();
        assert!(
            !catalog.is_empty(),
            "{provider:?} catalog must not be empty"
        );
        assert!(
            catalog
                .iter()
                .all(|model| model.provider == provider.kind()),
            "{provider:?} catalog contained another provider"
        );
        let chat_catalog = provider.chat_catalog();
        assert!(
            !chat_catalog.is_empty(),
            "{provider:?} chat catalog must not be empty"
        );
        let auth = provider.auth("credential-free-construction-key".to_owned());
        for model in &chat_catalog {
            let _model = provider.build(client.clone(), auth.clone(), model);
        }
    }
}

#[test]
fn chat_catalog_excludes_image_generation_models() {
    let image_model_count = LiveProvider::ALL
        .iter()
        .flat_map(|provider| provider.catalog())
        .filter(|model| model.has_feature(ModelFeature::ImageGeneration))
        .count();
    assert!(
        image_model_count > 0,
        "expected image-only catalog coverage"
    );

    for provider in LiveProvider::ALL {
        assert!(
            provider
                .chat_catalog()
                .iter()
                .all(|model| !model.has_feature(ModelFeature::ImageGeneration)),
            "{provider:?} chat catalog included an image-generation model"
        );
    }
}

#[test]
fn workflow_covers_every_live_provider() {
    let workflow = include_str!("../../.github/workflows/live-models.yml");

    for provider in LiveProvider::ALL {
        assert!(
            workflow.contains(&format!("test: {}", provider.workflow_test())),
            "workflow omitted the {} test",
            provider.kind()
        );
        assert!(
            workflow.contains(&format!("api_key: {}", provider.workflow_secret())),
            "workflow omitted the {} credential",
            provider.kind()
        );
    }
}

#[test]
fn workflow_runs_for_trusted_pull_requests() {
    let workflow = include_str!("../../.github/workflows/live-models.yml");

    assert!(
        workflow.contains("  pull_request:\n    branches:\n      - main"),
        "workflow must run for pull requests targeting main"
    );
    assert!(
        workflow.contains("github.event.pull_request.head.repo.full_name == github.repository"),
        "workflow must restrict credentialed pull-request jobs to repository branches"
    );
    assert!(
        workflow.contains("github.event.pull_request.user.login != 'dependabot[bot]'"),
        "workflow must not expose Actions secrets to Dependabot jobs"
    );
    assert!(
        workflow.contains("github.ref_name == github.event.repository.default_branch"),
        "scheduled and manual jobs must remain restricted to the default branch"
    );
}

#[tokio::test]
#[ignore = "requires a live Anthropic API credential"]
async fn anthropic_catalog() {
    run_catalog(LiveProvider::Anthropic).await;
}

#[tokio::test]
#[ignore = "requires a live DeepSeek API credential"]
async fn deepseek_catalog() {
    run_catalog(LiveProvider::DeepSeek).await;
}

#[tokio::test]
#[ignore = "requires a live Google API credential"]
async fn google_catalog() {
    run_catalog(LiveProvider::Google).await;
}

#[tokio::test]
#[ignore = "requires a live Kimi API credential"]
async fn kimi_catalog() {
    run_catalog(LiveProvider::Kimi).await;
}

#[tokio::test]
#[ignore = "requires a live MiniMax API credential"]
async fn minimax_catalog() {
    run_catalog(LiveProvider::MiniMax).await;
}

#[tokio::test]
#[ignore = "requires a live OpenAI API credential"]
async fn openai_catalog() {
    run_catalog(LiveProvider::OpenAi).await;
}

#[tokio::test]
#[ignore = "requires a live QwenCloud API credential"]
async fn qwen_catalog() {
    run_catalog(LiveProvider::Qwen).await;
}

#[tokio::test]
#[ignore = "requires a live xAI API credential"]
async fn xai_catalog() {
    run_catalog(LiveProvider::Xai).await;
}
