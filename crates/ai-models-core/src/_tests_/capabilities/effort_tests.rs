//! Effort defaults and downward-only resolution.

use ai_interface::{Effort, EffortSet, ModelControl};

use super::support::{request, resolve, spec, unsupported};

#[test]
fn resolves_default_and_requested_efforts_without_raising() {
    let mut spec = spec();
    spec.efforts = EffortSet::new(Effort::Low).with(Effort::High);
    for (requested, expected) in [
        (None, Effort::High),
        (Some(Effort::Max), Effort::High),
        (Some(Effort::High), Effort::High),
        (Some(Effort::Medium), Effort::Low),
    ] {
        let mut request = request();
        request.controls.generation.effort = requested;

        assert_eq!(
            resolve(&request, &spec).map(|controls| controls.effort),
            Ok(Some(expected)),
            "requested effort: {requested:?}"
        );
    }
}

#[test]
fn rejects_effort_when_no_supported_effort_is_low_enough() {
    let mut spec = spec();
    spec.efforts = EffortSet::new(Effort::High);
    let mut request = request();
    request.controls.generation.effort = Some(Effort::Low);

    assert_eq!(
        resolve(&request, &spec),
        Err(unsupported(ModelControl::Effort))
    );
}
