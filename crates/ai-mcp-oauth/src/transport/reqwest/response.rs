//! Size-bounded OAuth response decoding.

use std::collections::BTreeMap;

use futures_util::StreamExt;
use serde_json::Value;

use crate::{Error, OAuthEndpointKind, OAuthHttpResponse, Result};

pub(super) async fn bounded_response(
    response: reqwest::Response,
    limit: usize,
    endpoint: OAuthEndpointKind,
) -> Result<OAuthHttpResponse> {
    let status = response.status().as_u16();
    let headers = normalized_headers(response.headers());
    if endpoint == OAuthEndpointKind::Revocation && (200..300).contains(&status) {
        return Ok(OAuthHttpResponse {
            status,
            headers,
            body: Value::Null,
        });
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(_) => return Err(Error::Transport),
        };
        if bytes.len().saturating_add(chunk.len()) > limit {
            return Err(Error::ResponseTooLarge { limit_bytes: limit });
        }
        bytes.extend_from_slice(&chunk);
    }
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        match serde_json::from_slice(&bytes) {
            Ok(body) => body,
            Err(_)
                if !(200..300).contains(&status) || endpoint == OAuthEndpointKind::Revocation =>
            {
                Value::Null
            }
            Err(_) => return Err(Error::InvalidJsonResponse),
        }
    };
    Ok(OAuthHttpResponse {
        status,
        headers,
        body,
    })
}

fn normalized_headers(headers: &reqwest::header::HeaderMap) -> BTreeMap<String, Vec<String>> {
    let mut normalized = BTreeMap::new();
    for name in headers.keys() {
        let values = headers
            .get_all(name)
            .iter()
            .filter_map(|value| value.to_str().ok().map(str::to_owned))
            .collect::<Vec<_>>();
        normalized.insert(name.as_str().to_ascii_lowercase(), values);
    }
    normalized
}
