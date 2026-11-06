//! Response header collection shared by buffered and streaming responses.

use std::fmt;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

/// Case-insensitive response headers whose diagnostics contain names only.
#[derive(Clone, Default, Eq, PartialEq)]
pub struct JsonHttpHeaders(HeaderMap);

impl JsonHttpHeaders {
    /// Creates an empty header collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Explicitly borrows raw headers, including repeated and binary values.
    pub fn as_raw(&self) -> &HeaderMap {
        &self.0
    }

    /// Explicitly borrows raw headers for insertion or replacement of values.
    pub fn as_raw_mut(&mut self) -> &mut HeaderMap {
        &mut self.0
    }

    /// Reports whether this collection contains no headers.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl From<HeaderMap> for JsonHttpHeaders {
    fn from(headers: HeaderMap) -> Self {
        Self(headers)
    }
}

impl FromIterator<(HeaderName, HeaderValue)> for JsonHttpHeaders {
    fn from_iter<T: IntoIterator<Item = (HeaderName, HeaderValue)>>(iter: T) -> Self {
        Self(HeaderMap::from_iter(iter))
    }
}

impl fmt::Debug for JsonHttpHeaders {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_list().entries(self.0.keys()).finish()
    }
}
