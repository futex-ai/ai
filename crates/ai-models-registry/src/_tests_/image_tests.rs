//! Registry image factories use the injected client and provider configuration.

use std::time::Duration;

use ai_interface::ImageGenerationRequest;
use serde_json::json;

use crate::{ProviderConfig, Registry};

use super::{media_support::json_client, support::spec};

#[tokio::test]
async fn image_factories_forward_specs_and_provider_settings() {
    let cases = [
        #[cfg(feature = "google")]
        (
            "google",
            "/v1/models/unlisted-model:generateContent",
            "x-goog-api-key",
            "registry-key",
            json!({"candidates":[{"content":{"parts":[{"inlineData":{"mimeType":"image/png","data":"AQ=="}}]}}]}),
        ),
        #[cfg(feature = "openai")]
        (
            "openai",
            "/v1/images/generations",
            "Authorization",
            "Bearer registry-key",
            json!({"data":[{"b64_json":"AQ=="}],"output_format":"png"}),
        ),
    ];
    for (provider, path, auth_header, auth_value, body) in cases {
        let (http, requests) = json_client(body);
        let spec = spec(provider);
        let registry = Registry::new(http).with_provider(
            ProviderConfig::new(spec.provider.clone(), "registry-key")
                .with_base_url("https://proxy.test/team/")
                .with_header("X-Tenant", "tenant")
                .with_total_timeout(Duration::from_secs(31)),
        );
        let built = registry.image(&spec);
        assert!(built.is_ok(), "{provider}");

        let response = built
            .unwrap()
            .generate(&ImageGenerationRequest {
                prompt: "Draw a bird".into(),
                ..Default::default()
            })
            .await
            .unwrap();

        assert_eq!(response.provider, provider);
        assert_eq!(response.model_id, "unlisted-model");
        assert_eq!(response.image.data, vec![1]);
        let requests = requests.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url, format!("https://proxy.test/team{path}"));
        assert_eq!(
            requests[0].headers.get(auth_header).map(String::as_str),
            Some(auth_value)
        );
        assert_eq!(
            requests[0].headers.get("X-Tenant").map(String::as_str),
            Some("tenant")
        );
        assert_eq!(requests[0].timeout, Duration::from_secs(31));
    }
}
