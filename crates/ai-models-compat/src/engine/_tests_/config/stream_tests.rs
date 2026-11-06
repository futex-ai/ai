//! Config endpoint construction, defaults, and per-call timeout precedence.

use std::time::Duration;

use ai_models_core::{
    ProviderConfig,
    test_support::{fixtures_for_buffered_responses, recording_streaming_client},
};
use serde_json::Value;

use crate::engine::error_support::request;

use super::support::{CASES, model, response};

#[tokio::test]
async fn config_streams_apply_base_headers_spec_and_timeout_precedence() {
    for (spec, default_url, path, effort) in CASES {
        for (base, configured, call_timeout, expected_total, expected_idle) in [
            (None, false, None, 3_600, 120),
            (Some("https://gateway.test/prefix"), true, None, 19, 7),
            (Some("https://gateway.test/prefix/"), true, Some(11), 11, 7),
        ] {
            let (http, requests) =
                recording_streaming_client(fixtures_for_buffered_responses(vec![response()]));
            let mut config = ProviderConfig::new(spec.provider.clone(), "config-key")
                .with_header("AUTHORIZATION", "extra-auth")
                .with_header("Content-Type", "extra-type")
                .with_header("X-Route", "proxy");
            if let Some(base) = base {
                config = config.with_base_url(base);
            }
            if configured {
                config = config
                    .with_total_timeout(Duration::from_secs(19))
                    .with_idle_timeout(Duration::from_secs(7))
                    .with_poll_interval(Duration::from_secs(2));
            }
            let model = model(http, spec.clone(), config);
            let mut input = request();
            input.controls.execution.total_timeout = call_timeout.map(Duration::from_secs);

            let response = model.complete(&input).await.unwrap();

            assert_eq!(response.text(), "Done");
            assert_eq!(response.effort(), Some(*effort));
            let requests = requests.lock().unwrap();
            assert_eq!(requests.len(), 1);
            let request = &requests[0];
            let expected_url = if configured {
                format!("https://gateway.test/prefix{path}")
            } else {
                (*default_url).to_owned()
            };
            assert_eq!(request.url, expected_url, "{} base={base:?}", spec.provider);
            assert_eq!(request.timeout, Duration::from_secs(expected_total));
            assert_eq!(
                request.idle_timeout,
                Some(Duration::from_secs(expected_idle))
            );
            let auth: Vec<_> = request
                .headers
                .iter()
                .filter(|(name, _)| name.eq_ignore_ascii_case("authorization"))
                .collect();
            assert_eq!(
                auth,
                vec![(&"Authorization".to_owned(), &"Bearer config-key".to_owned())]
            );
            assert_eq!(
                request.headers.get("X-Route").map(String::as_str),
                Some("proxy")
            );
            assert_eq!(
                request
                    .headers
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case("content-type")),
                None
            );
            assert!(request.auth_headers.contains("authorization"));
            let body: Value =
                serde_json::from_slice(request.body.as_ref().unwrap().as_json_bytes().unwrap())
                    .unwrap();
            assert_eq!(body["model"], spec.provider_model_id.as_ref());
            assert_eq!(body["stream"], true);
            assert_eq!(body.get("deferred"), None);
        }
    }
}
