//! TypeSafe judgment model construction tests.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use ai_interface::JudgmentModel;
use json_http::{JsonHttpBody, StaticHeaderAuth};

use super::{
    TypeSafeJudgmentModel,
    test_support::{
        recording_http_client, simple_request, successful_response, unused_http_client,
    },
};

#[test]
fn constructors_use_the_documented_defaults_and_clone() {
    let keyed = TypeSafeJudgmentModel::new(unused_http_client(), "jev-latest", "typesafe-key");

    assert_eq!(keyed.model_id, "jev-latest");
    assert_eq!(keyed.endpoint, "https://api.typesafe.ai/v1/systemone");
    assert_eq!(keyed.timeout, Duration::from_secs(60));

    let authenticated = TypeSafeJudgmentModel::with_auth(
        unused_http_client(),
        "jev-1.13.0",
        Arc::new(StaticHeaderAuth::default()),
    );
    assert_eq!(authenticated.model_id, "jev-1.13.0");
    let _clone = authenticated.clone();
}

#[tokio::test]
async fn auth_endpoint_timeout_and_unlisted_model_are_passed_through() {
    let (http_client, requests) = recording_http_client(successful_response());
    let auth = Arc::new(StaticHeaderAuth::new(BTreeMap::from([(
        "X-Test-Auth".to_owned(),
        "injected".to_owned(),
    )])));
    let model = TypeSafeJudgmentModel::with_auth(http_client, "jev-1.14.2", auth)
        .with_endpoint("https://typesafe.test/judge")
        .with_timeout(Duration::from_secs(7));

    model
        .judge(&simple_request())
        .await
        .expect("unlisted provider model should be accepted");

    let requests = requests.lock().expect("request lock should be valid");
    assert_eq!(requests[0].url, "https://typesafe.test/judge");
    assert_eq!(requests[0].timeout, Duration::from_secs(7));
    assert_eq!(
        requests[0].headers.get("X-Test-Auth").map(String::as_str),
        Some("injected")
    );
    assert_eq!(
        requests[0]
            .body
            .as_ref()
            .and_then(JsonHttpBody::as_json)
            .expect("request body should be JSON")["model"],
        "jev-1.14.2"
    );
}
