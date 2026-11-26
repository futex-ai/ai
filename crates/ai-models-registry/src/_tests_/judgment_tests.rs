//! Registry judgment uses the supplied spec, client, and provider settings.

use std::{collections::BTreeMap, time::Duration};

#[cfg(feature = "openai")]
use ai_interface::JudgmentAnswer;
use ai_interface::{JudgmentQuestion, JudgmentRequest};
use serde_json::json;

use crate::{ProviderConfig, Registry};

use super::media_support::bytes_client;
#[cfg(feature = "typesafe")]
use super::support::spec;

#[cfg(feature = "typesafe")]
#[tokio::test]
async fn judgment_factory_forwards_specs_and_provider_settings() {
    let (http, requests) = bytes_client(
        json!({"model":"provider-version","answers":{"urgent":{"type":"noul","noul":0.9}}}),
    );
    let spec = spec("typesafe");
    let registry = Registry::new(http).with_provider(
        ProviderConfig::new(spec.provider.clone(), "registry-key")
            .with_base_url("https://proxy.test/team/")
            .with_header("X-Tenant", "tenant")
            .with_total_timeout(Duration::from_secs(31)),
    );
    let built = registry.judgment(&spec);
    assert!(built.is_ok());

    let response = built
        .unwrap()
        .judge(&JudgmentRequest {
            input: "A ticket is delayed".into(),
            questions: BTreeMap::from([(
                "urgent".into(),
                JudgmentQuestion::condition("Is it urgent?", None),
            )]),
        })
        .await
        .unwrap();

    assert_eq!(response.provider, "typesafe");
    assert_eq!(response.model_id, "unlisted-model");
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "https://proxy.test/team/v1/systemone");
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer registry-key")
    );
    assert_eq!(
        requests[0].headers.get("X-Tenant").map(String::as_str),
        Some("tenant")
    );
    assert_eq!(requests[0].timeout, Duration::from_secs(31));
    let body: serde_json::Value =
        serde_json::from_slice(requests[0].body.as_ref().unwrap().as_json_bytes().unwrap())
            .unwrap();
    assert_eq!(body["model"], "unlisted-model");
}

#[cfg(feature = "openai")]
#[tokio::test]
async fn openai_judgment_factory_uses_its_spec_and_provider_config() {
    let (http, requests) = bytes_client(json!({
        "model":"provider-version","answers":[{"name":"q0","type":"predicate","probability":0.9}]
    }));
    let spec = ai_models_openai::GPT_6_LUNA;
    let registry = Registry::new(http).with_provider(
        ProviderConfig::new(spec.provider.clone(), "fixture-registry-key")
            .with_base_url("https://proxy.example.test/project/")
            .with_header("X-Tenant", "fixture-tenant")
            .with_total_timeout(Duration::from_secs(31)),
    );
    let built = registry.judgment(&spec);
    assert!(built.is_ok(), "{:?}", built.as_ref().err());

    let response = built
        .unwrap()
        .judge(&JudgmentRequest {
            input: "A ticket is delayed".into(),
            questions: BTreeMap::from([(
                "urgent".into(),
                JudgmentQuestion::condition("Is it urgent?", None),
            )]),
        })
        .await
        .unwrap();

    assert_eq!(response.provider, "openai");
    assert_eq!(response.model_id, spec.provider_model_id);
    assert_eq!(response.resolved_model_id, "provider-version");
    assert_eq!(
        response.answers["urgent"],
        JudgmentAnswer::Condition { probability: 0.9 }
    );
    let requests = requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].url,
        "https://proxy.example.test/project/v1/decisions"
    );
    assert_eq!(requests[0].timeout, Duration::from_secs(31));
    assert_eq!(
        requests[0].headers.get("Authorization").map(String::as_str),
        Some("Bearer fixture-registry-key")
    );
    assert_eq!(
        requests[0].headers.get("X-Tenant").map(String::as_str),
        Some("fixture-tenant")
    );
    let body: serde_json::Value =
        serde_json::from_slice(requests[0].body.as_ref().unwrap().as_json_bytes().unwrap())
            .unwrap();
    assert_eq!(body["model"], spec.provider_model_id.as_ref());
}
