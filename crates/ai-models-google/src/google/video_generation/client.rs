//! Google video generation transport client.

use std::{sync::Arc, time::Duration};

use ai_interface::ModelSpec;
use ai_models_core::{
    Clock, DynPollingRuntime, ErrorContext, ProviderConfig, SystemClock, TokioPollingRuntime,
};
use json_http::{DynJsonHttpAuth, DynJsonHttpClient};

use crate::{
    PROVIDER_ID,
    google::{GOOGLE_API_BASE_URL, error_context},
};

const DEFAULT_POLL_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// Google-backed `ai_interface::VideoGenerator` implementation.
#[derive(Clone)]
pub struct GoogleVideoGenerator {
    pub(super) http_client: DynJsonHttpClient,
    pub(super) model_id: String,
    pub(super) config: ProviderConfig,
    pub(super) runtime: DynPollingRuntime,
    pub(super) base_url: String,
    pub(super) poll_interval: Duration,
    pub(super) timeout: Duration,
    clock: Arc<dyn Clock>,
}

impl GoogleVideoGenerator {
    /// Builds a Google video generator from shared provider settings.
    pub fn from_config(
        http_client: DynJsonHttpClient,
        spec: ModelSpec,
        config: ProviderConfig,
    ) -> Self {
        Self::with_config(
            http_client,
            spec.provider_model_id.into_owned(),
            config,
            Arc::new(TokioPollingRuntime),
        )
    }

    /// Builds a Google video generator from an injected client, model id, and API key.
    pub fn new(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self::with_config(
            http_client,
            model_id.into(),
            ProviderConfig::new(PROVIDER_ID, api_key),
            Arc::new(TokioPollingRuntime),
        )
    }

    /// Builds a Google video generator from an injected client and auth hook.
    pub fn with_auth(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
    ) -> Self {
        Self::with_auth_and_runtime(http_client, model_id, auth, Arc::new(TokioPollingRuntime))
    }

    /// Builds a Google video generator with an explicit polling runtime.
    pub fn with_auth_and_runtime(
        http_client: DynJsonHttpClient,
        model_id: impl Into<String>,
        auth: DynJsonHttpAuth,
        runtime: DynPollingRuntime,
    ) -> Self {
        Self::with_config(
            http_client,
            model_id.into(),
            ProviderConfig::with_auth(PROVIDER_ID, auth),
            runtime,
        )
    }

    fn with_config(
        http_client: DynJsonHttpClient,
        model_id: String,
        config: ProviderConfig,
        runtime: DynPollingRuntime,
    ) -> Self {
        Self {
            http_client,
            model_id,
            runtime,
            base_url: config.endpoint(GOOGLE_API_BASE_URL, "/v1beta"),
            poll_interval: config
                .timeouts
                .poll_interval
                .unwrap_or(DEFAULT_POLL_INTERVAL),
            timeout: config.timeouts.total_timeout.unwrap_or(DEFAULT_TIMEOUT),
            clock: Arc::new(SystemClock),
            config,
        }
    }

    /// Uses an injected wall clock for response retry dates.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub(super) fn context(&self) -> ErrorContext {
        let context = error_context::new(&self.model_id, self.clock.clone());
        self.config.capture_secrets(&context);
        context
    }

    /// Overrides the API base URL used for submission, polling, and downloads.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into().trim_end_matches('/').to_owned();
        self
    }

    /// Overrides polling interval and total generation timeout.
    pub fn with_polling(mut self, poll_interval: Duration, timeout: Duration) -> Self {
        self.poll_interval = poll_interval;
        self.timeout = timeout;
        self
    }
}
