//! Mocked retry collaborators and literal schedules.

use std::{
    num::NonZeroUsize,
    sync::{Arc, Mutex},
    time::Duration,
};

use ai_interface::{ErrorKind, ModelControl};
use unimock::{MockFn, Unimock, matching};

use crate::{DynRetryRandom, DynSleeper, RetryConfig, RetryRandomMock, SleeperMock};

pub(super) const KINDS: [(ErrorKind, bool, bool); 13] = [
    (ErrorKind::InvalidRequest, false, false),
    (ErrorKind::RejectedKey, true, false),
    (ErrorKind::UnknownModel, true, false),
    (ErrorKind::UnsupportedInput, true, false),
    (ErrorKind::InvalidOutput, true, false),
    (ErrorKind::ContentPolicy, false, false),
    (ErrorKind::RateLimited, true, true),
    (ErrorKind::Transient, true, true),
    (ErrorKind::Interrupted, true, false),
    (ErrorKind::ContextLimitExceeded, true, false),
    (
        ErrorKind::UnsupportedControl(ModelControl::ToolChoice),
        true,
        false,
    ),
    (ErrorKind::TimedOut, true, false),
    (ErrorKind::Internal, false, false),
];

pub(super) fn config(base_delays: Vec<Duration>) -> RetryConfig {
    RetryConfig {
        max_attempts: NonZeroUsize::new(base_delays.len() + 1).unwrap(),
        base_delays,
        max_wait: Duration::from_secs(30),
    }
}

pub(super) fn full_random() -> DynRetryRandom {
    Arc::new(Unimock::new(
        RetryRandomMock::between
            .some_call(matching!(_, _))
            .answers(&|_, _, maximum| maximum),
    ))
}

pub(super) fn recording_sleeper(waits: Arc<Mutex<Vec<Duration>>>) -> DynSleeper {
    Arc::new(Unimock::new(
        SleeperMock::sleep
            .some_call(matching!(_))
            .answers_arc(Arc::new(move |_, wait| waits.lock().unwrap().push(wait))),
    ))
}
