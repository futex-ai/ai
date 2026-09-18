//! Catalog iteration, credential handling, and probe request tests.

use std::{
    collections::BTreeMap,
    env,
    sync::{Arc, Mutex},
};

use ai_interface::{
    DynJudgmentModel, JudgmentContent, JudgmentError, JudgmentModel, JudgmentModelMock,
    JudgmentQuestion, JudgmentRequest, MockJudgmentModel, ProviderKind,
};
use ai_models_core::{
    CostTier, IntelligenceScore, KnownModelSpec, ModelFeature, SpeedTier, ThinkingLevel,
};
use json_http::{JsonHttpClient, ReqwestJsonHttpClient};
use serde_json::{Map, Value};
use thiserror::Error;
use unimock::{MockFn, Unimock, matching};

use super::{
    provider_tests::LiveJudgmentProvider, retry_tests::RetryingJudgmentModel,
    validation_tests::validation_failures,
};

const JUDGMENT_FEATURES: &[ModelFeature] = &[ModelFeature::Judgment];
const API_KEY_ENV: &str = "LIVE_JUDGMENT_API_KEY";

#[derive(Debug, Eq, Error, PartialEq)]
enum ApiKeyError {
    #[error("[xtask/live_judgments] LIVE_JUDGMENT_API_KEY is missing")]
    Missing,
    #[error("[xtask/live_judgments] LIVE_JUDGMENT_API_KEY is blank")]
    Blank,
}

struct ProbeTarget {
    spec: KnownModelSpec,
    model: DynJudgmentModel,
}

pub(super) async fn run_catalog(provider: LiveJudgmentProvider) {
    let api_key =
        require_api_key(env::var(API_KEY_ENV).ok()).unwrap_or_else(|error| panic!("{error}"));
    let client: Arc<dyn JsonHttpClient> = Arc::new(ReqwestJsonHttpClient::new());
    let auth = provider.auth(api_key);
    let catalog = provider.judgment_catalog();
    assert!(
        !catalog.is_empty(),
        "{provider:?} judgment catalog must not be empty"
    );
    let targets = catalog
        .into_iter()
        .map(|spec| {
            let inner = provider.build(client.clone(), auth.clone(), &spec);
            ProbeTarget {
                spec,
                model: Arc::new(RetryingJudgmentModel::with_standard_transient_retry(inner)),
            }
        })
        .collect();
    let failures = run_targets(targets).await;

    assert!(
        failures.is_empty(),
        "{} live judgment catalog failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

fn require_api_key(value: Option<String>) -> Result<String, ApiKeyError> {
    let value = value.ok_or(ApiKeyError::Missing)?;
    if value.trim().is_empty() {
        return Err(ApiKeyError::Blank);
    }
    Ok(value)
}

fn probe_request() -> JudgmentRequest {
    JudgmentRequest {
        state: JudgmentContent::Object(Map::from_iter([(
            "message".to_owned(),
            Value::String("Please help; my payment did not go through.".to_owned()),
        )])),
        questions: BTreeMap::from([
            (
                "urgent".to_owned(),
                JudgmentQuestion::condition("Does the message express urgency?", None),
            ),
            (
                "queue".to_owned(),
                JudgmentQuestion::choice(
                    "Which support queue best fits the message?",
                    [("billing", None), ("technical", None), ("other", None)],
                ),
            ),
            (
                "frustration".to_owned(),
                JudgmentQuestion::score("How frustrated is the customer?", [None, None, None]),
            ),
        ]),
    }
}

async fn run_targets(targets: Vec<ProbeTarget>) -> Vec<String> {
    let request = probe_request();
    let mut failures = Vec::new();

    for target in targets {
        println!("{}", check_message(&target.spec));
        match target.model.judge(&request).await {
            Ok(response) => {
                failures.extend(validation_failures(&target.spec, &request, &response));
            }
            Err(error) => failures.push(request_failure_message(&target.spec, &error)),
        }
    }

    failures
}

fn check_message(spec: &KnownModelSpec) -> String {
    format!("checking {}/{}", spec.provider, spec.id)
}

fn request_failure_message(spec: &KnownModelSpec, error: &JudgmentError) -> String {
    format!("{}: request failed: {error}", spec.id)
}

#[test]
fn requires_a_present_non_empty_api_key() {
    assert!(matches!(require_api_key(None), Err(ApiKeyError::Missing)));
    assert!(matches!(
        require_api_key(Some(" \n\t".to_owned())),
        Err(ApiKeyError::Blank)
    ));
    assert_eq!(
        require_api_key(Some(" secret ".to_owned())).unwrap(),
        " secret "
    );
}

#[test]
fn probe_request_has_the_documented_provider_neutral_shape() {
    let request = probe_request();
    let JudgmentContent::Object(state) = request.state else {
        panic!("probe state must be an object");
    };

    assert_eq!(state.len(), 1);
    assert_eq!(
        state.get("message"),
        Some(&Value::String(
            "Please help; my payment did not go through.".to_owned()
        ))
    );
    assert_eq!(
        request
            .questions
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["frustration", "queue", "urgent"]
    );
    assert!(matches!(
        request.questions.get("urgent"),
        Some(JudgmentQuestion::Condition { .. })
    ));
    assert!(matches!(
        request.questions.get("queue"),
        Some(JudgmentQuestion::Choice { options, .. })
            if options.keys().map(String::as_str).collect::<Vec<_>>()
                == vec!["billing", "other", "technical"]
    ));
    assert!(matches!(
        request.questions.get("frustration"),
        Some(JudgmentQuestion::Score { levels, .. }) if levels.len() == 3
    ));
}

#[tokio::test]
async fn runs_targets_sequentially_and_aggregates_every_failure() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let targets = vec![
        target("first", recording_failure_model("first", calls.clone())),
        target("second", recording_failure_model("second", calls.clone())),
    ];

    let failures = run_targets(targets).await;

    assert_eq!(
        *calls.lock().expect("call lock should not be poisoned"),
        vec!["first", "second"]
    );
    assert_eq!(failures.len(), 2);
    assert!(failures.iter().any(|failure| failure.contains("first")));
    assert!(failures.iter().any(|failure| failure.contains("second")));
}

#[tokio::test]
async fn runner_accepts_a_provider_neutral_dynamic_model() {
    let model = mock_model_with_non_zero_usage().await;
    let failures = run_targets(vec![ProbeTarget {
        spec: mock_spec(),
        model,
    }])
    .await;

    assert!(failures.is_empty());
}

#[test]
fn log_messages_contain_identifiers_but_not_sensitive_values() {
    let spec = judgment_spec(ProviderKind::TypeSafe, "catalog-id", "jev-latest");
    let check = check_message(&spec);
    let failure = request_failure_message(
        &spec,
        &JudgmentError::provider("typesafe", "jev-latest", "rejected"),
    );

    assert_eq!(check, "checking typesafe/catalog-id");
    assert!(failure.contains("catalog-id"));
    assert!(failure.contains("jev-latest"));
    assert!(!check.contains("secret-key"));
    assert!(!failure.contains("secret-key"));
}

fn recording_failure_model(
    label: &'static str,
    calls: Arc<Mutex<Vec<&'static str>>>,
) -> DynJudgmentModel {
    Arc::new(Unimock::new(
        JudgmentModelMock::judge
            .next_call(matching!(_))
            .answers_arc(Arc::new(move |_, _request: &JudgmentRequest| {
                calls
                    .lock()
                    .expect("call lock should not be poisoned")
                    .push(label);
                Err(JudgmentError::provider(
                    "typesafe",
                    label,
                    "expected failure",
                ))
            })),
    ))
}

async fn mock_model_with_non_zero_usage() -> DynJudgmentModel {
    let mut response = MockJudgmentModel
        .judge(&probe_request())
        .await
        .expect("built-in judgment mock should answer the probe");
    response.provider = "typesafe".to_owned();
    response.usage.input_tokens = 1;
    response.usage.total_tokens = 1;
    Arc::new(Unimock::new(
        JudgmentModelMock::judge
            .next_call(matching!(_))
            .returns(Ok(response)),
    ))
}

fn target(id: &'static str, model: DynJudgmentModel) -> ProbeTarget {
    ProbeTarget {
        spec: judgment_spec(ProviderKind::TypeSafe, id, id),
        model,
    }
}

fn judgment_spec(
    provider: ProviderKind,
    id: &'static str,
    provider_model_id: &'static str,
) -> KnownModelSpec {
    KnownModelSpec {
        provider,
        id,
        provider_model_id,
        context_window_tokens: 0,
        intelligence_score: IntelligenceScore::Five,
        speed: SpeedTier::VeryFast,
        cost: CostTier::Low,
        thinking_level: ThinkingLevel::Disabled,
        features: JUDGMENT_FEATURES,
    }
}

fn mock_spec() -> KnownModelSpec {
    judgment_spec(ProviderKind::TypeSafe, "mock-judgment", "mock-judgment")
}
