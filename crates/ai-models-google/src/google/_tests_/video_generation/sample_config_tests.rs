//! Sample validation uses the configured host and scopes authentication by origin.

use std::sync::{Arc, Mutex};

use ai_interface::{ErrorKind, VideoGenerator};
use ai_models_core::ProviderConfig;
use json_http::{
    JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use unimock::{MockFn, Unimock, matching};

use crate::{GoogleVideoGenerator, PROVIDER_ID};

use super::{
    client_tests::{request, scripted_client, stationary_runtime},
    config_tests::{completed, ok_json, video_spec},
};

#[tokio::test]
async fn config_video_accepts_https_base_host_and_scopes_download_auth() {
    for (base, sample, expected_key) in [
        (
            "https://google.test/proxy",
            "https://google.test:8443/files/1",
            None,
        ),
        (
            "http://google.test/proxy",
            "https://google.test/files/1",
            None,
        ),
        (
            "https://GOOGLE.test:443/proxy/",
            "https://google.test/files/1",
            Some("config-key"),
        ),
    ] {
        let (client, requests) = scripted_client(
            vec![ok_json(completed(sample))],
            vec![Ok(JsonHttpResponse {
                status: 200,
                headers: Default::default(),
                body: vec![1],
            })],
        );
        let mut generator = GoogleVideoGenerator::from_config(
            client,
            video_spec(),
            ProviderConfig::new(PROVIDER_ID, "config-key")
                .with_base_url(base)
                .with_header("x-gateway-key", "gateway-secret")
                .with_header("X-GOOG-API-KEY", "extra-key"),
        );
        generator.runtime = stationary_runtime();

        let result = generator.generate(&request()).await;

        assert_eq!(
            result
                .as_ref()
                .ok()
                .map(|response| response.video.data.as_slice()),
            Some([1].as_slice()),
            "{base} -> {sample}"
        );
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[1].url, sample);
        assert_eq!(
            requests[0].headers.get("x-gateway-key").map(String::as_str),
            Some("gateway-secret")
        );
        assert_eq!(
            requests[1].headers.get("x-gateway-key").map(String::as_str),
            expected_key.map(|_| "gateway-secret"),
            "{base} -> {sample}"
        );
        assert_eq!(
            requests[1]
                .headers
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
                .map(|(_, value)| value.as_str()),
            expected_key,
            "{base} -> {sample}"
        );
        assert_eq!(
            requests[1]
                .headers
                .get("x-goog-api-key")
                .map(String::as_str),
            expected_key,
            "{base} -> {sample}"
        );
    }
}

#[tokio::test]
async fn config_video_rejects_samples_without_https_and_the_configured_host() {
    for sample in [
        "https://generativelanguage.googleapis.com/v1beta/files/1",
        "http://google.test/files/1",
        "https://google.test.attacker.example/files/1",
        "https://google.test@attacker.example/files/1",
    ] {
        let requests = Arc::new(Mutex::new(Vec::new()));
        let transport = Unimock::new(
            JsonHttpTransportMock::execute
                .some_call(matching!(_))
                .answers_arc({
                    let requests = requests.clone();
                    Arc::new(move |_, request: &JsonHttpRequest| {
                        requests.lock().unwrap().push(request.clone());
                        ok_json(completed(sample))
                    })
                }),
        );
        let client = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport)));
        let mut generator = GoogleVideoGenerator::from_config(
            client,
            video_spec(),
            ProviderConfig::new(PROVIDER_ID, "config-key")
                .with_base_url("https://google.test/proxy"),
        );
        generator.runtime = stationary_runtime();

        let result = generator.generate(&request()).await;

        assert_eq!(
            result.as_ref().err().map(|error| error.core().kind),
            Some(ErrorKind::InvalidOutput),
            "{sample}"
        );
        assert_eq!(requests.lock().unwrap().len(), 1, "{sample}");
    }
}
