//! Google video config controls every phase without widening auth scope.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ModelSpec, VideoGenerator};
use ai_models_core::ProviderConfig;
use json_http::{JsonHttpAuthMock, JsonHttpResponse};
use serde_json::{Value, json};
use unimock::{MockFn, Unimock, matching};

use crate::{CUSTOM_DEFAULTS, GoogleVideoGenerator, PROVIDER_ID};

use super::client_tests::{advancing_runtime, request, scripted_client};

const OPERATION: &str = "models/custom-video-model/operations/config_1";

#[tokio::test]
async fn config_video_applies_base_headers_and_deadline_to_every_phase() {
    for (base, timeout, interval, submit, poll, sample) in [
        (
            None,
            600,
            10,
            "https://generativelanguage.googleapis.com/v1beta/models/custom-video-model:predictLongRunning",
            "https://generativelanguage.googleapis.com/v1beta/models/custom-video-model/operations/config_1",
            "https://generativelanguage.googleapis.com/v1beta/files/1",
        ),
        (
            Some("https://google.test/proxy"),
            31,
            3,
            "https://google.test/proxy/v1beta/models/custom-video-model:predictLongRunning",
            "https://google.test/proxy/v1beta/models/custom-video-model/operations/config_1",
            "https://google.test/proxy/files/1",
        ),
        (
            Some("https://google.test/proxy/"),
            31,
            3,
            "https://google.test/proxy/v1beta/models/custom-video-model:predictLongRunning",
            "https://google.test/proxy/v1beta/models/custom-video-model/operations/config_1",
            "https://google.test/proxy/files/1",
        ),
    ] {
        let (client, requests) = scripted_client(
            vec![
                ok_json(json!({"name": OPERATION})),
                ok_json(completed(sample)),
            ],
            vec![Ok(JsonHttpResponse {
                status: 200,
                headers: Default::default(),
                body: vec![1],
            })],
        );
        let mut config = ProviderConfig::new(PROVIDER_ID, "config-key")
            .with_header("X-Route", "proxy")
            .with_header("X-GOOG-API-KEY", "extra-key")
            .with_idle_timeout(Duration::from_secs(17));
        if let Some(base) = base {
            config = config
                .with_base_url(base)
                .with_total_timeout(Duration::from_secs(timeout))
                .with_poll_interval(Duration::from_secs(interval));
        }
        let (runtime, sleeps) = advancing_runtime();
        let mut generator = GoogleVideoGenerator::from_config(client, video_spec(), config);
        generator.runtime = runtime;

        let response = generator.generate(&request()).await.unwrap();

        assert_eq!(response.video.data, vec![1]);
        assert_eq!(*sleeps.lock().unwrap(), vec![Duration::from_secs(interval)]);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(
            requests
                .iter()
                .map(|request| request.url.as_str())
                .collect::<Vec<_>>(),
            vec![submit, poll, sample]
        );
        assert_eq!(
            requests
                .iter()
                .map(|request| request.timeout)
                .collect::<Vec<_>>(),
            vec![
                Duration::from_secs(timeout),
                Duration::from_secs(timeout - interval),
                Duration::from_secs(timeout - interval)
            ]
        );
        for request in requests.iter() {
            assert_eq!(request.idle_timeout, None);
            assert_eq!(
                request.headers.get("X-Route").map(String::as_str),
                Some("proxy")
            );
            let keys: Vec<_> = request
                .headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
                .map(|(_, value)| value.as_str())
                .collect();
            assert_eq!(keys, vec!["config-key"]);
        }
    }
}

#[tokio::test]
async fn config_video_applies_fresh_hook_to_submission_poll_and_download() {
    let auth = Arc::new(Unimock::new(JsonHttpAuthMock::apply_headers.stub(
        |calls| {
            calls
                .call(matching!(_))
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("X-Goog-Api-Key".into(), "submit-key".into());
                    Ok(())
                })
                .once()
                .then()
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("X-Goog-Api-Key".into(), "poll-key".into());
                    Ok(())
                })
                .once()
                .then()
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("X-Goog-Api-Key".into(), "download-key".into());
                    Ok(())
                })
                .once();
        },
    )));
    let sample = "https://generativelanguage.googleapis.com/v1beta/files/1";
    let (client, requests) = scripted_client(
        vec![
            ok_json(json!({"name": OPERATION})),
            ok_json(completed(sample)),
        ],
        vec![Ok(JsonHttpResponse {
            status: 200,
            headers: Default::default(),
            body: vec![1],
        })],
    );
    let config =
        ProviderConfig::with_auth(PROVIDER_ID, auth).with_header("x-goog-api-key", "extra-key");
    let (runtime, _) = advancing_runtime();
    let mut generator = GoogleVideoGenerator::from_config(client, video_spec(), config);
    generator.runtime = runtime;

    generator.generate(&request()).await.unwrap();

    let requests = requests.lock().unwrap();
    let keys: Vec<Vec<_>> = requests
        .iter()
        .map(|request| {
            request
                .headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
                .map(|(_, value)| value.as_str())
                .collect()
        })
        .collect();
    assert_eq!(
        keys,
        vec![vec!["submit-key"], vec!["poll-key"], vec!["download-key"]]
    );
}

pub(super) fn video_spec() -> ModelSpec {
    ModelSpec::custom(PROVIDER_ID, "custom-video-model", &CUSTOM_DEFAULTS)
}

pub(super) fn completed(sample: &str) -> Value {
    json!({"name": OPERATION, "done": true,
        "response": {"generateVideoResponse": {"generatedSamples": [{"video": {"uri": sample}}]}}})
}

pub(super) fn ok_json(body: Value) -> json_http::Result<JsonHttpResponse<Value>> {
    Ok(JsonHttpResponse {
        status: 200,
        headers: Default::default(),
        body,
    })
}
