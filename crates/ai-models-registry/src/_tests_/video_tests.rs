//! Registry video factories forward custom specs and settings through generation.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ModelSpec, VideoGenerationRequest};
use ai_models_google::{CUSTOM_DEFAULTS, PROVIDER_ID};
use json_http::{
    DynJsonHttpAuth, DynJsonHttpClient, JsonHttpAuthMock, JsonHttpMethod, JsonHttpRequest,
    JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{ProviderConfig, Registry};

use super::media_support::Requests;

const SAMPLE: &str = "https://proxy.test/team/files/clip.mp4";

#[tokio::test]
async fn video_factory_forwards_custom_spec_and_settings_to_submission_and_download() {
    for base in ["https://proxy.test/team", "https://proxy.test/team/"] {
        let (http, requests) = completed_video_client();
        let spec = ModelSpec::custom(PROVIDER_ID, "registry-video-model", &CUSTOM_DEFAULTS);
        let registry = Registry::new(http).with_provider(
            ProviderConfig::with_auth(PROVIDER_ID, video_auth())
                .with_base_url(base)
                .with_header("X-Tenant", "tenant")
                .with_header("X-Goog-Api-Key", "extra-key")
                .with_total_timeout(Duration::from_secs(31)),
        );
        let built = registry.video(&spec);
        assert_eq!(built.as_ref().err(), None, "{base}");

        let response = built
            .unwrap()
            .generate(&VideoGenerationRequest {
                prompt: "A bird takes flight".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(response.provider, "google");
        assert_eq!(response.model_id, "registry-video-model");
        assert_eq!(response.video.data, vec![1, 2, 3]);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 2, "{base}");
        assert_eq!(requests[0].method, JsonHttpMethod::Post);
        assert_eq!(
            requests[0].url,
            "https://proxy.test/team/v1beta/models/registry-video-model:predictLongRunning"
        );
        assert_eq!(requests[1].method, JsonHttpMethod::Get);
        assert_eq!(requests[1].url, SAMPLE);
        assert!(requests[0].timeout <= Duration::from_secs(31));
        assert!(requests[1].timeout <= requests[0].timeout);
        for (request, key) in requests.iter().zip(["submit-key", "download-key"]) {
            assert!(!request.timeout.is_zero());
            assert_eq!(
                request.headers.get("X-Tenant").map(String::as_str),
                Some("tenant")
            );
            let keys: Vec<_> = request
                .headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
                .map(|(_, value)| value.as_str())
                .collect();
            assert_eq!(keys, vec![key]);
        }
    }
}

fn video_auth() -> DynJsonHttpAuth {
    Arc::new(Unimock::new(JsonHttpAuthMock::apply_headers.stub(
        |calls| {
            calls
                .call(matching!(_))
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("x-goog-api-key".into(), "submit-key".into());
                    Ok(())
                })
                .once()
                .then()
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("x-goog-api-key".into(), "download-key".into());
                    Ok(())
                })
                .once();
        },
    )))
}

fn completed_video_client() -> (DynJsonHttpClient, Requests) {
    let requests = Requests::default();
    let transport = Unimock::new((
        JsonHttpTransportMock::execute
            .some_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests.lock().unwrap().push(request.clone());
                    Ok(JsonHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: json!({
                            "name": "models/registry-video-model/operations/completed",
                            "done": true,
                            "response": {"generateVideoResponse": {"generatedSamples": [
                                {"video": {"uri": SAMPLE}}
                            ]}}
                        }),
                    })
                })
            }),
        JsonHttpTransportMock::execute_bytes
            .some_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests.lock().unwrap().push(request.clone());
                    Ok(JsonHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: vec![1, 2, 3],
                    })
                })
            }),
    ));
    (
        Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport))),
        requests,
    )
}
