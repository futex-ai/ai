//! Fixed response facts and clocks for credential-free Google boundary tests.

use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use ai_models_core::{Clock, ClockMock};
use json_http::JsonHttpHeaders;
use unimock::{MockFn, Unimock, matching};

pub(super) fn headers(id: &str, retry: &str) -> JsonHttpHeaders {
    JsonHttpHeaders::from_iter([
        ("x-goog-request-id".parse().unwrap(), id.parse().unwrap()),
        (
            "x-request-id".parse().unwrap(),
            "secondary-x-id".parse().unwrap(),
        ),
        (
            "request-id".parse().unwrap(),
            "secondary-id".parse().unwrap(),
        ),
        ("retry-after".parse().unwrap(), retry.parse().unwrap()),
    ])
}

pub(super) fn frozen_clock() -> Arc<dyn Clock> {
    Arc::new(Unimock::new(ClockMock::now.each_call(matching!()).returns(
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_445_412_480),
    )))
}

pub(super) fn unused_clock() -> Arc<dyn Clock> {
    Arc::new(Unimock::new(()))
}
