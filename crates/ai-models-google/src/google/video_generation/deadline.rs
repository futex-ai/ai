//! One generation budget governs every HTTP phase and transport failure.

use std::time::{Duration, Instant};

use ai_interface::{ErrorKind, VideoGenerationError, VideoGenerationResult};
use ai_models_core::{ErrorContext, PollingRuntime};

use super::error::classify_request_error;

pub(super) struct VideoDeadline<'a> {
    runtime: &'a dyn PollingRuntime,
    started_at: Instant,
    timeout: Duration,
}

impl<'a> VideoDeadline<'a> {
    pub(super) fn new(runtime: &'a dyn PollingRuntime, timeout: Duration) -> Self {
        Self {
            runtime,
            started_at: runtime.now(),
            timeout,
        }
    }

    pub(super) fn remaining(&self, context: &ErrorContext) -> VideoGenerationResult<Duration> {
        let remaining = self.remaining_time();
        if remaining.is_zero() {
            return Err(VideoGenerationError::new(
                context.core(ErrorKind::TimedOut, "generation timed out"),
            ));
        }
        Ok(remaining)
    }

    pub(super) fn request_error(
        &self,
        context: &ErrorContext,
        source: json_http::Error,
    ) -> VideoGenerationError {
        let timed_out = source.is_transport_failure() && self.remaining_time().is_zero();
        let mut error = classify_request_error(context, source);
        if timed_out {
            error.core_mut().kind = ErrorKind::TimedOut;
            error.core_mut().message = "generation timed out".to_owned();
        }
        error
    }

    fn remaining_time(&self) -> Duration {
        self.timeout.saturating_sub(
            self.runtime
                .now()
                .saturating_duration_since(self.started_at),
        )
    }
}
