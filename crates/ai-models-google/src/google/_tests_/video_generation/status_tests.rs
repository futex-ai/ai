//! Final redirect statuses never enter video operation parsing or binary output.

use std::{sync::Arc, time::Duration};

use ai_interface::ErrorKind;
use ai_models_core::ProviderConfig;
use json_http::{JsonHttpResponse, JsonHttpTransportMock, TransportBackedJsonHttpClient};
use unimock::{MockFn, Unimock, matching};

use crate::google::video_generation::deadline::VideoDeadline;
use crate::{GoogleVideoGenerator, PROVIDER_ID};

use super::{
    client_tests::{request, scripted_client, stationary_runtime},
    config_tests::{completed, video_spec},
};

const SAMPLE: &str = "https://google.test/files/1";
const TIMEOUT: Duration = Duration::from_secs(10);

#[tokio::test]
async fn final_redirect_bodies_never_become_video_submissions() {
    for status in [300, 302, 307, 308] {
        let (http, _) = scripted_client(
            vec![Ok(JsonHttpResponse {
                status,
                headers: Default::default(),
                body: completed(SAMPLE),
            })],
            vec![],
        );
        let generator = GoogleVideoGenerator::from_config(http, video_spec(), config());
        let runtime = stationary_runtime();
        let deadline = VideoDeadline::new(runtime.as_ref(), TIMEOUT);
        let result = generator
            .submit(&generator.context(), &request(), &deadline)
            .await;
        assert_eq!(
            result.as_ref().err().map(|error| error.core().kind),
            Some(ErrorKind::InvalidOutput),
            "status {status}"
        );
    }
}

#[tokio::test]
async fn final_redirect_bodies_never_become_video_poll_results() {
    for status in [300, 302, 307, 308] {
        let (http, _) = scripted_client(
            vec![Ok(JsonHttpResponse {
                status,
                headers: Default::default(),
                body: completed(SAMPLE),
            })],
            vec![],
        );
        let generator = GoogleVideoGenerator::from_config(http, video_spec(), config());
        let runtime = stationary_runtime();
        let deadline = VideoDeadline::new(runtime.as_ref(), TIMEOUT);
        let result = generator
            .retrieve(
                &generator.context(),
                "models/custom-video-model/operations/config_1",
                &deadline,
            )
            .await;
        assert_eq!(
            result.as_ref().err().map(|error| error.core().kind),
            Some(ErrorKind::InvalidOutput),
            "status {status}"
        );
    }
}

#[tokio::test]
async fn final_redirect_bodies_never_become_downloaded_video_bytes() {
    for status in 300..=308 {
        let transport = Arc::new(Unimock::new(
            JsonHttpTransportMock::execute_bytes
                .some_call(matching!(_))
                .returns(Ok(JsonHttpResponse {
                    status,
                    headers: Default::default(),
                    body: b"<html>redirect</html>".to_vec(),
                })),
        ));
        let http = Arc::new(TransportBackedJsonHttpClient::new(transport));
        let generator = GoogleVideoGenerator::from_config(http, video_spec(), config());
        let runtime = stationary_runtime();
        let deadline = VideoDeadline::new(runtime.as_ref(), TIMEOUT);
        let result = generator
            .download(&generator.context(), SAMPLE, &deadline)
            .await;
        assert_eq!(
            result.as_ref().err().map(|error| error.core().kind),
            Some(ErrorKind::InvalidOutput),
            "status {status}"
        );
    }
}

fn config() -> ProviderConfig {
    ProviderConfig::new(PROVIDER_ID, "placeholder-key").with_base_url("https://google.test")
}
