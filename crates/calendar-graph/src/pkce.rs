use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use rand::RngCore;
use scribe_core::calendar::Secret;
use sha2::{Digest, Sha256};
use url::Url;

use crate::authority::{AUTHORIZE_URL, CLIENT_ID, SCOPES};

const RANDOM_BYTES: usize = 32;

pub struct SignInRequest {
    verifier: Secret,
    challenge: String,
    state: String,
}

impl SignInRequest {
    #[must_use]
    pub fn generate() -> SignInRequest {
        let verifier = Secret::new(random_token());
        SignInRequest {
            challenge: challenge_for(&verifier),
            verifier,
            state: random_token(),
        }
    }

    #[must_use]
    pub fn verifier(&self) -> &Secret {
        &self.verifier
    }

    #[must_use]
    pub fn state(&self) -> &str {
        &self.state
    }

    #[must_use]
    pub fn authorize_url(&self, redirect: &Url) -> Url {
        let mut url = Url::parse(AUTHORIZE_URL).expect("the authorize URL is a valid constant");
        url.query_pairs_mut()
            .append_pair("client_id", CLIENT_ID)
            .append_pair("response_type", "code")
            .append_pair("redirect_uri", redirect.as_str())
            .append_pair("response_mode", "query")
            .append_pair("scope", SCOPES)
            .append_pair("state", &self.state)
            .append_pair("code_challenge", &self.challenge)
            .append_pair("code_challenge_method", "S256");
        url
    }
}

#[must_use]
pub fn challenge_for(verifier: &Secret) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.expose().as_bytes()))
}

fn random_token() -> String {
    let mut bytes = [0_u8; RANDOM_BYTES];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}
