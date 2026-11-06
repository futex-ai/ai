//! Provider-neutral credential selection for adapter configurations.

use std::{collections::BTreeMap, fmt, sync::Arc};

use json_http::{DynJsonHttpAuth, StaticHeaderAuth};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// The wire scheme that an adapter uses for a configured API key.
pub enum ApiKeyScheme {
    /// Sends `Authorization: Bearer <key>`.
    Bearer,
    /// Sends the key in the named header.
    Header(&'static str),
}

#[derive(Clone)]
pub(crate) enum ProviderAuth {
    ApiKey(String),
    Hook(DynJsonHttpAuth),
}

impl ProviderAuth {
    pub(crate) fn auth(&self, scheme: ApiKeyScheme) -> DynJsonHttpAuth {
        match self {
            Self::Hook(auth) => auth.clone(),
            Self::ApiKey(key) => Arc::new(match scheme {
                ApiKeyScheme::Bearer => StaticHeaderAuth::bearer_token(key),
                ApiKeyScheme::Header(name) => {
                    StaticHeaderAuth::new(BTreeMap::from([(name.to_owned(), key.clone())]))
                }
            }),
        }
    }

    pub(crate) fn api_key(&self) -> Option<&str> {
        match self {
            Self::ApiKey(key) => Some(key),
            Self::Hook(_) => None,
        }
    }
}

impl fmt::Debug for ProviderAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKey(_) => formatter.write_str("ApiKey([redacted])"),
            Self::Hook(_) => formatter.write_str("Hook"),
        }
    }
}
