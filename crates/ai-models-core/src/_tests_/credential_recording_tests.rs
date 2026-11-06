//! Configured custom credential headers stay private in portable recordings.

use std::sync::Arc;

use ai_interface::ProviderId;
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock, MemoryRecordingLog,
    RecordingLog, RecordingTransport, TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::ProviderConfig;

#[tokio::test]
async fn config_gateway_key_is_transmitted_and_redacted_in_recordings() {
    let upstream = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .some_call(matching!(_))
            .answers(&|_, request: &JsonHttpRequest| {
                assert_eq!(
                    request.headers.get("x-gateway-key").map(String::as_str),
                    Some("placeholder-gateway-secret")
                );
                Ok(JsonHttpResponse {
                    status: 200,
                    headers: Default::default(),
                    body: json!({"echo":"placeholder-gateway-secret"}),
                })
            }),
    ));
    let log = Arc::new(MemoryRecordingLog::default());
    let http = TransportBackedJsonHttpClient::new(Arc::new(RecordingTransport::new(
        upstream,
        log.clone(),
    )));
    let config = ProviderConfig::new(ProviderId::new("example"), "placeholder-api-key")
        .with_base_url("https://gateway.test")
        .with_header("x-gateway-key", "placeholder-gateway-secret");
    let response = config
        .apply_headers(
            http.get("https://gateway.test/file"),
            "https://gateway.test",
        )
        .send_value()
        .await
        .unwrap();
    assert_eq!(response.body["echo"], "placeholder-gateway-secret");
    let recordings = log.snapshot();
    assert_eq!(recordings[0].request.headers["x-gateway-key"], "[redacted]");
    assert!(
        !serde_json::to_string(&recordings)
            .unwrap()
            .contains("placeholder-gateway-secret")
    );
}
