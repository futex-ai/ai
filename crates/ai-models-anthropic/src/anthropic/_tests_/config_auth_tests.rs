//! Configured Anthropic hooks remain fresh, tracked, and secret-safe.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ErrorKind, Model};
use ai_models_core::ProviderConfig;
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{AnthropicModel, PROVIDER_ID};

use super::{
    config_tests::{header_values, request},
    stream_support::{client_for_buffered_bodies, custom_spec},
};

#[tokio::test]
async fn config_hooks_run_once_per_call_override_extras_and_fail_safely() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".to_owned(), "Scheme first-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Custom-Key".to_owned(), "Scheme second-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("undiscovered-secret")))
            .once();
    }));
    let (http, requests) = client_for_buffered_bodies(vec![
        json!({"stop_reason":"end_turn", "content":[]}),
        json!({"stop_reason":"end_turn", "content":[]}),
    ]);
    let config = ProviderConfig::with_auth(PROVIDER_ID, Arc::new(auth))
        .with_header("x-custom-key", "wrong-key")
        .with_header("ANTHROPIC-VERSION", "wrong-version")
        .with_header("X-Tenant", "tenant-value");
    let model = AnthropicModel::from_config(http, custom_spec("claude-configured"), config);
    model.complete(&request()).await.unwrap();
    model.complete(&request()).await.unwrap();
    let error = model.complete(&request()).await.unwrap_err();
    assert_eq!(error.core().kind, ErrorKind::Transient);
    assert_eq!(error.core().message, "authentication hook failed");
    assert!(!error.to_string().contains("undiscovered-secret"));
    assert!(!format!("{error:?}").contains("undiscovered-secret"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for (request, expected) in requests
        .iter()
        .zip(["Scheme first-secret", "Scheme second-secret"])
    {
        assert_eq!(header_values(request, "x-custom-key"), [expected]);
        assert_eq!(header_values(request, "anthropic-version"), ["2023-06-01"]);
        assert_eq!(
            request.headers.get("X-Tenant").map(String::as_str),
            Some("tenant-value")
        );
        assert!(request.auth_headers.contains("x-custom-key"));
    }
}
