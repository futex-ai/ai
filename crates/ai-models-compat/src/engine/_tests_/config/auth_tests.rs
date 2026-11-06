//! Config hooks and pre-dispatch credential redaction through the model API.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ConversationMessage, ConversationPart, ErrorKind, Model, ModelControl};
use ai_models_core::{
    ProviderConfig,
    test_support::{fixtures_for_buffered_responses, recording_streaming_client},
};
use json_http::{JsonHttpAuthMock, TransportBackedJsonHttpClient};
use unimock::{MockFn, Unimock, matching};

use crate::{
    deepseek::{DEEPSEEK_V4_PRO, DeepSeekModel, PROVIDER_ID},
    engine::error_support::request,
    kimi::KIMI_K3,
};

use super::support::{model, response};

#[tokio::test]
async fn config_auth_hook_overrides_extras_and_failure_stays_transient() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Auth".to_owned(), "hook-key".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("private hook failure")))
            .once();
    }));
    let config = ProviderConfig::with_auth(KIMI_K3.provider.clone(), Arc::new(auth))
        .with_base_url("https://gateway.test/prefix")
        .with_header("X-Custom-Auth", "hook-key")
        .with_header("X-Route", "proxy");
    let (http, requests) =
        recording_streaming_client(fixtures_for_buffered_responses(vec![response()]));
    let model = model(http, KIMI_K3, config);

    let success = model.complete(&request()).await.unwrap();
    let failure = model.complete(&request()).await.unwrap_err();

    assert_eq!(success.text(), "Done");
    assert_eq!(failure.core().kind, ErrorKind::Transient);
    assert_eq!(failure.core().message, "authentication hook failed");
    assert!(!format!("{failure:?}").contains("private hook failure"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].headers.get("X-Custom-Auth").map(String::as_str),
        Some("hook-key")
    );
    assert_eq!(
        requests[0].headers.get("X-Route").map(String::as_str),
        Some("proxy")
    );
    assert!(requests[0].auth_headers.contains("x-custom-auth"));
    assert_eq!(
        requests[0]
            .headers
            .keys()
            .filter(|name| name.eq_ignore_ascii_case("authorization"))
            .count(),
        0
    );
}

#[tokio::test]
async fn config_secrets_are_captured_before_controls_and_input_validation() {
    for (control_failure, key, extra, expected_kind, message) in [
        (
            false,
            "Scheme DeepSeek",
            "Gateway accepts",
            ErrorKind::UnsupportedInput,
            "[redacted] [redacted] plain text messages only",
        ),
        (
            true,
            "Scheme StopSequences",
            "Gateway unsupported",
            ErrorKind::UnsupportedControl(ModelControl::StopSequences),
            "[redacted] [redacted] control",
        ),
    ] {
        let http = Arc::new(TransportBackedJsonHttpClient::new(Arc::new(Unimock::new(
            (),
        ))));
        let config = ProviderConfig::new(PROVIDER_ID, key).with_header("X-Private", extra);
        let mut spec = DEEPSEEK_V4_PRO;
        spec.supports_stop_sequences = false;
        let model = DeepSeekModel::from_config(http, spec, config);
        let mut input = request();
        if control_failure {
            input.controls.generation.stop_sequences = vec!["stop".to_owned()];
        } else {
            input.messages = vec![ConversationMessage::user_with_parts(vec![
                ConversationPart::Image {
                    mime_type: "image/png".to_owned(),
                    data_base64: "aW1hZ2U=".to_owned(),
                    provider_state: None,
                },
            ])];
        }

        let error = model.complete(&input).await.unwrap_err();

        assert_eq!(error.core().kind, expected_kind);
        assert_eq!(error.core().message, message);
    }
}
