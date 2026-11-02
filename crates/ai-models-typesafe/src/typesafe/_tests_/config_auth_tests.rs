//! Configured TypeSafe auth is applied once and tracked for redirects.

use std::{collections::BTreeMap, sync::Arc};

use ai_interface::{ErrorKind, JudgmentModel};
use ai_models_core::ProviderConfig;
use json_http::JsonHttpAuthMock;
use serde_json::json;
use unimock::{MockFn, Unimock, matching};

use crate::{PROVIDER_ID, TypeSafeJudgmentModel};

use super::{
    config_tests::{header_values, spec},
    error_context_tests::headers,
    test_support::{json_response, scripted_http_client, simple_request},
};

#[tokio::test]
async fn config_hooks_refresh_once_track_all_headers_and_redact_provider_failures() {
    let auth = Unimock::new(JsonHttpAuthMock::apply_headers.stub(|calls| {
        calls
            .call(matching!(_))
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme first-secret".to_owned());
                headers.insert("X-Second".to_owned(), "other-first-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, headers: &mut BTreeMap<String, String>| {
                headers.insert("X-Auth".to_owned(), "Scheme second-secret".to_owned());
                headers.insert("X-Second".to_owned(), "other-second-secret".to_owned());
                Ok(())
            })
            .once()
            .then()
            .answers(&|_, _| Err(json_http::Error::auth("undiscovered-secret")))
            .once();
    }));
    let mut first = json_response(403, json!({"detail":"rejected first-secret extra-secret"}));
    first.headers = headers("req-first-secret", "3");
    let mut second = json_response(403, json!({"detail":"rejected second-secret extra-secret"}));
    second.headers = headers("req-second-secret", "3");
    let (http, requests) = scripted_http_client(vec![Ok(first), Ok(second)]);
    let config = ProviderConfig::with_auth(PROVIDER_ID, Arc::new(auth))
        .with_header("x-auth", "wrong-key")
        .with_header("X-Tenant", "extra-secret");
    let model = TypeSafeJudgmentModel::from_config(http, spec(), config)
        .with_clock(Arc::new(Unimock::new(())));
    let first = model.judge(&simple_request()).await.unwrap_err();
    let second = model.judge(&simple_request()).await.unwrap_err();
    let failed_hook = model.judge(&simple_request()).await.unwrap_err();
    for error in [&first, &second] {
        assert_eq!(error.core().kind, ErrorKind::RejectedKey);
        assert_eq!(error.core().message, "rejected [redacted] [redacted]");
        assert_eq!(
            error.core().provider_request_id.as_deref(),
            Some("req-[redacted]")
        );
        assert_eq!(
            error.core().retry_after,
            Some(std::time::Duration::from_secs(3))
        );
    }
    assert_eq!(failed_hook.core().kind, ErrorKind::Transient);
    assert_eq!(failed_hook.core().message, "authentication hook failed");
    assert!(!failed_hook.to_string().contains("undiscovered-secret"));
    assert!(!format!("{failed_hook:?}").contains("undiscovered-secret"));
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    for (request, value) in requests
        .iter()
        .zip(["Scheme first-secret", "Scheme second-secret"])
    {
        assert_eq!(header_values(request, "x-auth"), [value]);
        assert_eq!(
            request.headers.get("X-Tenant").map(String::as_str),
            Some("extra-secret")
        );
        assert!(request.auth_headers.contains("x-auth"));
        assert!(request.auth_headers.contains("x-second"));
        assert!(!request.auth_headers.contains("x-tenant"));
    }
}
