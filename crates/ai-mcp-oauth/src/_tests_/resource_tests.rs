//! Canonical resource and scope tests.

use crate::{CanonicalMcpResource, Error, OAuthScopes, OAuthUrlPolicy};

#[test]
fn canonicalizes_scheme_host_and_root_slash() {
    let resource =
        CanonicalMcpResource::parse("HTTPS://EXAMPLE.COM/", &OAuthUrlPolicy::default()).unwrap();

    assert_eq!(resource.as_str(), "https://example.com");
}

#[test]
fn preserves_specific_path_and_query_identity() {
    let resource = CanonicalMcpResource::parse(
        "https://EXAMPLE.com/mcp/?tenant=one",
        &OAuthUrlPolicy::default(),
    )
    .unwrap();

    assert_eq!(resource.as_str(), "https://example.com/mcp/?tenant=one");
}

#[test]
fn rejects_resource_fragments() {
    let error = CanonicalMcpResource::parse(
        "https://example.com/mcp#fragment",
        &OAuthUrlPolicy::default(),
    )
    .unwrap_err();

    assert!(matches!(error, Error::UnsafeUrl { .. }));
}

#[test]
fn inserts_well_known_path_for_root_and_specific_resources() {
    let root =
        CanonicalMcpResource::parse("https://example.com", &OAuthUrlPolicy::default()).unwrap();
    let path =
        CanonicalMcpResource::parse("https://example.com/api/mcp", &OAuthUrlPolicy::default())
            .unwrap();

    assert_eq!(
        root.protected_resource_metadata_url().unwrap(),
        "https://example.com/.well-known/oauth-protected-resource"
    );
    assert_eq!(
        path.protected_resource_metadata_url().unwrap(),
        "https://example.com/.well-known/oauth-protected-resource/api/mcp"
    );
}

#[test]
fn scopes_deduplicate_without_reordering() {
    let scopes = OAuthScopes::new(["read", "write", "read"]);
    let elevated = scopes.union(&OAuthScopes::new(["admin", "write"]));

    assert_eq!(scopes.as_slice(), &["read", "write"]);
    assert_eq!(elevated.as_slice(), &["read", "write", "admin"]);
    assert_eq!(elevated.to_parameter(), "read write admin");
}
