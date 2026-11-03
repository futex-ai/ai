//! Shared config URL, credential, and request-header contracts.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{ErrorKind, ProviderId};
use json_http::{
    JsonHttpAuthMock, JsonHttpClient, JsonHttpRequest, JsonHttpResponse, JsonHttpTransportMock,
    TransportBackedJsonHttpClient,
};
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{ApiKeyScheme, ErrorContext, ProviderConfig};

const PROVIDER: ProviderId = ProviderId::new("example");

#[test]
fn endpoint_preserves_prefix_and_ignores_boundary_slashes() {
    for base in [
        "https://gateway.test/prefix",
        "https://gateway.test/prefix/",
    ] {
        let config = ProviderConfig::new(PROVIDER, "api-secret").with_base_url(base);
        for path in ["/v1/messages?alt=sse", "v1/messages?alt=sse"] {
            assert_eq!(
                config.endpoint("https://default.test", path),
                "https://gateway.test/prefix/v1/messages?alt=sse",
                "base={base}, path={path}"
            );
        }
    }
    let config = ProviderConfig::new(PROVIDER, "api-secret");
    assert_eq!(
        config.endpoint("https://default.test/prefix/", "/v1/messages"),
        "https://default.test/prefix/v1/messages"
    );
    assert_eq!(
        config.endpoint("https://default.test/", ""),
        "https://default.test"
    );
}

#[test]
fn debug_and_local_errors_hide_all_configured_secret_values() {
    let config = ProviderConfig::new(PROVIDER, "Scheme api-secret")
        .with_header("x-gateway-key", "Gateway extra-secret")
        .with_base_url("https://gateway.test/prefix")
        .with_idle_timeout(Duration::from_secs(3))
        .with_total_timeout(Duration::from_secs(8))
        .with_poll_interval(Duration::from_secs(2));
    let debug = format!("{config:?}");
    assert!(!debug.contains("api-secret"), "{debug}");
    assert!(!debug.contains("extra-secret"), "{debug}");
    let context = context();
    config.capture_secrets(&context);
    assert_eq!(
        context
            .core(
                ErrorKind::InvalidRequest,
                "Scheme api-secret Gateway extra-secret api-secret extra-secret"
            )
            .message,
        "[redacted] [redacted] [redacted] [redacted]"
    );
}

#[test]
fn extra_headers_redact_full_values_without_hiding_common_short_words() {
    let config = ProviderConfig::new(PROVIDER, "api-secret").with_header("X-Title", "My App");
    let context = context();
    config.capture_secrets(&context);
    assert_eq!(
        context.redact("The App field rejected My App"),
        "The App field rejected [redacted]"
    );
}

#[tokio::test]
async fn config_applies_key_scheme_with_auth_and_adapter_header_precedence() {
    for (scheme, name, expected) in [
        (ApiKeyScheme::Bearer, "Authorization", "Bearer api-secret"),
        (ApiKeyScheme::Header("x-api-key"), "x-api-key", "api-secret"),
        (
            ApiKeyScheme::Header("x-goog-api-key"),
            "x-goog-api-key",
            "api-secret",
        ),
    ] {
        let config = ProviderConfig::new(PROVIDER, "api-secret")
            .with_header(name.to_uppercase(), "extra-auth")
            .with_header("CONTENT-TYPE", "extra-content")
            .with_header("X-Trace", "trace");
        let transport = Arc::new(Unimock::new(
            JsonHttpTransportMock::execute
                .some_call(matching!(_))
                .answers_arc(Arc::new(move |_, request: &JsonHttpRequest| {
                    let headers = request
                        .headers
                        .iter()
                        .map(|(name, value)| (name.to_ascii_lowercase(), value.as_str()))
                        .collect::<Vec<_>>();
                    assert_eq!(
                        headers
                            .iter()
                            .filter(|(key, _)| key == &name.to_ascii_lowercase())
                            .count(),
                        1
                    );
                    assert_eq!(
                        headers
                            .iter()
                            .find(|(key, _)| key == &name.to_ascii_lowercase())
                            .unwrap()
                            .1,
                        expected
                    );
                    assert_eq!(
                        headers
                            .iter()
                            .find(|(key, _)| key == "content-type")
                            .unwrap()
                            .1,
                        "application/json"
                    );
                    assert_eq!(
                        headers.iter().find(|(key, _)| key == "x-trace").unwrap().1,
                        "trace"
                    );
                    let debug = format!("{request:?}");
                    assert!(!debug.contains("api-secret"), "{debug}");
                    assert!(!debug.contains("extra-auth"), "{debug}");
                    Ok(JsonHttpResponse {
                        status: 200,
                        headers: Default::default(),
                        body: json!({"ok":true}),
                    })
                })),
        ));
        let client = TransportBackedJsonHttpClient::new(transport);
        let context = context();
        config
            .apply_headers(
                client
                    .post("https://gateway.test/v1/messages")
                    .header("Content-Type", "application/json"),
                "https://gateway.test",
            )
            .auth(context.capture_auth(config.auth(scheme)))
            .send_value()
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn config_hook_replaces_extra_header_and_failure_stays_transient() {
    let auth = Arc::new(Unimock::new(
        JsonHttpAuthMock::apply_headers
            .some_call(matching!(_))
            .answers_arc(Arc::new(|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Auth".to_owned(), "rotating-secret".to_owned());
                Err(json_http::Error::auth("unknown private hook secret"))
            })),
    ));
    let config = ProviderConfig::with_auth(PROVIDER, auth);
    let context = context();
    let mut headers = BTreeMap::new();
    let result = context
        .capture_auth(config.auth(ApiKeyScheme::Bearer))
        .apply_headers(&mut headers)
        .await;
    assert_eq!(
        headers.get("X-Custom-Auth").map(String::as_str),
        Some("rotating-secret")
    );
    let error = result.unwrap_err();
    assert_eq!(
        error.to_string(),
        "[json_http/error] auth error: authentication hook failed"
    );
    let core = context.request_error(&error);
    assert_eq!(core.kind, ErrorKind::Transient);
    assert_eq!(context.redact("rotating-secret"), "[redacted]");
}

fn context() -> ErrorContext {
    ErrorContext::new("example", "model", &[], Arc::new(Unimock::new(())))
}
