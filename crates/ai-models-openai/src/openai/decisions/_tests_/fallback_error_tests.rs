//! Decisions diagnostics for unexpected response bodies.

use ai_interface::{
    ErrorKind, JudgmentContent, JudgmentError, JudgmentModel, JudgmentPart, JudgmentRequest,
    SafeErrorSource,
};
use serde_json::json;

use crate::OpenAiJudgmentModel;

use super::support::{assert_safe, client, config, predicate, response, spec};

const HEADER_VALUE: &str = r#"example-header"quoted-value"#;
const HEADER_MARKER: &str = "example-header";
const IMAGE_DATA: &str = "c29tZS1kYXRh/+4=";
const IMAGE_MARKER: &str = "c29tZS1kYXRh";

#[tokio::test]
async fn malformed_usage_reports_only_decoder_category_and_location() {
    let error = error_for_body(
        200,
        r#"{"model":"decisions-release",
"usage":{"input_tokens":"example-header\"quoted-value"},
"answers":[{"name":"q0","type":"predicate","probability":0.9}]}"#,
        &predicate(),
    )
    .await;

    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_safe(&error, &[HEADER_MARKER]);
    assert_eq!(
        error.core().message,
        "malformed provider payload: Data at line 2 column 54"
    );
    let JudgmentError::Internal { source, .. } = error else {
        panic!("expected the retained decoder error");
    };
    let safe_source = source
        .source_ref()
        .downcast_ref::<SafeErrorSource>()
        .unwrap();
    assert_eq!(safe_source.to_string(), "[redacted source]");
    let raw = safe_source
        .raw_cause()
        .downcast_ref::<serde_json::Error>()
        .unwrap();
    assert!(raw.to_string().contains(HEADER_MARKER));
}

#[tokio::test]
async fn transport_decode_failure_hides_raw_serde_source_chain() {
    let raw = serde_json::from_str::<u64>(r#""example-header\"quoted-value""#).unwrap_err();
    let (http, _) = client(Err(json_http::Error::DeserializeResponse {
        body: json!(HEADER_VALUE),
        source: raw,
    }));
    let model = OpenAiJudgmentModel::from_config(
        http,
        spec(),
        config().with_header("X-Fixture-Auth", HEADER_VALUE),
    );

    let error = model.judge(&predicate()).await.unwrap_err();

    assert_eq!(error.core().kind, ErrorKind::InvalidOutput);
    assert_safe(&error, &[HEADER_MARKER]);
    assert_eq!(
        error.core().message,
        "malformed provider payload: Data at line 1 column 30"
    );
}

#[tokio::test]
async fn truncated_json_does_not_expose_escaped_header() {
    assert_status_error(
        r#"{"auth":"example-header\"quoted-value""#,
        &predicate(),
        &[HEADER_MARKER],
    )
    .await;
}

#[tokio::test]
async fn plain_text_with_json_does_not_expose_escaped_header() {
    assert_status_error(
        r#"gateway rejected {"auth":"example-header\"quoted-value"}"#,
        &predicate(),
        &[HEADER_MARKER],
    )
    .await;
}

#[tokio::test]
async fn nested_json_string_does_not_expose_escaped_header() {
    assert_status_error(
        r#"{"error":{"message":{"detail":"{\"auth\":\"example-header\\\"quoted-value\"}"}}}"#,
        &predicate(),
        &[HEADER_MARKER],
    )
    .await;
}

#[tokio::test]
async fn truncated_json_does_not_expose_escaped_image_data() {
    let mut request = predicate();
    request.input = JudgmentContent {
        parts: vec![JudgmentPart::Image {
            mime_type: "image/png".to_owned(),
            data_base64: IMAGE_DATA.to_owned(),
        }],
    }
    .into();

    assert_status_error(
        r#"{"auth":"example-header\"quoted-value","image":"c29tZS1kYXRh\/+4=""#,
        &request,
        &[IMAGE_MARKER, HEADER_MARKER],
    )
    .await;
}

#[tokio::test]
async fn json_without_an_envelope_uses_fixed_status_message() {
    assert_status_error(
        r#"{
            "message": "gateway rejected example-header\"quoted-value",
            "nested": {
                "example-header\u0022quoted-value": ["example-header\u0022quoted-value", 7, true, null]
            }
        }"#,
        &predicate(),
        &[HEADER_MARKER],
    )
    .await;
}

async fn assert_status_error(body: &str, request: &JudgmentRequest, secrets: &[&str]) {
    let error = error_for_body(400, body, request).await;
    assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
    assert_safe(&error, secrets);
    assert!(
        error.core().message == "HTTP 400",
        "expected a fixed status diagnostic"
    );
}

async fn error_for_body(status: u16, body: &str, request: &JudgmentRequest) -> JudgmentError {
    let mut reply = response(status, json!(null));
    reply.body = body.as_bytes().to_vec();
    let (http, _) = client(Ok(reply));
    let model = OpenAiJudgmentModel::from_config(
        http,
        spec(),
        config().with_header("X-Fixture-Auth", HEADER_VALUE),
    );
    model.judge(request).await.unwrap_err()
}
