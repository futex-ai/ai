//! Parsing and typed projection of HTTP Bearer challenges.

use std::collections::BTreeSet;

use url::Url;

#[derive(Clone, Debug, Eq, PartialEq)]
/// Actionable Bearer challenge details returned by an MCP server.
pub struct McpAuthorizationChallenge {
    /// Authorization outcome inferred from HTTP status and Bearer parameters.
    pub failure: McpAuthorizationFailure,
    /// Agreed, syntactically valid RFC 9728 metadata URL when advertised.
    pub resource_metadata_url: Option<String>,
    /// Deduplicated scope hints in first-seen order.
    pub scopes: Vec<String>,
    /// Optional OAuth error description.
    pub error_description: Option<String>,
    /// Every raw `WWW-Authenticate` field value in wire order.
    pub raw_www_authenticate: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Typed authorization outcome inferred from a Bearer challenge.
pub enum McpAuthorizationFailure {
    /// Credentials are absent or no recognized Bearer error was advertised.
    AuthorizationRequired,
    /// The authorization request itself was invalid.
    InvalidRequest,
    /// The supplied access token was invalid or expired.
    InvalidToken,
    /// Valid credentials lack one or more required scopes.
    InsufficientScope,
    /// Access was denied without an incremental-scope challenge.
    Forbidden,
}

#[derive(Default)]
struct ParsedBearer {
    descriptions: Vec<String>,
    errors: Vec<String>,
    resource_metadata: Vec<String>,
    scopes: Vec<String>,
}

pub(crate) fn authorization_challenge(
    status: u16,
    raw_fields: &[String],
) -> McpAuthorizationChallenge {
    let parsed = parse_bearer_fields(raw_fields);
    McpAuthorizationChallenge {
        failure: failure(status, &parsed.errors),
        resource_metadata_url: agreed_metadata_url(&parsed.resource_metadata),
        scopes: deduplicated_scopes(&parsed.scopes),
        error_description: parsed.descriptions.into_iter().next(),
        raw_www_authenticate: raw_fields.to_vec(),
    }
}

fn parse_bearer_fields(fields: &[String]) -> ParsedBearer {
    let mut parsed = ParsedBearer::default();
    for field in fields {
        let mut in_bearer = false;
        for segment in split_unquoted_commas(field) {
            let trimmed = segment.trim();
            if let Some(remainder) = challenge_remainder(trimmed, "Bearer") {
                in_bearer = true;
                parse_parameter(remainder, &mut parsed);
            } else if starts_challenge(trimmed) {
                in_bearer = false;
            } else if in_bearer {
                parse_parameter(trimmed, &mut parsed);
            }
        }
    }
    parsed
}

fn split_unquoted_commas(value: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut escaped = false;
    for character in value.chars() {
        match character {
            '"' if !escaped => {
                quoted = !quoted;
                current.push(character);
            }
            ',' if !quoted => {
                segments.push(current);
                current = String::new();
            }
            _ => current.push(character),
        }
        escaped = character == '\\' && !escaped;
        if character != '\\' {
            escaped = false;
        }
    }
    segments.push(current);
    segments
}

fn challenge_remainder<'a>(segment: &'a str, scheme: &str) -> Option<&'a str> {
    let token_end = segment.find(char::is_whitespace).unwrap_or(segment.len());
    let token = &segment[..token_end];
    if token.eq_ignore_ascii_case(scheme) {
        Some(segment[token_end..].trim())
    } else {
        None
    }
}

fn starts_challenge(segment: &str) -> bool {
    let token = segment
        .split_once(char::is_whitespace)
        .map_or(segment, |(token, _)| token);
    !token.contains('=')
}

fn parse_parameter(segment: &str, parsed: &mut ParsedBearer) {
    let Some((name, raw_value)) = segment.split_once('=') else {
        return;
    };
    let Some(value) = decode_parameter_value(raw_value.trim()) else {
        return;
    };
    match name.trim().to_ascii_lowercase().as_str() {
        "error" => parsed.errors.push(value),
        "error_description" => parsed.descriptions.push(value),
        "resource_metadata" => parsed.resource_metadata.push(value),
        "scope" => parsed.scopes.push(value),
        _ => {}
    }
}

fn decode_parameter_value(raw: &str) -> Option<String> {
    if !raw.starts_with('"') {
        return (!raw.is_empty() && !raw.chars().any(char::is_whitespace)).then(|| raw.to_owned());
    }
    if raw.len() < 2 || !raw.ends_with('"') {
        return None;
    }
    let mut decoded = String::new();
    let mut escaped = false;
    for character in raw[1..raw.len() - 1].chars() {
        if escaped {
            decoded.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else {
            decoded.push(character);
        }
    }
    (!escaped).then_some(decoded)
}

fn failure(status: u16, errors: &[String]) -> McpAuthorizationFailure {
    if status == 403 {
        if errors.iter().any(|error| error == "insufficient_scope") {
            McpAuthorizationFailure::InsufficientScope
        } else {
            McpAuthorizationFailure::Forbidden
        }
    } else if errors.iter().any(|error| error == "invalid_token") {
        McpAuthorizationFailure::InvalidToken
    } else if errors.iter().any(|error| error == "invalid_request") {
        McpAuthorizationFailure::InvalidRequest
    } else {
        McpAuthorizationFailure::AuthorizationRequired
    }
}

fn agreed_metadata_url(values: &[String]) -> Option<String> {
    let first = values.first()?;
    if values.iter().any(|value| value != first) || Url::parse(first).is_err() {
        return None;
    }
    Some(first.clone())
}

fn deduplicated_scopes(values: &[String]) -> Vec<String> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .flat_map(|value| value.split_ascii_whitespace())
        .filter(|scope| seen.insert((*scope).to_owned()))
        .map(str::to_owned)
        .collect()
}
