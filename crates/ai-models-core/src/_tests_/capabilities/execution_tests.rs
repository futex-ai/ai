//! Completion lifecycle resolution keeps the call deadline.

use std::time::Duration;

use ai_interface::{ModelCompletionMode, ModelControl};

use super::support::{request, resolve, spec, unsupported};

#[test]
fn completion_mode_resolves_to_supported_lifecycle_and_keeps_timeout() {
    let mut request = request();
    request.controls.execution.total_timeout = Some(Duration::from_secs(42));
    request.controls.execution.completion_mode = ModelCompletionMode::RequireDeferred;
    let mut spec = spec();

    assert_eq!(
        resolve(&request, &spec),
        Err(unsupported(ModelControl::CompletionMode))
    );

    for (mode, supported, expected) in [
        (ModelCompletionMode::Synchronous, false, false),
        (ModelCompletionMode::Synchronous, true, false),
        (ModelCompletionMode::PreferDeferred, false, false),
        (ModelCompletionMode::PreferDeferred, true, true),
        (ModelCompletionMode::RequireDeferred, true, true),
    ] {
        request.controls.execution.completion_mode = mode;
        spec.supports_deferred = supported;

        assert_eq!(
            resolve(&request, &spec).map(|controls| (controls.deferred, controls.total_timeout)),
            Ok((expected, Some(Duration::from_secs(42)))),
            "completion mode: {mode:?}, deferred support: {supported}"
        );
    }
}
