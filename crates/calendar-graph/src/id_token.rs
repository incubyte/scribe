use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use scribe_core::calendar::AccountIdentity;
use serde::Deserialize;

use crate::sign_in_error::SignInError;

#[derive(Deserialize)]
struct IdentityClaims {
    name: String,
    preferred_username: String,
}

pub fn identity_from(id_token: &str) -> Result<AccountIdentity, SignInError> {
    let claims = claims_segment(id_token).ok_or(SignInError::Malformed)?;
    let json = URL_SAFE_NO_PAD
        .decode(claims)
        .map_err(|_| SignInError::Malformed)?;
    let claims: IdentityClaims =
        serde_json::from_slice(&json).map_err(|_| SignInError::Malformed)?;
    Ok(AccountIdentity {
        display_name: claims.name,
        email: claims.preferred_username,
    })
}

fn claims_segment(id_token: &str) -> Option<&str> {
    match id_token.split('.').collect::<Vec<_>>().as_slice() {
        [_, claims, _] => Some(claims),
        _ => None,
    }
}
