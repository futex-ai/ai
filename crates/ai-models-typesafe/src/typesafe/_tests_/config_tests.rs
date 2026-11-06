//! Configured TypeSafe URLs, headers, timeouts, and local secret capture.

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use ai_interface::{
    ErrorKind, JudgmentError, JudgmentModel, JudgmentQuestion, JudgmentQuestionProblem,
    JudgmentRequest, ModelSpec,
};
use ai_models_core::ProviderConfig;
use unimock::Unimock;

use crate::{CUSTOM_DEFAULTS, PROVIDER_ID, TypeSafeJudgmentModel};

use super::test_support::{
    recording_http_client, simple_request, successful_response, unused_http_client,
};

#[tokio::test]
async fn config_judgment_uses_base_path_headers_auth_and_relevant_timeout() {
    for (base, timeout, expected_url, expected_timeout) in [
        (None, None, "https://api.typesafe.ai/v1/systemone", 60),
        (
            Some("https://proxy.test/team/"),
            Some(17),
            "https://proxy.test/team/v1/systemone",
            17,
        ),
        (
            Some("https://proxy.test/team"),
            Some(17),
            "https://proxy.test/team/v1/systemone",
            17,
        ),
    ] {
        let (http, requests) = recording_http_client(successful_response());
        let mut config = ProviderConfig::new(PROVIDER_ID, "config-key")
            .with_header("X-Tenant", "tenant-value")
            .with_header("AUTHORIZATION", "wrong-key")
            .with_idle_timeout(Duration::from_secs(1))
            .with_poll_interval(Duration::from_secs(1));
        config.base_url = base.map(str::to_owned);
        config.timeouts.total_timeout = timeout.map(Duration::from_secs);
        let model = TypeSafeJudgmentModel::from_config(http, spec(), config);
        let response = model.judge(&simple_request()).await.unwrap();
        assert_eq!(response.model_id, "jev-configured");
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        let request = &requests[0];
        assert_eq!(request.url, expected_url);
        assert_eq!(request.timeout, Duration::from_secs(expected_timeout));
        assert_eq!(request.idle_timeout, None);
        assert_eq!(
            request.headers.get("X-Tenant").map(String::as_str),
            Some("tenant-value")
        );
        assert_eq!(
            header_values(request, "authorization"),
            ["Bearer config-key"]
        );
        assert!(request.auth_headers.contains("authorization"));
    }
}

#[tokio::test]
async fn config_secrets_are_available_to_local_validation_before_dynamic_auth() {
    for (config, id, expected) in [
        (
            ProviderConfig::new(PROVIDER_ID, "local-secret"),
            "q-local-secret",
            "q-[redacted]",
        ),
        (
            ProviderConfig::with_auth(PROVIDER_ID, Arc::new(Unimock::new(())))
                .with_header("X-Credential", "extra-secret"),
            "q-extra-secret",
            "q-[redacted]",
        ),
    ] {
        let model = TypeSafeJudgmentModel::from_config(unused_http_client(), spec(), config);
        let error = model
            .judge(&JudgmentRequest {
                state: "ticket".into(),
                questions: BTreeMap::from([(
                    id.to_owned(),
                    JudgmentQuestion::Choice {
                        instructions: "Choose".into(),
                        options: BTreeMap::new(),
                    },
                )]),
            })
            .await
            .unwrap_err();
        assert_eq!(error.core().kind, ErrorKind::InvalidRequest);
        assert_eq!(error.core().provider, "typesafe");
        assert_eq!(error.core().model_id, "jev-configured");
        let JudgmentError::InvalidQuestion { id, problem, .. } = &error else {
            panic!("typed question error lost")
        };
        assert_eq!(id, expected);
        assert_eq!(problem, &JudgmentQuestionProblem::NoOptions);
        assert!(!format!("{error:?}").contains("local-secret"));
        assert!(!error.to_string().contains("local-secret"));
        assert!(!format!("{error:?}").contains("extra-secret"));
        assert!(!error.to_string().contains("extra-secret"));
    }
}

pub(super) fn spec() -> ModelSpec {
    ModelSpec::custom(PROVIDER_ID, "jev-configured", &CUSTOM_DEFAULTS)
}

pub(super) fn header_values<'a>(
    request: &'a json_http::JsonHttpRequest,
    header: &str,
) -> Vec<&'a str> {
    request
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case(header))
        .map(|(_, value)| value.as_str())
        .collect()
}
