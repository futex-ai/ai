//! Response header collection shared by buffered and streaming responses.

/// Case-insensitive response headers, including repeated and binary values.
pub type JsonHttpHeaders = reqwest::header::HeaderMap;
