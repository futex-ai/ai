//! Config settings cover submission, polling, and the deferred deadline.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ErrorKind, Model};
use ai_models_core::ProviderConfig;
use json_http::{JsonHttpAuthMock, JsonHttpMethod};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::{
    GROK_4_5, PROVIDER_ID, XaiModel,
    deferred_support::{
        advancing_runtime, completed_body, deferred_request, response, scripted_http_client,
    },
};

#[tokio::test]
async fn config_deferred_applies_base_headers_interval_and_timeout_precedence() {
    for (base, configured, request_timeout, total, interval, poll_timeouts) in [
        (None, false, None, 60, 5, [30, 30]),
        (
            Some("https://gateway.test/prefix"),
            true,
            None,
            41,
            7,
            [30, 30],
        ),
        (
            Some("https://gateway.test/prefix/"),
            true,
            Some(13),
            13,
            7,
            [13, 6],
        ),
    ] {
        let (http, requests) = scripted_http_client(vec![
            response(200, json!({"request_id":"req_config"})),
            response(202, json!({"status":"pending"})),
            response(200, completed_body()),
        ]);
        let (runtime, sleeps) = advancing_runtime();
        let mut config = ProviderConfig::new(PROVIDER_ID, "config-key")
            .with_header("AUTHORIZATION", "extra-key")
            .with_header("X-Route", "proxy");
        if let Some(base) = base {
            config = config.with_base_url(base);
        }
        if configured {
            config = config
                .with_total_timeout(Duration::from_secs(41))
                .with_poll_interval(Duration::from_secs(7))
                .with_idle_timeout(Duration::from_secs(1));
        }
        let model = XaiModel::from_config_with_runtime(http, GROK_4_5, config, runtime);
        let mut request = deferred_request(Duration::from_secs(60));
        request.controls.execution.total_timeout = request_timeout.map(Duration::from_secs);

        let response = model.complete(&request).await.unwrap();

        assert_eq!(response.text(), "Done");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 3);
        assert_eq!(requests[0].timeout, Duration::from_secs(total));
        assert_eq!(requests[1].timeout, Duration::from_secs(poll_timeouts[0]));
        assert_eq!(requests[2].timeout, Duration::from_secs(poll_timeouts[1]));
        let prefix = if configured {
            "https://gateway.test/prefix"
        } else {
            "https://api.x.ai"
        };
        assert_eq!(requests[0].url, format!("{prefix}/v1/chat/completions"));
        for request in &requests[1..] {
            assert_eq!(
                request.url,
                format!("{prefix}/v1/chat/deferred-completion/req_config")
            );
            assert_eq!(request.method, JsonHttpMethod::Get);
        }
        for request in requests.iter() {
            assert_eq!(
                request.headers.get("Authorization").map(String::as_str),
                Some("Bearer config-key")
            );
            assert_eq!(
                request.headers.get("X-Route").map(String::as_str),
                Some("proxy")
            );
            assert_eq!(
                request
                    .headers
                    .keys()
                    .filter(|name| name.eq_ignore_ascii_case("authorization"))
                    .count(),
                1
            );
            assert!(request.auth_headers.contains("authorization"));
            assert_eq!(request.idle_timeout, None);
        }
        let body: serde_json::Value =
            serde_json::from_slice(requests[0].body.as_ref().unwrap().as_json_bytes().unwrap())
                .unwrap();
        assert_eq!(body["deferred"], true);
        assert_eq!(body.get("stream"), None);
        assert_eq!(body.get("stream_options"), None);
        assert_eq!(*sleeps.lock().unwrap(), vec![Duration::from_secs(interval)]);
    }
}

#[tokio::test]
async fn config_deferred_refreshes_auth_per_phase_and_redacts_all_hook_secrets() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme submit-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme poll-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme result-secret".to_owned());
                Ok(())
            })
            .once();
    }));
    let (http, requests) = scripted_http_client(vec![
        response(200, json!({"request_id":"req_hook"})),
        response(202, json!({})),
        response(
            400,
            json!({"error":{"message":"submit-secret poll-secret result-secret"}}),
        ),
    ]);
    let (runtime, sleeps) = advancing_runtime();
    let config = ProviderConfig::with_auth(PROVIDER_ID, Arc::new(auth))
        .with_header("X-Auth", "Scheme submit-secret")
        .with_header("X-Route", "proxy")
        .with_poll_interval(Duration::from_secs(2));
    let model = XaiModel::from_config_with_runtime(http, GROK_4_5, config, runtime);

    let error = model
        .complete(&deferred_request(Duration::from_secs(60)))
        .await
        .unwrap_err();

    assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
    assert_eq!(
        error.core().message,
        "HTTP 400: [redacted] [redacted] [redacted]"
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    for (request, value) in requests.iter().zip([
        "Scheme submit-secret",
        "Scheme poll-secret",
        "Scheme result-secret",
    ]) {
        assert_eq!(
            request.headers.get("X-Auth").map(String::as_str),
            Some(value)
        );
        assert_eq!(
            request.headers.get("X-Route").map(String::as_str),
            Some("proxy")
        );
        assert!(request.auth_headers.contains("x-auth"));
    }
    assert_eq!(*sleeps.lock().unwrap(), vec![Duration::from_secs(2)]);
}

#[tokio::test]
async fn config_deferred_deadline_keeps_transient_kind_without_resubmission() {
    let (http, requests) = scripted_http_client(vec![
        response(200, json!({"request_id":"req_deadline"})),
        response(202, json!({})),
        response(202, json!({})),
        response(200, completed_body()),
    ]);
    let (runtime, sleeps) = advancing_runtime();
    let config = ProviderConfig::new(PROVIDER_ID, "config-key")
        .with_total_timeout(Duration::from_secs(3))
        .with_poll_interval(Duration::from_secs(2));
    let model = XaiModel::from_config_with_runtime(http, GROK_4_5, config, runtime);
    let mut request = deferred_request(Duration::from_secs(60));
    request.controls.execution.total_timeout = None;

    let result = model.complete(&request).await;

    assert_eq!(
        result.as_ref().err().map(|error| error.core().kind),
        Some(ErrorKind::Transient)
    );
    assert_eq!(
        result.unwrap_err().core().message,
        "deferred completion exceeded the total call timeout"
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.method == JsonHttpMethod::Post)
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .map(|request| request.timeout)
            .collect::<Vec<_>>(),
        vec![
            Duration::from_secs(3),
            Duration::from_secs(3),
            Duration::from_secs(1)
        ]
    );
    assert_eq!(
        *sleeps.lock().unwrap(),
        vec![Duration::from_secs(2), Duration::from_secs(1)]
    );
}
