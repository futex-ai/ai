//! Decisions status, transport, support, and redaction tests.

use std::{collections::BTreeMap, error::Error, sync::Arc, time::Duration};

use ai_interface::{
    ConversationMessage, ConversationPart, ErrorKind, JudgmentContent,
    JudgmentConversationPartKind, JudgmentError, JudgmentInput, JudgmentInputFeature,
    JudgmentModel, JudgmentPart, JudgmentQuestion,
};
use ai_models_core::ProviderConfig;
use json_http::{
    JsonHttpAuthMock, MemoryRecordingLog, RecordingLog, RecordingTransport, ReplayTransport,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{OpenAiJudgmentModel, PROVIDER_ID, openai::error_context_support::headers};

use super::support::{
    API_KEY, MODEL_ID, client, config, predicate, request, response, sent_request, spec,
};

const INPUT_IMAGE: &str = "RklYVFVSRV9JTUFHRQ==";
const INSTRUCTION_IMAGE: &str = "RklYVFVSRV9JTlNUUlVDVElPTg==";
const CONVERSATION_IMAGE: &str = "RklYVFVSRV9DT05WRVJTQVRJT04=";
const EXTRA_HEADER: &str = "fixture-extra-header";
const HOOK_VALUE: &str = "fixture-auth-value";

#[tokio::test]
async fn known_native_codes_override_status_and_keep_response_metadata() {
    for (code, status, expected) in [
        ("insufficient_quota", 429, ErrorKind::RejectedKey),
        (
            "context_length_exceeded",
            400,
            ErrorKind::ContextLimitExceeded,
        ),
    ] {
        let mut reply = response(
            status,
            json!({"error":{"code":code,"message":"fixture rejection"}}),
        );
        reply.headers = headers("req-decisions", "9");
        let (http, requests) = client(Ok(reply));
        let model = OpenAiJudgmentModel::from_config(http, spec(), config());

        let error = model.judge(&predicate()).await.unwrap_err();

        assert_eq!(error.core().kind, expected);
        assert_eq!(error.core().message, "fixture rejection");
        assert_eq!(error.core().provider, "openai");
        assert_eq!(error.core().model_id, MODEL_ID);
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-decisions")
        );
        assert_eq!(error.core().retry_after, Some(Duration::from_secs(9)));
        sent_request(&requests);
    }
}

#[tokio::test]
async fn unknown_codes_and_non_envelope_bodies_use_http_status() {
    for (mut reply, kind, expected) in [
        (
            response(
                429,
                json!({"error":{"code":"new_code","message":"slow down"}}),
            ),
            ErrorKind::RateLimited,
            "slow down",
        ),
        (
            response(503, json!(null)),
            ErrorKind::Transient,
            "proxy unavailable",
        ),
    ] {
        if reply.status == 503 {
            reply.body = b"proxy unavailable".to_vec();
        }
        let (http, _) = client(Ok(reply));
        let model = OpenAiJudgmentModel::from_config(http, spec(), config());

        let error = model.judge(&predicate()).await.unwrap_err();

        assert_eq!(error.core().kind, kind);
        assert_eq!(error.core().message, expected);
    }
}

#[tokio::test]
async fn provider_errors_redact_configured_and_hook_credentials_and_images() {
    let mut image_request = request(JudgmentQuestion::condition(image(INSTRUCTION_IMAGE), None));
    image_request.input = image(INPUT_IMAGE).into();
    let mut reply = response(
        400,
        json!({"error":{"message":format!(
            "ordinary text {API_KEY} {EXTRA_HEADER} {INPUT_IMAGE} {INSTRUCTION_IMAGE}"
        )}}),
    );
    reply.headers = headers(&format!("req-{API_KEY}"), "4");
    let (http, _) = client(Ok(reply));
    let model = OpenAiJudgmentModel::from_config(
        http,
        spec(),
        config().with_header("X-Extra", EXTRA_HEADER),
    );

    let error = model.judge(&image_request).await.unwrap_err();

    assert_eq!(
        error.core().message,
        "ordinary text [redacted] [redacted] [redacted] [redacted]"
    );
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-[redacted]")
    );
    assert_safe(
        &error,
        &[API_KEY, EXTRA_HEADER, INPUT_IMAGE, INSTRUCTION_IMAGE],
    );

    let auth = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .some_call(matching!(_))
            .answers(&|_, values: &mut BTreeMap<String, String>| {
                values.insert("X-Auth".to_owned(), format!("Scheme {HOOK_VALUE}"));
                Ok(())
            }),
    ));
    let mut conversation = predicate();
    conversation.input = JudgmentInput::Conversation {
        system: None,
        messages: vec![ConversationMessage::user_with_parts(vec![
            ConversationPart::image("image/png", CONVERSATION_IMAGE),
        ])],
    };
    let (http, requests) = client(Ok(response(
        400,
        json!({"error":{"message":format!(
            "ordinary text Scheme {HOOK_VALUE} {HOOK_VALUE} {CONVERSATION_IMAGE}"
        )}}),
    )));
    let model = OpenAiJudgmentModel::from_config(
        http,
        spec(),
        ProviderConfig::with_auth(PROVIDER_ID, auth),
    );

    let error = model.judge(&conversation).await.unwrap_err();

    assert_eq!(
        error.core().message,
        "ordinary text [redacted] [redacted] [redacted]"
    );
    assert_safe(&error, &[HOOK_VALUE, CONVERSATION_IMAGE]);
    assert_eq!(
        sent_request(&requests)
            .headers
            .get("X-Auth")
            .map(String::as_str),
        Some(format!("Scheme {HOOK_VALUE}").as_str())
    );
}

#[tokio::test]
async fn transport_timeout_uses_config_and_returns_one_safe_transient_failure() {
    let (http, requests) = client(Err(json_http::Error::transport(format!(
        "timed out {API_KEY} {INPUT_IMAGE}"
    ))));
    let model = OpenAiJudgmentModel::from_config(
        http,
        spec(),
        config()
            .with_base_url("https://proxy.example.test/project/")
            .with_total_timeout(Duration::from_secs(17)),
    );
    let mut request = predicate();
    request.input = image(INPUT_IMAGE).into();

    let error = model.judge(&request).await.unwrap_err();

    assert_eq!(error.core().kind, ErrorKind::Transient);
    assert!(error.core().message.contains("timed out"));
    assert_safe(&error, &[API_KEY, INPUT_IMAGE]);
    let http = sent_request(&requests);
    assert_eq!(http.timeout, Duration::from_secs(17));
    assert_eq!(http.url, "https://proxy.example.test/project/v1/decisions");
}

#[tokio::test]
async fn malformed_output_redacts_decoder_diagnostics() {
    let mut input = predicate();
    input.input = image(INPUT_IMAGE).into();
    let mut reply = response(
        200,
        json!({"model":"decisions-release","answers":[
            {"name":"q0","type": format!("{API_KEY} {INPUT_IMAGE}")}
        ]}),
    );
    reply.headers = headers("req-decode", "3");
    let (http, _) = client(Ok(reply));
    let model = OpenAiJudgmentModel::from_config(http, spec(), config());

    let result = model.judge(&input).await;

    assert!(
        matches!(&result, Err(error) if error.core().kind == ErrorKind::InvalidOutput
            && error.core().message.contains("malformed provider payload")),
        "malformed output must fail with a safe decoder diagnostic"
    );
    let error = result.unwrap_err();
    assert_eq!(
        error.core().provider_request_id.as_deref(),
        Some("req-decode")
    );
    assert_safe(&error, &[API_KEY, INPUT_IMAGE]);
}

#[tokio::test]
async fn video_is_rejected_before_transport() {
    let log = Arc::new(MemoryRecordingLog::default());
    let transport = RecordingTransport::new(Arc::new(ReplayTransport::new(vec![])), log.clone());
    let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(transport)));
    let model = OpenAiJudgmentModel::from_config(http, spec(), config());
    let mut input = predicate();
    input.input = JudgmentInput::Conversation {
        system: None,
        messages: vec![ConversationMessage::user_with_parts(vec![
            ConversationPart::video("video/mp4", "AQID"),
        ])],
    };

    let result = model.judge(&input).await;

    assert_eq!(log.snapshot().len(), 0);
    assert!(
        matches!(&result, Err(JudgmentError::UnsupportedInput { core, feature:
            JudgmentInputFeature::ConversationPart(JudgmentConversationPartKind::Video) })
            if core.kind == ErrorKind::UnsupportedInput && core.provider == "openai" && core.model_id == MODEL_ID),
        "{result:?}"
    );
}

fn image(data_base64: &str) -> JudgmentContent {
    JudgmentContent {
        parts: vec![JudgmentPart::Image {
            mime_type: "image/png".to_owned(),
            data_base64: data_base64.to_owned(),
        }],
    }
}

fn assert_safe(error: &JudgmentError, secrets: &[&str]) {
    let mut link: Option<&dyn Error> = Some(error);
    while let Some(error) = link {
        for secret in secrets {
            assert!(
                !error.to_string().contains(secret),
                "secret in error display"
            );
            assert!(
                !format!("{error:?}").contains(secret),
                "secret in error debug"
            );
        }
        link = error.source();
    }
}
