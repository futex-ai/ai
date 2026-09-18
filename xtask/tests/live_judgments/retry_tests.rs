//! Judgment retry behavior tests.

use std::{
    collections::{BTreeMap, VecDeque},
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use ai_interface::{
    DynJudgmentModel, JudgmentAnswer, JudgmentAnswerProblem, JudgmentContent, JudgmentError,
    JudgmentJsonType, JudgmentModel, JudgmentModelMock, JudgmentQuestion, JudgmentQuestionProblem,
    JudgmentRequest, JudgmentResponse, JudgmentResult, ModelUsage,
};
use ai_models_core::{DynSleeper, STANDARD_TRANSIENT_RETRY_DELAYS, TokioSleeper};
use async_trait::async_trait;
use unimock::{MockFn, Unimock, matching};

#[derive(Clone)]
pub(super) struct RetryingJudgmentModel {
    inner: DynJudgmentModel,
    sleeper: DynSleeper,
    retry_delays: Vec<Duration>,
}

impl RetryingJudgmentModel {
    pub(super) fn new(
        inner: DynJudgmentModel,
        sleeper: DynSleeper,
        retry_delays: Vec<Duration>,
    ) -> Self {
        Self {
            inner,
            sleeper,
            retry_delays,
        }
    }

    pub(super) fn with_standard_transient_retry(inner: DynJudgmentModel) -> Self {
        Self::new(
            inner,
            Arc::new(TokioSleeper),
            Self::standard_retry_delays().to_vec(),
        )
    }

    pub(super) fn standard_retry_delays() -> [Duration; 2] {
        STANDARD_TRANSIENT_RETRY_DELAYS
    }
}

#[async_trait]
impl JudgmentModel for RetryingJudgmentModel {
    async fn judge(&self, request: &JudgmentRequest) -> JudgmentResult<JudgmentResponse> {
        let mut retry_index = 0usize;

        loop {
            match self.inner.judge(request).await {
                Err(
                    JudgmentError::RateLimited { .. } | JudgmentError::TransientProvider { .. },
                ) if retry_index < self.retry_delays.len() => {
                    let delay = self.retry_delays[retry_index];
                    retry_index += 1;
                    self.sleeper.sleep(delay).await;
                }
                result => return result,
            }
        }
    }
}

#[tokio::test]
async fn retries_only_rate_limits_and_transient_provider_failures() {
    let calls = Arc::new(AtomicUsize::new(0));
    let retrying = RetryingJudgmentModel::new(
        scripted_model(
            vec![
                Err(JudgmentError::rate_limited(
                    "typesafe",
                    "jev-latest",
                    "retry one",
                )),
                Err(JudgmentError::transient_provider(
                    "typesafe",
                    "jev-latest",
                    "retry two",
                )),
                Ok(success_response()),
            ],
            calls.clone(),
        ),
        Arc::new(TokioSleeper),
        vec![Duration::ZERO, Duration::ZERO],
    );

    retrying
        .judge(&probe_request())
        .await
        .expect("third attempt should succeed");

    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn returns_the_last_retryable_error_after_three_attempts() {
    let calls = Arc::new(AtomicUsize::new(0));
    let retrying = RetryingJudgmentModel::new(
        scripted_model(
            vec![transient("one"), transient("two"), transient("three")],
            calls.clone(),
        ),
        Arc::new(TokioSleeper),
        vec![Duration::ZERO, Duration::ZERO],
    );

    let error = retrying
        .judge(&probe_request())
        .await
        .expect_err("third transient failure should be returned");

    assert!(matches!(error, JudgmentError::TransientProvider { .. }));
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}

#[tokio::test]
async fn terminal_error_classes_stop_after_one_attempt() {
    let errors = vec![
        JudgmentError::UnsupportedContent {
            json_type: JudgmentJsonType::Null,
        },
        JudgmentError::EmptyState,
        JudgmentError::NoQuestions,
        JudgmentError::invalid_question("urgent", JudgmentQuestionProblem::NoOptions),
        JudgmentError::invalid_answer(
            "typesafe",
            "jev-latest",
            "urgent",
            JudgmentAnswerProblem::Missing,
        ),
        JudgmentError::provider("typesafe", "jev-latest", "rejected"),
        JudgmentError::internal(std::io::Error::other("internal")),
    ];

    for error in errors {
        let calls = Arc::new(AtomicUsize::new(0));
        let retrying = RetryingJudgmentModel::new(
            scripted_model(vec![Err(error)], calls.clone()),
            Arc::new(TokioSleeper),
            vec![Duration::ZERO, Duration::ZERO],
        );

        retrying
            .judge(&probe_request())
            .await
            .expect_err("terminal error should be returned");
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }
}

#[test]
fn standard_retry_schedule_is_the_shared_bounded_schedule() {
    assert_eq!(
        RetryingJudgmentModel::standard_retry_delays(),
        STANDARD_TRANSIENT_RETRY_DELAYS
    );
}

fn scripted_model(
    responses: Vec<JudgmentResult<JudgmentResponse>>,
    calls: Arc<AtomicUsize>,
) -> DynJudgmentModel {
    let responses = Arc::new(Mutex::new(VecDeque::from(responses)));
    Arc::new(Unimock::new(
        JudgmentModelMock::judge
            .each_call(matching!(_))
            .answers_arc({
                let responses = responses.clone();
                Arc::new(move |_, _request: &JudgmentRequest| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    responses
                        .lock()
                        .expect("response lock should not be poisoned")
                        .pop_front()
                        .expect("unexpected judgment call")
                })
            }),
    ))
}

fn transient(message: &str) -> JudgmentResult<JudgmentResponse> {
    Err(JudgmentError::transient_provider(
        "typesafe",
        "jev-latest",
        message,
    ))
}

fn probe_request() -> JudgmentRequest {
    JudgmentRequest {
        state: JudgmentContent::from("A support message"),
        questions: BTreeMap::from([(
            "urgent".to_owned(),
            JudgmentQuestion::condition("Is it urgent?", None),
        )]),
    }
}

fn success_response() -> JudgmentResponse {
    JudgmentResponse {
        provider: "typesafe".to_owned(),
        model_id: "jev-latest".to_owned(),
        resolved_model_id: "jev-1.13.0".to_owned(),
        answers: BTreeMap::from([(
            "urgent".to_owned(),
            JudgmentAnswer::Condition { probability: 1.0 },
        )]),
        usage: ModelUsage::default(),
    }
}
