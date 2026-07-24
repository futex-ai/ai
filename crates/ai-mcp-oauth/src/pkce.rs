//! PKCE S256 and authorization state generation.

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use secrecy::SecretString;
use sha2::{Digest, Sha256};

use crate::{OAuthRandom, Result};

const SECRET_BYTE_LENGTH: usize = 32;

pub(crate) struct AuthorizationSecrets {
    pub(crate) state: SecretString,
    pub(crate) verifier: SecretString,
    pub(crate) challenge: String,
}

pub(crate) fn generate_authorization_secrets(
    random: &dyn OAuthRandom,
) -> Result<AuthorizationSecrets> {
    let state_bytes = random.bytes(SECRET_BYTE_LENGTH)?;
    let verifier_bytes = random.bytes(SECRET_BYTE_LENGTH)?;
    if state_bytes.len() != SECRET_BYTE_LENGTH || verifier_bytes.len() != SECRET_BYTE_LENGTH {
        return Err(crate::Error::Random);
    }
    let state = URL_SAFE_NO_PAD.encode(state_bytes);
    let verifier = URL_SAFE_NO_PAD.encode(verifier_bytes);
    let challenge = pkce_challenge(&verifier);
    Ok(AuthorizationSecrets {
        state: SecretString::from(state),
        verifier: SecretString::from(verifier),
        challenge,
    })
}

pub(crate) fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}
