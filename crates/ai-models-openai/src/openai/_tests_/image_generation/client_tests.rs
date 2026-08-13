//! OpenAI image client configuration tests.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_interface::{
    ImageGenerationError, ImageGenerationInputImage, ImageGenerationRequest, ImageGenerator,
};
use json_http::{
    JsonHttpBody, JsonHttpClient, JsonHttpMultipartField, JsonHttpRequest, JsonHttpResponse,
    JsonHttpTransportMock, TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use super::super::client::OpenAiImageGenerator;

#[tokio::test]
async fn client_uses_injected_transport_auth_timeout_and_endpoints() {
    let (http_client, requests) = recording_http_client(VecDeque::from([
        successful_response(),
        successful_response(),
    ]));
    let generator = OpenAiImageGenerator::new(http_client, "gpt-image-2", "sk-test");

    assert_eq!(generator.timeout, Duration::from_secs(120));
    assert_eq!(
        generator.generation_endpoint,
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(
        generator.edit_endpoint,
        "https://api.openai.com/v1/images/edits"
    );

    generator
        .generate(&ImageGenerationRequest {
            prompt: "A blue circle".to_owned(),
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap();

    let generator = generator
        .with_endpoints("http://generation.test", "http://edit.test")
        .with_timeout(Duration::from_secs(3));
    assert_eq!(generator.generation_endpoint, "http://generation.test");
    assert_eq!(generator.edit_endpoint, "http://edit.test");
    assert_eq!(generator.timeout, Duration::from_secs(3));

    generator
        .generate(&ImageGenerationRequest {
            prompt: "Make this blue".to_owned(),
            input_images: vec![ImageGenerationInputImage {
                data: vec![1, 2, 3],
                mime_type: "image/png".to_owned(),
            }],
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap();

    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(
        requests[0].url,
        "https://api.openai.com/v1/images/generations"
    );
    assert_eq!(requests[0].timeout, Duration::from_secs(120));
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer sk-test")
    );
    let generation = requests[0]
        .body
        .as_ref()
        .and_then(JsonHttpBody::as_json)
        .expect("generation body should be JSON");
    assert_eq!(generation["model"], "gpt-image-2");
    assert_eq!(generation["prompt"], "A blue circle");
    assert_eq!(generation["n"], 1);

    assert_eq!(requests[1].url, "http://edit.test");
    assert_eq!(requests[1].timeout, Duration::from_secs(3));
    assert_eq!(
        requests[1].headers.get("Authorization").map(String::as_str),
        Some("Bearer sk-test")
    );
    let Some(JsonHttpBody::Multipart(edit)) = requests[1].body.as_ref() else {
        panic!("edit body should be multipart");
    };
    assert_text_field(&edit.fields, "model", b"gpt-image-2");
    assert_text_field(&edit.fields, "prompt", b"Make this blue");
    assert_text_field(&edit.fields, "size", b"auto");
    assert_text_field(&edit.fields, "quality", b"auto");
    assert_text_field(&edit.fields, "n", b"1");
    assert!(edit.fields.iter().any(|field| {
        field.name == "image[]"
            && field.filename.as_deref() == Some("image-0.png")
            && field.content_type.as_deref() == Some("image/png")
            && field.bytes == vec![1, 2, 3]
    }));
}

#[tokio::test]
async fn client_classifies_injected_status_and_transport_failures() {
    let (http_client, _) = recording_http_client(VecDeque::from([JsonHttpResponse {
        status: 429,
        body: json!({"error": {"message": "slow down"}}),
    }]));
    let error = OpenAiImageGenerator::new(http_client, "gpt-image-2", "sk-test")
        .generate(&ImageGenerationRequest {
            prompt: "A blue circle".to_owned(),
            ..ImageGenerationRequest::default()
        })
        .await
        .unwrap_err();
    assert!(
        matches!(error, ImageGenerationError::RateLimited { message, .. } if message == "slow down")
    );

    let transport = Arc::new(Unimock::new(
        JsonHttpTransportMock::execute
            .next_call(matching!(_))
            .returns(Err(json_http::Error::transport("offline"))),
    ));
    let error = OpenAiImageGenerator::new(
        Arc::new(TransportBackedJsonHttpClient::new(transport)),
        "gpt-image-2",
        "sk-test",
    )
    .generate(&ImageGenerationRequest {
        prompt: "A blue circle".to_owned(),
        ..ImageGenerationRequest::default()
    })
    .await
    .unwrap_err();
    assert!(
        matches!(error, ImageGenerationError::TransientProvider { message, .. } if message.contains("offline"))
    );
}

type RecordedRequests = Arc<Mutex<Vec<JsonHttpRequest>>>;

fn recording_http_client(
    responses: VecDeque<JsonHttpResponse<serde_json::Value>>,
) -> (Arc<dyn JsonHttpClient>, RecordedRequests) {
    let requests = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(responses));
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

fn successful_response() -> JsonHttpResponse<serde_json::Value> {
    JsonHttpResponse {
        status: 200,
        body: json!({
            "data": [{"b64_json": "iVBORw0KGgo="}],
            "output_format": "png"
        }),
    }
}

fn assert_text_field(fields: &[JsonHttpMultipartField], name: &str, expected: &[u8]) {
    let field = fields
        .iter()
        .find(|field| field.name == name)
        .expect("multipart text field should be present");
    assert_eq!(field.bytes, expected);
    assert_eq!(field.filename, None);
    assert_eq!(field.content_type, None);
}
