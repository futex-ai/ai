//! Portable exchange data, independent of reqwest and runtime clocks.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{JsonHttpHeaders, JsonHttpMethod};

use super::RecordedFailure;

/// Supported request method in the portable recording format.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum RecordedMethod {
    /// Read a resource.
    Get,
    /// Submit a resource.
    Post,
    /// Replace a resource.
    Put,
    /// Remove a resource.
    Delete,
    /// Update a resource.
    Patch,
}

impl From<JsonHttpMethod> for RecordedMethod {
    fn from(method: JsonHttpMethod) -> Self {
        match method {
            JsonHttpMethod::Get => Self::Get,
            JsonHttpMethod::Post => Self::Post,
            JsonHttpMethod::Put => Self::Put,
            JsonHttpMethod::Delete => Self::Delete,
            JsonHttpMethod::Patch => Self::Patch,
        }
    }
}

/// One request and its observed response.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Recording {
    /// Sanitized request after authentication.
    pub request: RecordedRequest,
    /// Buffered body or observed stream events.
    pub response: RecordedResponse,
}

/// A request with credentials removed.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RecordedRequest {
    /// Uppercase HTTP method.
    pub method: RecordedMethod,
    /// URL with credential parameters and user information removed.
    pub url: String,
    /// Headers, including redacted authentication headers.
    pub headers: BTreeMap<String, String>,
    /// JSON payload or multipart metadata.
    pub body: Option<RecordedBody>,
}

/// Request data that is safe to persist.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RecordedBody {
    /// Sanitized JSON body.
    Json(Value),
    /// Field metadata only. Uploaded bytes are never stored.
    Multipart(Vec<MultipartSummary>),
}

/// Metadata for one multipart field.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MultipartSummary {
    /// Form field name.
    pub name: String,
    /// Original uploaded filename, if present.
    pub filename: Option<String>,
    /// Declared content type.
    pub content_type: Option<String>,
    /// Original byte count.
    pub size: usize,
}

/// The observed result of one transport call.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordedResponse {
    /// A JSON response, including a rejected SSE opening.
    Json {
        /// HTTP response status.
        status: u16,
        /// Sanitized response headers. Older recordings omit this field.
        #[serde(
            default,
            with = "super::headers",
            skip_serializing_if = "JsonHttpHeaders::is_empty"
        )]
        headers: JsonHttpHeaders,
        /// Sanitized response payload.
        body: Value,
    },
    /// A buffered binary response.
    Bytes {
        /// HTTP response status.
        status: u16,
        /// Sanitized response headers. Older recordings omit this field.
        #[serde(
            default,
            with = "super::headers",
            skip_serializing_if = "JsonHttpHeaders::is_empty"
        )]
        headers: JsonHttpHeaders,
        /// Sanitized bytes in standard base64.
        body_base64: String,
    },
    /// A successfully opened SSE stream.
    Sse {
        /// Successful openings use 200 because the stream boundary has no status.
        status: u16,
        /// Sanitized response headers. Older recordings omit this field.
        #[serde(
            default,
            with = "super::headers",
            skip_serializing_if = "JsonHttpHeaders::is_empty"
        )]
        headers: JsonHttpHeaders,
        /// Events in consumption order.
        events: Vec<RecordedEvent>,
        /// Observed terminal state.
        end: StreamEnd,
    },
    /// A failure before a response was available.
    Failure {
        /// Sanitized response headers when headers arrived before this failure.
        #[serde(
            default,
            with = "super::optional_headers",
            skip_serializing_if = "Option::is_none"
        )]
        headers: Option<JsonHttpHeaders>,
        /// Failure class without raw error text or credentials.
        failure: RecordedFailure,
    },
}

/// One decoded SSE event.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RecordedEvent {
    /// Optional event name.
    pub event: Option<String>,
    /// Sanitized event data.
    pub data: String,
}

/// End of the observed stream. Unconsumed events are not recorded.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StreamEnd {
    /// No failure occurred while the caller consumed the stream.
    Done,
    /// The stream failed at the stated event count.
    Failure {
        /// Number of events emitted before failure.
        after_events: u64,
        /// Failure class and timing facts.
        failure: RecordedFailure,
    },
}
