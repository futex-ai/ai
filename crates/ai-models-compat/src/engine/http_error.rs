//! Dialect HTTP diagnostics with shared classification and original source retention.

use ai_interface::ModelError;
use ai_models_core::{ErrorContext, classify_json_http_error, classify_json_http_stream_error};

use super::Dialect;

/// Keeps error-specific headers and the original source when an opening fails.
pub(super) fn classify(
    dialect: &Dialect,
    context: &ErrorContext,
    source: json_http::Error,
) -> ModelError {
    if let json_http::Error::HttpStatus {
        status,
        headers,
        body,
    } = source.cause()
    {
        let headers = source.response_headers().unwrap_or(headers);
        let error = match dialect.http_error {
            Some(classify) => classify(context, *status, headers, body),
            None => classify_json_http_error(context, *status, headers, body),
        };
        return ModelError::with_source(error.core().clone(), source);
    }
    classify_json_http_stream_error(context, 0, source)
}
