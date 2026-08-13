//! OpenAI video client lifecycle tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use ai_interface::{
    VideoGenerationAspect, VideoGenerationError, VideoGenerationRequest, VideoGenerator,
};
use ai_models_core::{PollingRuntime, PollingRuntimeMock};
use json_http::{
    JsonHttpBody, JsonHttpMethod, JsonHttpRequest, JsonHttpResponse, JsonHttpTransport,
    JsonHttpTransportMock, StaticHeaderAuth, TransportBackedJsonHttpClient,
};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use super::super::client::OpenAiVideoGenerator;

type JsonResult = std::result::Result<JsonHttpResponse<Value>, json_http::Error>;
type BytesResult = std::result::Result<JsonHttpResponse<Vec<u8>>, json_http::Error>;
type JsonRecorder = dyn Fn(&Unimock, &JsonHttpRequest) -> JsonResult + Send + Sync;
type BytesRecorder = dyn Fn(&Unimock, &JsonHttpRequest) -> BytesResult + Send + Sync;

#[tokio::test]
async fn submits_polls_and_downloads_one_authenticated_video() {
    let (client, requests) = scripted_client(
        vec![
            ok_json(json!({"id": "video_1", "status": "queued"})),
            ok_json(json!({"id": "video_1", "status": "in_progress"})),
            ok_json(json!({"id": "video_1", "status": "completed"})),
        ],
        vec![Ok(JsonHttpResponse {
            status: 200,
            body: mp4(),
        })],
    );
    let (runtime, sleeps) = advancing_runtime();
    let generator = OpenAiVideoGenerator::with_auth_and_runtime(
        client,
        "sora-2",
        Arc::new(StaticHeaderAuth::bearer_token("sk-test")),
        runtime,
    )
    .with_endpoint("https://openai.test/v1/videos")
    .with_polling(Duration::from_secs(2), Duration::from_secs(20));

    let response = generator
        .generate(&VideoGenerationRequest {
            prompt: "A bird takes flight".to_owned(),
            aspect: VideoGenerationAspect::Portrait,
            ..VideoGenerationRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.video.data, mp4());
    assert_eq!((response.video.width, response.video.height), (720, 1280));
    assert_eq!(
        *sleeps.lock().expect("sleep lock should be available"),
        vec![Duration::from_secs(2); 2]
    );
    let requests = requests.lock().expect("request lock should be available");
    assert_eq!(requests.len(), 4);
    assert_eq!(requests[0].method, JsonHttpMethod::Post);
    assert_eq!(requests[1].url, "https://openai.test/v1/videos/video_1");
    assert_eq!(
        requests[3].url,
        "https://openai.test/v1/videos/video_1/content"
    );
    assert!(matches!(requests[0].body, Some(JsonHttpBody::Json(_))));
    assert!(requests.iter().all(|request| {
        request.headers.get("Authorization").map(String::as_str) == Some("Bearer sk-test")
            && request.timeout <= Duration::from_secs(20)
    }));
}

#[tokio::test]
async fn deadline_stops_pending_jobs_and_empty_download_is_no_video() {
    let (client, _) = scripted_client(
        vec![
            ok_json(json!({"id": "video_1", "status": "queued"})),
            ok_json(json!({"id": "video_1", "status": "queued"})),
        ],
        Vec::new(),
    );
    let (runtime, _) = advancing_runtime();
    let error = generator(client, runtime)
        .with_polling(Duration::from_secs(3), Duration::from_secs(3))
        .generate(&request())
        .await
        .unwrap_err();
    assert!(matches!(error, VideoGenerationError::TimedOut { .. }));

    let (client, _) = scripted_client(
        vec![ok_json(json!({"id": "video_1", "status": "completed"}))],
        vec![Ok(JsonHttpResponse {
            status: 200,
            body: Vec::new(),
        })],
    );
    let error = generator(client, stationary_runtime())
        .generate(&request())
        .await
        .unwrap_err();
    assert!(matches!(error, VideoGenerationError::NoVideo { .. }));
}

#[tokio::test]
async fn initial_completed_job_downloads_without_polling() {
    let (client, requests) = scripted_client(
        vec![ok_json(json!({"id": "video_1", "status": "completed"}))],
        vec![Ok(JsonHttpResponse {
            status: 200,
            body: mp4(),
        })],
    );

    let response = generator(client, stationary_runtime())
        .generate(&request())
        .await
        .unwrap();

    assert_eq!(response.video.data, mp4());
    assert_eq!(requests.lock().expect("request lock").len(), 2);
}

fn generator(
    client: Arc<dyn json_http::JsonHttpClient>,
    runtime: Arc<dyn PollingRuntime>,
) -> OpenAiVideoGenerator {
    OpenAiVideoGenerator::with_auth_and_runtime(
        client,
        "sora-2",
        Arc::new(StaticHeaderAuth::bearer_token("key")),
        runtime,
    )
}

fn scripted_client(
    json_responses: Vec<JsonResult>,
    byte_responses: Vec<BytesResult>,
) -> (
    Arc<dyn json_http::JsonHttpClient>,
    Arc<Mutex<Vec<JsonHttpRequest>>>,
) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let json_responses = Arc::new(Mutex::new(VecDeque::from(json_responses)));
    let transport: Arc<dyn JsonHttpTransport> = if byte_responses.is_empty() {
        Arc::new(Unimock::new(
            JsonHttpTransportMock::execute
                .each_call(matching!(_))
                .answers_arc(record_json(requests.clone(), json_responses)),
        ))
    } else {
        let byte_responses = Arc::new(Mutex::new(VecDeque::from(byte_responses)));
        Arc::new(Unimock::new((
            JsonHttpTransportMock::execute
                .each_call(matching!(_))
                .answers_arc(record_json(requests.clone(), json_responses)),
            JsonHttpTransportMock::execute_bytes
                .each_call(matching!(_))
                .answers_arc(record_bytes(requests.clone(), byte_responses)),
        )))
    };
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}

fn record_json(
    requests: Arc<Mutex<Vec<JsonHttpRequest>>>,
    responses: Arc<Mutex<VecDeque<JsonResult>>>,
) -> Arc<JsonRecorder> {
    Arc::new(move |_, request| {
        requests.lock().expect("request lock").push(request.clone());
        responses
            .lock()
            .expect("response lock")
            .pop_front()
            .expect("JSON call")
    })
}

fn record_bytes(
    requests: Arc<Mutex<Vec<JsonHttpRequest>>>,
    responses: Arc<Mutex<VecDeque<BytesResult>>>,
) -> Arc<BytesRecorder> {
    Arc::new(move |_, request| {
        requests.lock().expect("request lock").push(request.clone());
        responses
            .lock()
            .expect("response lock")
            .pop_front()
            .expect("byte call")
    })
}

fn advancing_runtime() -> (Arc<dyn PollingRuntime>, Arc<Mutex<Vec<Duration>>>) {
    let started_at = Instant::now();
    let elapsed = Arc::new(Mutex::new(Duration::ZERO));
    let sleeps = Arc::new(Mutex::new(Vec::new()));
    let runtime = Arc::new(Unimock::new((
        PollingRuntimeMock::now.each_call(matching!()).answers_arc({
            let elapsed = elapsed.clone();
            Arc::new(move |_| started_at + *elapsed.lock().expect("elapsed lock"))
        }),
        PollingRuntimeMock::sleep
            .each_call(matching!(_))
            .answers_arc({
                let elapsed = elapsed.clone();
                let sleeps = sleeps.clone();
                Arc::new(move |_, duration| {
                    sleeps.lock().expect("sleep lock").push(duration);
                    *elapsed.lock().expect("elapsed lock") += duration;
                })
            }),
    ))) as Arc<dyn PollingRuntime>;
    (runtime, sleeps)
}

fn stationary_runtime() -> Arc<dyn PollingRuntime> {
    let now = Instant::now();
    Arc::new(Unimock::new(
        PollingRuntimeMock::now.each_call(matching!()).returns(now),
    ))
}

fn ok_json(body: Value) -> JsonResult {
    Ok(JsonHttpResponse { status: 200, body })
}

fn request() -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "A bird takes flight".to_owned(),
        ..VideoGenerationRequest::default()
    }
}

fn mp4() -> Vec<u8> {
    b"\0\0\0\x18ftypisom\0\0\0\0isommp42".to_vec()
}
