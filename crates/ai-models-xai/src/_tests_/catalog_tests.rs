use ai_models_core::ModelFeature;

use super::{GROK_4_20, GROK_4_20_REASONING, GROK_4_20_THINKING_HIGH, known_models};

#[test]
fn grok_primary_models_advertise_vision() {
    let models = known_models();
    for model_id in [GROK_4_20_REASONING, GROK_4_20, GROK_4_20_THINKING_HIGH] {
        let model = models
            .iter()
            .find(|model| model.id == model_id)
            .expect("model should exist");
        assert!(model.has_feature(ModelFeature::Vision));
    }
}
