//! Authorization-server metadata URL derivation tests.

use crate::OAuthUrlPolicy;

use super::authorization_server_metadata_url;

#[test]
fn well_known_url_preserves_issuer_path_shape() {
    let cases = [
        (
            "https://auth.example//tenant",
            "https://auth.example/.well-known/oauth-authorization-server//tenant",
        ),
        (
            "https://auth.example/tenant/",
            "https://auth.example/.well-known/oauth-authorization-server/tenant/",
        ),
        (
            "https://auth.example/a%2Fb",
            "https://auth.example/.well-known/oauth-authorization-server/a%2Fb",
        ),
    ];

    for (issuer, expected) in cases {
        assert_eq!(
            authorization_server_metadata_url(issuer, &OAuthUrlPolicy::default()).unwrap(),
            expected
        );
    }
}
