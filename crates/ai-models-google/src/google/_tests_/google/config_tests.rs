//! Shared provider settings reach the final Gemini streaming request.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::Model;
use ai_models_core::ProviderConfig;
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{GEMINI_3_6_FLASH, GoogleModel, PROVIDER_ID};

use super::{simple_request, stream_support};

#[tokio::test]
async fn config_stream_defaults_keep_endpoint_auth_and_timeouts() {
    let (client, requests) =
        stream_support::recording_streaming_client(vec![vec![stream_support::event(json!({
            "candidates": [{"finishReason": "STOP", "content": {"parts": [{"text": "Done"}]}}]
        }))]]);
    let model = GoogleModel::from_config(
        client,
        GEMINI_3_6_FLASH,
        ProviderConfig::new(PROVIDER_ID, "config-key"),
    );

    model.complete(&simple_request()).await.unwrap();

    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0]
            .headers
            .get("x-goog-api-key")
            .map(String::as_str),
        Some("config-key")
    );
    assert_eq!(
        requests[0].url,
        "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.6-flash:streamGenerateContent?alt=sse"
    );
    assert_eq!(requests[0].timeout, Duration::from_secs(3_600));
    assert_eq!(requests[0].idle_timeout, Some(Duration::from_secs(120)));
    let body: serde_json::Value =
        serde_json::from_slice(requests[0].body.as_ref().unwrap().as_json_bytes().unwrap())
            .unwrap();
    assert_eq!(
        body["generationConfig"]["thinkingConfig"]["thinkingLevel"],
        "medium"
    );
}

#[tokio::test]
async fn config_stream_preserves_base_prefix_and_call_timeout_priority() {
    for (base, call_timeout, total) in [
        ("https://google.test/proxy", None, 47),
        ("https://google.test/proxy/", Some(79), 79),
    ] {
        let (client, requests) =
            stream_support::recording_streaming_client(vec![vec![stream_support::event(json!({
                "candidates": [{"finishReason": "STOP", "content": {"parts": [{"text": "Done"}]}}]
            }))]]);
        let config = ProviderConfig::new(PROVIDER_ID, "config-key")
            .with_base_url(base)
            .with_header("X-Route", "proxy")
            .with_header("X-GOOG-API-KEY", "extra-key")
            .with_idle_timeout(Duration::from_secs(17))
            .with_total_timeout(Duration::from_secs(47))
            .with_poll_interval(Duration::from_secs(3));
        let model = GoogleModel::from_config(client, GEMINI_3_6_FLASH, config);
        let mut request = simple_request();
        request.controls.execution.total_timeout = call_timeout.map(Duration::from_secs);

        model.complete(&request).await.unwrap();

        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1, "{base}");
        assert_eq!(
            requests[0].url,
            "https://google.test/proxy/v1beta/models/gemini-3.6-flash:streamGenerateContent?alt=sse",
            "{base}"
        );
        assert_eq!(requests[0].timeout, Duration::from_secs(total), "{base}");
        assert_eq!(requests[0].idle_timeout, Some(Duration::from_secs(17)));
        assert_eq!(
            requests[0].headers.get("X-Route").map(String::as_str),
            Some("proxy")
        );
        let keys: Vec<_> = requests[0]
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("x-goog-api-key"))
            .map(|(_, value)| value.as_str())
            .collect();
        assert_eq!(keys, vec!["config-key"]);
    }
}

#[tokio::test]
async fn config_stream_applies_fresh_hook_auth_over_extra_headers() {
    let auth = Arc::new(Unimock::new(JsonHttpAuthMock::apply_headers.stub(
        |calls| {
            calls
                .call(matching!(_))
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("X-GOOG-API-KEY".into(), "first-key".into());
                    Ok(())
                })
                .once()
                .then()
                .answers(&|_, headers: &mut BTreeMap<String, String>| {
                    headers.insert("X-GOOG-API-KEY".into(), "second-key".into());
                    Ok(())
                })
                .once();
        },
    )));
    let chunk =
        json!({"candidates": [{"finishReason": "STOP", "content": {"parts": [{"text": "Done"}]}}]});
    let (client, requests) = stream_support::recording_streaming_client(vec![
        vec![stream_support::event(chunk.clone())],
        vec![stream_support::event(chunk)],
    ]);
    let model = GoogleModel::from_config(
        client,
        GEMINI_3_6_FLASH,
        ProviderConfig::with_auth(PROVIDER_ID, auth).with_header("x-goog-api-key", "extra-key"),
    );

    model.complete(&simple_request()).await.unwrap();
    model.complete(&simple_request()).await.unwrap();

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
    assert_eq!(keys, vec![vec!["first-key"], vec!["second-key"]]);
}
