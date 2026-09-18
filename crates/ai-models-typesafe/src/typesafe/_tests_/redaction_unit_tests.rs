//! TypeSafe credential secret derivation tests.

use std::collections::BTreeMap;

use super::secrets_from_headers;

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
