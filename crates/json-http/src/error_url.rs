//! Removes credential URL components before retaining transport failures.

use crate::Error;

pub(crate) fn sanitize_reqwest(mut source: reqwest::Error) -> reqwest::Error {
    if let Some(url) = source.url_mut() {
        url.set_query(None);
        url.set_fragment(None);
        let _ = url.set_username("");
        let _ = url.set_password(None);
    }
    source
}

impl Error {
    pub(crate) fn sanitize_urls(self) -> Self {
        match self {
            Self::ResponseContext { headers, source } => Self::ResponseContext {
                headers,
                source: Box::new(source.sanitize_urls()),
            },
            Self::ReqwestTransport { source } => Self::ReqwestTransport {
                source: sanitize_reqwest(source),
            },
            Self::SseTransport {
                events_received,
                source,
            } => Self::SseTransport {
                events_received,
                source: sanitize_reqwest(source),
            },
            source => source,
        }
    }
}
