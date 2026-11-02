//! Configured video submission, polling, and authenticated download.

use std::time::Duration;

use ai_interface::{VideoGenerationInputImage, VideoGenerationRequest, VideoGenerator};
use ai_models_core::ProviderConfig;
use json_http::{JsonHttpBody, JsonHttpResponse};
use serde_json::json;

use crate::{OpenAiVideoGenerator, PROVIDER_ID, SORA_2};

use super::support::{
    advancing_runtime, assert_headers, buffered_client, config, json_response, rotating_auth,
};

#[tokio::test]
async fn config_video_defaults_and_overrides_cover_every_phase() {
    for (base, timeout, interval, prefix, expected_timeout, expected_interval, edit) in [
        (None, None, None, "https://api.openai.com", 600, 10, false),
        (
            Some("https://proxy.test/team/"),
            Some(23),
            Some(3),
            "https://proxy.test/team",
            23,
            3,
            true,
        ),
    ] {
        let (http, requests) = client();
        let (runtime, sleeps) = advancing_runtime();
        let mut config = config(base).with_idle_timeout(Duration::from_secs(1));
        config.timeouts.total_timeout = timeout.map(Duration::from_secs);
        config.timeouts.poll_interval = interval.map(Duration::from_secs);
        let generator =
            OpenAiVideoGenerator::from_config(http, SORA_2, config).with_runtime(runtime);
        let response = generator.generate(&request(edit)).await.unwrap();
        assert_eq!(response.model_id, "sora-2");
        assert_eq!(
            *sleeps.lock().unwrap(),
            [Duration::from_secs(expected_interval)]
        );
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].url, format!("{prefix}/v1/videos"));
        assert_eq!(requests[1].url, format!("{prefix}/v1/videos/video_1"));
        assert_eq!(
            requests[2].url,
            format!("{prefix}/v1/videos/video_1/content")
        );
        assert_eq!(requests[0].timeout, Duration::from_secs(expected_timeout));
        for request in &requests[1..] {
            assert_eq!(
                request.timeout,
                Duration::from_secs(expected_timeout - expected_interval)
            );
        }
        for request in requests.iter() {
            assert_eq!(request.idle_timeout, None);
            assert_headers(request, "Bearer config-key");
        }
        assert_eq!(
            matches!(requests[0].body, Some(JsonHttpBody::Multipart(_))),
            edit
        );
    }
}

#[tokio::test]
async fn config_video_hooks_refresh_on_submission_poll_and_download() {
    let (http, requests) = client();
    let (runtime, _) = advancing_runtime();
    let config = ProviderConfig::with_auth(PROVIDER_ID, rotating_auth())
        .with_header("AUTHORIZATION", "wrong")
        .with_header("X-Tenant", "tenant-value");
    let generator = OpenAiVideoGenerator::from_config(http, SORA_2, config).with_runtime(runtime);
    generator.generate(&request(false)).await.unwrap();
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_headers(&requests[0], "Bearer fresh-1");
    assert_headers(&requests[1], "Bearer fresh-2");
    assert_headers(&requests[2], "Bearer fresh-3");
}

fn client() -> (
    json_http::DynJsonHttpClient,
    super::support::RecordedRequests,
) {
    buffered_client(
        vec![
            json_response(json!({"id":"video_1", "status":"queued"})),
            json_response(json!({"id":"video_1", "status":"completed"})),
        ],
        vec![JsonHttpResponse {
            status: 200,
            headers: Default::default(),
            body: b"\0\0\0\x18ftypisom".to_vec(),
        }],
    )
}

fn request(edit: bool) -> VideoGenerationRequest {
    VideoGenerationRequest {
        prompt: "A bird takes flight".to_owned(),
        input_image: edit.then(|| VideoGenerationInputImage {
            data: vec![1, 2],
            mime_type: "image/png".to_owned(),
        }),
        ..Default::default()
    }
}
