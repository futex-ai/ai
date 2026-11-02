//! Shared provider settings without provider names or default endpoints.

use std::{collections::BTreeMap, fmt, time::Duration};

use ai_interface::ProviderId;
use json_http::{DynJsonHttpAuth, JsonHttpRequestBuilder};

use crate::{ApiKeyScheme, ErrorContext, provider_auth::ProviderAuth};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
/// Optional overrides for the timeouts supported by each adapter.
pub struct ProviderTimeouts {
    /// Maximum interval between stream events.
    pub idle_timeout: Option<Duration>,
    /// Total duration of one logical operation.
    pub total_timeout: Option<Duration>,
    /// Interval between polls of a generation job.
    pub poll_interval: Option<Duration>,
}

#[derive(Clone)]
/// Settings shared by all adapters of one provider.
pub struct ProviderConfig {
    /// Stable provider identity.
    pub provider: ProviderId,
    /// Base URL override, including an optional path prefix.
    pub base_url: Option<String>,
    /// Additional request headers with lower precedence than adapter and auth headers.
    pub extra_headers: BTreeMap<String, String>,
    /// Timeouts that replace the defaults of the adapter.
    pub timeouts: ProviderTimeouts,
    auth: ProviderAuth,
}

impl ProviderConfig {
    /// Builds a config with an explicit API key.
    pub fn new(provider: ProviderId, api_key: impl Into<String>) -> Self {
        Self {
            provider,
            base_url: None,
            extra_headers: BTreeMap::new(),
            timeouts: ProviderTimeouts::default(),
            auth: ProviderAuth::ApiKey(api_key.into()),
        }
    }

    /// Builds a config with an injected authentication hook.
    pub fn with_auth(provider: ProviderId, auth: DynJsonHttpAuth) -> Self {
        Self {
            provider,
            base_url: None,
            extra_headers: BTreeMap::new(),
            timeouts: ProviderTimeouts::default(),
            auth: ProviderAuth::Hook(auth),
        }
    }

    /// Sets the base URL for every adapter of this provider.
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// Adds an extra header with a case-insensitive name.
    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        let name = name.into();
        self.extra_headers
            .retain(|key, _| !key.eq_ignore_ascii_case(&name));
        self.extra_headers.insert(name, value.into());
        self
    }

    /// Adds extra headers with case-insensitive names.
    pub fn with_headers(mut self, headers: BTreeMap<String, String>) -> Self {
        for (name, value) in headers {
            self = self.with_header(name, value);
        }
        self
    }

    /// Sets the maximum gap between stream events.
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.timeouts.idle_timeout = Some(timeout);
        self
    }

    /// Sets the total timeout for each adapter operation.
    pub fn with_total_timeout(mut self, timeout: Duration) -> Self {
        self.timeouts.total_timeout = Some(timeout);
        self
    }

    /// Sets the interval between generation-job polls.
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.timeouts.poll_interval = Some(interval);
        self
    }

    /// Appends an adapter path while preserving the configured base path prefix.
    pub fn endpoint(&self, default_base: &str, fixed_path: &str) -> String {
        let base = self
            .base_url
            .as_deref()
            .unwrap_or(default_base)
            .trim_end_matches('/');
        let path = fixed_path.trim_start_matches('/');
        if path.is_empty() {
            base.to_owned()
        } else {
            format!("{base}/{path}")
        }
    }

    /// Selects the adapter's API-key scheme, or returns the configured hook.
    pub fn auth(&self, scheme: ApiKeyScheme) -> DynJsonHttpAuth {
        self.auth.auth(scheme)
    }

    /// Captures configured secrets before local request validation starts.
    pub fn capture_secrets(&self, context: &ErrorContext) {
        for value in self
            .auth
            .api_key()
            .into_iter()
            .chain(self.extra_headers.values().map(String::as_str))
        {
            context.add_secret(value);
            if let Some(token) = value.split_whitespace().last() {
                context.add_secret(token);
            }
        }
    }

    /// Adds extra headers without replacing adapter or authentication headers.
    pub fn apply_headers(&self, builder: JsonHttpRequestBuilder) -> JsonHttpRequestBuilder {
        builder.default_headers(self.extra_headers.clone())
    }
}

impl fmt::Debug for ProviderConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProviderConfig")
            .field("provider", &self.provider)
            .field("base_url", &self.base_url.as_ref().map(|_| "[configured]"))
            .field(
                "extra_headers",
                &self.extra_headers.keys().collect::<Vec<_>>(),
            )
            .field("timeouts", &self.timeouts)
            .field("auth", &self.auth)
            .finish()
    }
}

#[cfg(test)]
#[path = "_tests_/provider_config_tests.rs"]
mod provider_config_tests;
