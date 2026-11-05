//! Shared data and exact typed outcomes for capability tests.

use std::borrow::Cow;

use ai_interface::{
    Effort, EffortSet, ForcedToolChoice, ModelControl, ModelError, ModelRequest, ModelSpec,
    ProviderId, Sampling,
};

use crate::{ResolvedModelControls, resolve_controls};

pub(super) fn spec() -> ModelSpec {
    ModelSpec {
        provider: ProviderId::new("provider-a"),
        provider_model_id: Cow::Borrowed("organization/model"),
        context_window_tokens: 128_000,
        max_output_tokens: 64_000,
        efforts: EffortSet::new(Effort::Disabled).with(Effort::High),
        default_effort: Some(Effort::High),
        forced_tool_choice: ForcedToolChoice::Always,
        sampling: Sampling::Adjustable,
        supports_stop_sequences: true,
        supports_deferred: false,
        requires_output_limit: false,
        replay_efforts: Cow::Borrowed(&[]),
        features: Cow::Borrowed(&[]),
        routing: None,
    }
}

pub(super) fn request() -> ModelRequest {
    ModelRequest {
        system_prompt: String::new(),
        messages: Vec::new(),
        tools: Vec::new(),
        response_schema: None,
        controls: Default::default(),
    }
}

pub(super) fn resolve(
    request: &ModelRequest,
    spec: &ModelSpec,
) -> Result<ResolvedModelControls, (String, String, ModelControl)> {
    resolve_controls(request, spec).map_err(|error| match error {
        ModelError::UnsupportedControl {
            provider,
            model_id,
            control,
        } => (provider, model_id, control),
        other => panic!("expected an unsupported-control error, received {other:?}"),
    })
}

pub(super) fn unsupported(control: ModelControl) -> (String, String, ModelControl) {
    (
        "provider-a".to_owned(),
        "organization/model".to_owned(),
        control,
    )
}
