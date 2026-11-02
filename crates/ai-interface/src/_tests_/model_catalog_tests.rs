//! Model catalog text ids preserve the complete provider model suffix.

use std::borrow::Cow;

use crate::{Effort, EffortSet, ForcedToolChoice, ModelCatalog, ModelSpec, ProviderId, Sampling};

#[test]
fn text_id_lookup_splits_once_and_matches_case_exactly() {
    let spec = ModelSpec {
        provider: ProviderId::new("provider-a"),
        provider_model_id: Cow::Borrowed("organization/model"),
        context_window_tokens: 128_000,
        max_output_tokens: 64_000,
        efforts: EffortSet::new(Effort::Disabled),
        default_effort: Effort::Disabled,
        forced_tool_choice: ForcedToolChoice::Always,
        sampling: Sampling::Adjustable,
        supports_stop_sequences: true,
        supports_deferred: false,
        requires_output_limit: false,
        replay_efforts: Cow::Borrowed(&[]),
        features: Cow::Borrowed(&[]),
        routing: None,
    };
    let catalog = ModelCatalog::new().with_models([spec.clone()]);

    assert_eq!(catalog.find("provider-a/organization/model"), Some(&spec));
    for text_id in [
        "provider-a/organization/other",
        "Provider-a/organization/model",
        "provider-a/Organization/model",
        "organization/model",
        "model",
        "provider-a/",
        "/organization/model",
    ] {
        assert_eq!(catalog.find(text_id), None, "text id: {text_id}");
    }
}
