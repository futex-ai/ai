//! Google image client transport tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_interface::{ImageGenerationRequest, ImageGenerator};
use json_http::{
    JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::super::client::GoogleImageGenerator;

#[tokio::test]
async fn client_uses_v1_endpoint_auth_and_supports_override() {
    let (http_client, requests) = recording_http_client();
    let generator = GoogleImageGenerator::new(http_client, "gemini-3.1-flash-image", "google-key");

    let response = generator
        .generate(&ImageGenerationRequest {
            prompt: "A blue circle".to_owned(),
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap();

    assert_eq!(response.image.data, vec![1]);
    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(
        requests[0].url,
        "https://generativelanguage.googleapis.com/v1/models/gemini-3.1-flash-image:generateContent"
    );
    assert_eq!(
        requests[0]
            .headers
            .get("x-goog-api-key")
            .map(String::as_str),
        Some("google-key")
    );
    assert_eq!(requests[0].timeout, Duration::from_secs(120));

    let generator = generator
        .with_endpoint("http://images.test/generate")
        .with_timeout(Duration::from_secs(3));
    assert_eq!(generator.endpoint(), "http://images.test/generate");
    assert_eq!(generator.timeout, Duration::from_secs(3));
}

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

fn recording_http_client() -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([JsonHttpResponse {
        status: 200,
        body: json!({"candidates": [{"content": {"parts": [{
            "inlineData": {"mimeType": "image/png", "data": "AQ=="}
        }]}}]}),
    }])));
    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .each_call(matching!(_))
            .answers_arc({
                let requests = requests.clone();
                let responses = responses.clone();
                Arc::new(move |_, request: &JsonHttpRequest| {
                    requests
                        .lock()
                        .expect("request lock should be valid")
                        .push(request.clone());
                    Ok(responses
                        .lock()
                        .expect("response lock should be valid")
                        .pop_front()
                        .expect("unexpected transport call"))
                })
            }),
    ));
    (
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        requests,
    )
}
