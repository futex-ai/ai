//! Credentialed end-to-end checks for every chat-capable catalog model.

#[path = "live_models/event_tests.rs"]
mod event_tests;
#[path = "live_models/marker_retry_tests.rs"]
mod marker_retry_tests;
#[path = "live_models/provider_tests.rs"]
mod provider_tests;
#[path = "live_models/runner_tests.rs"]
mod runner_tests;
#[path = "live_models/tool_choice_tests.rs"]
mod tool_choice_tests;
#[path = "live_models/validation_tests.rs"]
mod validation_tests;

use std::collections::BTreeSet;
use std::sync::Arc;

use ai_interface::ModelFeature;
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};

use self::provider_tests::LiveProvider;
use self::runner_tests::run_live_provider;

#[test]
fn covers_every_real_provider() {
    assert_eq!(LiveProvider::from_id(ai_interface::MOCK_PROVIDER), None);
    assert_eq!(LiveProvider::from_id(ai_models_typesafe::PROVIDER_ID), None);
    let expected = BTreeSet::from([
        ai_models_anthropic::PROVIDER_ID,
        ai_models_compat::deepseek::PROVIDER_ID,
        ai_models_google::PROVIDER_ID,
        ai_models_compat::kimi::PROVIDER_ID,
        ai_models_compat::minimax::PROVIDER_ID,
        ai_models_openai::PROVIDER_ID,
        ai_models_compat::qwen::PROVIDER_ID,
        ai_models_compat::xai::PROVIDER_ID,
    ]);
    let actual = LiveProvider::ALL
        .iter()
        .map(|provider| provider.provider_id())
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);

    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    for provider in LiveProvider::ALL {
        assert_eq!(
            LiveProvider::from_id(provider.provider_id()),
            Some(provider)
        );
        let catalog = provider.catalog();
        assert!(
            !catalog.is_empty(),
            "{provider:?} catalog must not be empty"
        );
        assert!(
            catalog
                .iter()
                .all(|model| model.provider == provider.provider_id()),
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
fn chat_catalog_excludes_specialized_models() {
    let specialized_model_count = LiveProvider::ALL
        .iter()
        .flat_map(|provider| provider.catalog())
        .filter(|model| {
            model.has_feature(ModelFeature::ImageGeneration)
                || model.has_feature(ModelFeature::Judgment)
                || model.has_feature(ModelFeature::VideoGeneration)
        })
        .count();
    assert!(
        specialized_model_count > 0,
        "expected specialized generation catalog coverage"
    );

    for provider in LiveProvider::ALL {
        assert!(
            provider.chat_catalog().iter().all(|model| {
                !model.has_feature(ModelFeature::ImageGeneration)
                    && !model.has_feature(ModelFeature::Judgment)
                    && !model.has_feature(ModelFeature::VideoGeneration)
            }),
            "{provider:?} chat catalog included a specialized generation model"
        );
    }
}

#[test]
fn typesafe_judgment_models_never_reach_other_live_runners() {
    let catalog = ai_models_typesafe::CATALOG.to_vec();

    assert!(!catalog.is_empty(), "TypeSafe catalog must not be empty");
    assert!(catalog.iter().all(|model| {
        model.has_feature(ModelFeature::Judgment)
            && !model.has_feature(ModelFeature::ToolCalling)
            && !model.has_feature(ModelFeature::ImageGeneration)
            && !model.has_feature(ModelFeature::VideoGeneration)
    }));
}

#[test]
fn workflow_covers_every_live_provider() {
    let workflow = include_str!("../../.github/workflows/live-models.yml");

    for provider in LiveProvider::ALL {
        assert!(
            workflow.contains(&format!("test: {}", provider.workflow_test())),
            "workflow omitted the {} test",
            provider.provider_id()
        );
        assert!(
            workflow.contains(&format!("api_key: {}", provider.workflow_secret())),
            "workflow omitted the {} credential",
            provider.provider_id()
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
#[ignore = "requires a live Anthropic API credential for the streaming catalog"]
async fn anthropic_catalog() {
    run_live_provider(LiveProvider::Anthropic).await;
}

#[tokio::test]
#[ignore = "requires a live DeepSeek API credential for the streaming catalog"]
async fn deepseek_catalog() {
    run_live_provider(LiveProvider::DeepSeek).await;
}

#[tokio::test]
#[ignore = "requires a live Google API credential for the streaming catalog"]
async fn google_catalog() {
    run_live_provider(LiveProvider::Google).await;
}

#[tokio::test]
#[ignore = "requires a live Kimi API credential for the streaming catalog"]
async fn kimi_catalog() {
    run_live_provider(LiveProvider::Kimi).await;
}

#[tokio::test]
#[ignore = "requires a live MiniMax API credential for the streaming catalog"]
async fn minimax_catalog() {
    tool_choice_tests::run_minimax_m3_required_tool_call().await;
    run_live_provider(LiveProvider::MiniMax).await;
}

#[tokio::test]
#[ignore = "requires a live OpenAI API credential for the streaming catalog"]
async fn openai_catalog() {
    run_live_provider(LiveProvider::OpenAi).await;
}

#[tokio::test]
#[ignore = "requires a live QwenCloud API credential for the streaming catalog"]
async fn qwen_catalog() {
    run_live_provider(LiveProvider::Qwen).await;
}

#[tokio::test]
#[ignore = "requires a live xAI API credential for the streaming/deferred catalog"]
async fn xai_catalog() {
    run_live_provider(LiveProvider::Xai).await;
}
