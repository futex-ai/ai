//! Safe error-chain diagnostics with explicit access to the private original cause.

use std::{error::Error, fmt};

/// Retains a typed cause separately from the redacted diagnostics used by reporters.
pub struct SafeErrorSource {
    raw_cause: Box<dyn Error + Send + Sync>,
    diagnostic: Diagnostic,
}

#[derive(Debug)]
struct Diagnostic {
    message: String,
    next: Option<Box<Diagnostic>>,
}

impl SafeErrorSource {
    /// Attaches already-redacted text for each source link without formatting the cause.
    pub fn new(source: impl Error + Send + Sync + 'static, redacted_links: Vec<String>) -> Self {
        let mut next = None;
        for message in redacted_links.into_iter().rev() {
            next = Some(Box::new(Diagnostic { message, next }));
        }
        Self {
            raw_cause: Box::new(source),
            diagnostic: next.map(|link| *link).unwrap_or_else(|| Diagnostic {
                message: "[redacted source]".to_owned(),
                next: None,
            }),
        }
    }

    /// Keeps a cause with fully redacted diagnostics when no credential context exists.
    pub fn redacted(source: impl Error + Send + Sync + 'static) -> Self {
        let mut links = vec!["[redacted source]".to_owned()];
        let mut link = source.source();
        while let Some(error) = link {
            links.push("[redacted source]".to_owned());
            link = error.source();
        }
        Self::new(source, links)
    }

    /// Exposes the original cause for explicit typed inspection, outside safe reporting.
    pub fn raw_cause(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.raw_cause.as_ref()
    }
}

impl fmt::Display for SafeErrorSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.diagnostic.fmt(formatter)
    }
}

impl fmt::Debug for SafeErrorSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("SafeErrorSource")
            .field(&self.diagnostic)
            .finish()
    }
}

impl Error for SafeErrorSource {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.diagnostic.source()
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for Diagnostic {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.next.as_deref().map(|link| link as &dyn Error)
    }
}
