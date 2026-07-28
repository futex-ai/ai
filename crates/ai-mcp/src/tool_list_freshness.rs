//! Monotonic tool-list invalidation state.

use std::sync::atomic::{AtomicU64, Ordering};

/// Tracks invalidations observed and covered by successful tool-list snapshots.
///
/// Every generation comes from a network-delivered event, so exhausting the
/// `u64` counter within one client lifetime is physically unreachable. Atomic
/// addition wraps by design if that assumption is ever exceeded.
#[derive(Default)]
pub(crate) struct ToolListFreshness {
    observed: AtomicU64,
    acknowledged: AtomicU64,
}

impl ToolListFreshness {
    /// Records one accepted tool-list invalidation.
    pub(crate) fn invalidate(&self) {
        self.observed.fetch_add(1, Ordering::SeqCst);
    }

    /// Captures the invalidations that a new snapshot can cover.
    pub(crate) fn capture(&self) -> u64 {
        self.observed.load(Ordering::SeqCst)
    }

    /// Monotonically acknowledges invalidations covered by a completed snapshot.
    pub(crate) fn acknowledge(&self, generation: u64) {
        self.acknowledged.fetch_max(generation, Ordering::SeqCst);
    }

    /// Reports whether an observed invalidation remains unacknowledged.
    pub(crate) fn is_stale(&self) -> bool {
        self.observed.load(Ordering::SeqCst) != self.acknowledged.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
#[path = "_tests_/tool_list_freshness_tests.rs"]
mod tool_list_freshness_tests;
