//! Tool-list freshness state tests.

use super::ToolListFreshness;

#[test]
fn starts_fresh_and_acknowledges_a_captured_invalidation() {
    let freshness = ToolListFreshness::default();
    assert!(!freshness.is_stale());

    freshness.invalidate();
    assert!(freshness.is_stale());
    let generation = freshness.capture();
    freshness.acknowledge(generation);

    assert!(!freshness.is_stale());
}

#[test]
fn invalidation_after_acknowledgement_is_stale() {
    let freshness = ToolListFreshness::default();
    let generation = freshness.capture();
    freshness.acknowledge(generation);

    freshness.invalidate();

    assert!(freshness.is_stale());
}

#[test]
fn older_acknowledgement_cannot_regress_a_newer_one() {
    let freshness = ToolListFreshness::default();
    freshness.invalidate();
    let older = freshness.capture();
    freshness.invalidate();
    let newer = freshness.capture();
    freshness.acknowledge(newer);

    freshness.acknowledge(older);

    assert!(!freshness.is_stale());
}

#[test]
fn acknowledging_a_capture_does_not_hide_a_later_invalidation() {
    let freshness = ToolListFreshness::default();
    let captured = freshness.capture();
    freshness.invalidate();

    freshness.acknowledge(captured);

    assert!(freshness.is_stale());
}
