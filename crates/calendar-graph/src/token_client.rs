use std::time::Duration;

use scribe_core::calendar::{AccountIdentity, CalendarError, Secret};
use serde::Deserialize;
use url::Url;

use crate::authority::{CLIENT_ID, SCOPES, TOKEN_URL};
use crate::id_token::identity_from;
use crate::sign_in_error::SignInError;
use crate::transport::{HttpReply, HttpTransport};

const SIGN_IN_AGAIN_ERRORS: [&str; 2] = ["invalid_grant", "interaction_required"];

pub struct TokenGrant {
    pub access: Secret,
    pub expires_in: Duration,
    pub refresh: Secret,
    pub identity: AccountIdentity,
}

pub struct CodeProof<'a> {
    pub verifier: &'a Secret,
    pub redirect_uri: &'a Url,
}

#[derive(Deserialize)]
struct TokenReply {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
    id_token: String,
}

#[derive(Deserialize)]
struct TokenRefusal {
    error: String,
}

pub fn redeem_code(
    transport: &dyn HttpTransport,
    code: &Secret,
    proof: &CodeProof,
) -> Result<TokenGrant, SignInError> {
    let form = [
        ("client_id", CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("scope", SCOPES),
        ("code", code.expose()),
        ("redirect_uri", proof.redirect_uri.as_str()),
        ("code_verifier", proof.verifier.expose()),
    ];
    let reply = transport
        .post_form(TOKEN_URL, &form)
        .map_err(|_| SignInError::Unreachable)?;
    if !reply.is_success() {
        return Err(SignInError::Refused);
    }
    grant_from(&reply.body)
}

pub fn refresh(
    transport: &dyn HttpTransport,
    refresh: &Secret,
) -> Result<TokenGrant, CalendarError> {
    let form = [
        ("client_id", CLIENT_ID),
        ("grant_type", "refresh_token"),
        ("scope", SCOPES),
        ("refresh_token", refresh.expose()),
    ];
    let reply = transport
        .post_form(TOKEN_URL, &form)
        .map_err(|_| CalendarError::Unreachable)?;
    if !reply.is_success() {
        return Err(refusal_of(&reply));
    }
    grant_from(&reply.body).map_err(|_| CalendarError::Malformed)
}

fn refusal_of(reply: &HttpReply) -> CalendarError {
    let needs_sign_in = serde_json::from_str::<TokenRefusal>(&reply.body)
        .is_ok_and(|refusal| SIGN_IN_AGAIN_ERRORS.contains(&refusal.error.as_str()));
    if needs_sign_in {
        CalendarError::ReconnectNeeded
    } else {
        CalendarError::Unreachable
    }
}

fn grant_from(body: &str) -> Result<TokenGrant, SignInError> {
    let reply: TokenReply = serde_json::from_str(body).map_err(|_| SignInError::Malformed)?;
    Ok(TokenGrant {
        identity: identity_from(&reply.id_token)?,
        access: Secret::new(reply.access_token),
        expires_in: Duration::from_secs(reply.expires_in),
        refresh: Secret::new(reply.refresh_token),
    })
}
