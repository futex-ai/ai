//! Cache-Control directive parsing regressions.

use std::collections::BTreeMap;

use super::cache_age_seconds;

#[test]
fn non_reuse_directives_override_max_age_in_any_order() {
    for value in [
        "max-age=3600, no-store",
        "no-store, max-age=3600",
        "max-age=3600, no-cache",
        "max-age=3600, NO-CACHE=\"set-cookie\"",
        "no-cache=\"a, b\", max-age=3600",
    ] {
        assert_eq!(age(&[value], &[], 7200), 0, "{value}");
    }
}

#[test]
fn minimum_valid_max_age_wins_across_values_and_responses() {
    assert_eq!(age(&["max-age=120", "max-age=60"], &[], 7200), 60);
    assert_eq!(age(&["max-age=\"90\""], &[], 7200), 90);
    assert_eq!(age(&["max-age=120"], &["max-age=30"], 7200), 30);
}

#[test]
fn malformed_max_age_disables_caching() {
    for value in [
        "max-age=abc",
        "max-age=",
        "max-age=18446744073709551616",
        "max-age=abc, max-age=60",
    ] {
        assert_eq!(age(&[value], &[], 7200), 0, "{value}");
    }
}

#[test]
fn absent_or_irrelevant_directives_use_the_configured_maximum() {
    assert_eq!(age(&[], &[], 7200), 7200);
    assert_eq!(
        age(&["private, must-revalidate, s-maxage=5"], &["public"], 7200),
        7200
    );
}

fn age(protected: &[&str], server: &[&str], maximum: u64) -> u64 {
    cache_age_seconds(&headers(protected), &headers(server), maximum)
}

fn headers(values: &[&str]) -> BTreeMap<String, Vec<String>> {
    if values.is_empty() {
        return BTreeMap::new();
    }
    BTreeMap::from([(
        "cache-control".to_owned(),
        values.iter().map(|value| (*value).to_owned()).collect(),
    )])
}
