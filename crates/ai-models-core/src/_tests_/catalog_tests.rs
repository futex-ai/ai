//! Catalog identity, routing facts, and per-request effort resolution.

use ai_interface::{
    Effort, EffortSet, MOCK_CUSTOM_DEFAULTS, ModelCatalog, ModelControl, ModelError, ModelFeature,
    ModelRequest, ModelSpec, ProviderId,
};

use crate::{CostTier, IntelligenceScore, SpeedTier, known_mock_models, resolve_controls};

#[test]
fn mock_catalog_contains_valid_development_model() {
    let catalog = ModelCatalog::new().with_models(known_mock_models());
    let model = catalog
        .find("mock/mock")
        .expect("mock model should be present");
    let routing = model.routing.expect("mock model should have routing tiers");

    assert_eq!(routing.intelligence.value(), 5);
    assert_eq!(model.provider_model_id, "mock");
    assert_eq!(model.default_effort, None);
    assert!(model.context_window_tokens > 0);
    assert!(model.has_feature(ModelFeature::ToolCalling));
    assert_eq!(model.max_output_tokens, 128_000);
    assert_eq!(routing.speed, SpeedTier::VeryFast);
    assert_eq!(routing.cost, CostTier::Low);
}

#[test]
fn intelligence_scores_are_strongly_typed_one_to_ten() {
    assert_eq!(IntelligenceScore::One.value(), 1);
    assert_eq!(IntelligenceScore::Ten.value(), 10);
}

#[test]
fn one_catalog_model_supports_distinct_request_efforts() {
    let spec = model(
        "model",
        EffortSet::new(Effort::Low).with(Effort::ExtraHigh),
        Effort::Low,
    );
    let catalog = ModelCatalog::new().with_models([spec]);
    let spec = catalog.find("provider-a/model").expect("model exists");
    let low = resolved_effort(spec, Effort::Low);
    let extra_high = resolved_effort(spec, Effort::ExtraHigh);

    assert_eq!(spec.provider_model_id, "model");
    assert_eq!(catalog.all().len(), 1);
    assert_ne!(low, extra_high);
    assert_eq!(low, Ok(Some(Effort::Low)));
    assert_eq!(extra_high, Ok(Some(Effort::ExtraHigh)));
}

#[test]
fn efforts_have_stable_log_values() {
    assert_eq!(Effort::Disabled.as_str(), "disabled");
    assert_eq!(Effort::Low.as_str(), "low");
    assert_eq!(Effort::Medium.as_str(), "medium");
    assert_eq!(Effort::High.as_str(), "high");
    assert_eq!(Effort::ExtraHigh.as_str(), "extra_high");
    assert_eq!(Effort::Max.as_str(), "max");
}

#[test]
fn resolves_to_the_highest_supported_effort_not_above_the_request() {
    let catalog = ModelCatalog::new().with_models([
        model(
            "model-a",
            EffortSet::new(Effort::Low)
                .with(Effort::High)
                .with(Effort::ExtraHigh),
            Effort::High,
        ),
        model("model-b", EffortSet::new(Effort::High), Effort::High),
    ]);
    let spec = catalog.find("provider-a/model-a").expect("model exists");

    assert_eq!(
        resolved_effort(spec, Effort::Max),
        Ok(Some(Effort::ExtraHigh))
    );
    assert_eq!(resolved_effort(spec, Effort::High), Ok(Some(Effort::High)));
    assert_eq!(resolved_effort(spec, Effort::Medium), Ok(Some(Effort::Low)));
    assert_eq!(
        resolved_effort(spec, Effort::Disabled),
        Err(ModelControl::Effort)
    );
    assert_eq!(catalog.find("provider-a/unknown"), None);
    assert_eq!(catalog.find("provider-b/model-a"), None);
}

fn model(model_id: &'static str, efforts: EffortSet, default_effort: Effort) -> ModelSpec {
    let mut spec = ModelSpec::custom(
        ProviderId::new("provider-a"),
        model_id,
        &MOCK_CUSTOM_DEFAULTS,
    );
    spec.efforts = efforts;
    spec.default_effort = Some(default_effort);
    spec
}

fn resolved_effort(spec: &ModelSpec, effort: Effort) -> Result<Option<Effort>, ModelControl> {
    let mut request = ModelRequest {
        system_prompt: String::new(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    };
    request.controls.generation.effort = Some(effort);
    resolve_controls(&request, spec)
        .map(|controls| controls.effort)
        .map_err(|error| match error {
            ModelError::UnsupportedControl { control, .. } => control,
            other => panic!("expected unsupported effort, received {other:?}"),
        })
}
