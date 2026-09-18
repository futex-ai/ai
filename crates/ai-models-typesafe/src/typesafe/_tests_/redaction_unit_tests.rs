//! TypeSafe credential secret derivation tests.

use std::collections::BTreeMap;

use super::{redact_secrets, secrets_from_headers};

#[test]
fn secrets_include_explicit_full_and_scheme_stripped_values_without_empties() {
    let explicit = ["explicit-secret".to_owned(), String::new()];
    let headers = BTreeMap::from([
        (
            "Authorization".to_owned(),
            "Bearer header-secret".to_owned(),
        ),
        ("X-Api-Key".to_owned(), "custom-secret".to_owned()),
        ("X-Empty".to_owned(), String::new()),
    ]);

    assert_eq!(
        secrets_from_headers(&explicit, &headers),
        vec![
            "explicit-secret".to_owned(),
            "Bearer header-secret".to_owned(),
            "header-secret".to_owned(),
            "custom-secret".to_owned(),
        ]
    );
}

#[test]
fn overlapping_secrets_are_redacted_longest_first() {
    let secrets = [
        "token".to_owned(),
        "token-secret".to_owned(),
        "token".to_owned(),
    ];

    assert_eq!(
        redact_secrets("rejected token-secret and token", &secrets),
        "rejected [redacted] and [redacted]"
    );
    assert_eq!(
        redact_secrets(
            "rejected token-secret",
            &["token-secret".to_owned(), "token".to_owned()]
        ),
        "rejected [redacted]"
    );
}
