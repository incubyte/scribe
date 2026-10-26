use calendar_graph::{SignInRequest, challenge_for};
use scribe_core::calendar::Secret;
use url::Url;

const RFC_7636_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const RFC_7636_CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
const REDIRECT: &str = "http://127.0.0.1:53682/";

fn query_value(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

fn authorize_url_for(request: &SignInRequest) -> Url {
    request.authorize_url(&Url::parse(REDIRECT).unwrap())
}

#[test]
fn the_challenge_is_the_s256_hash_of_the_verifier_as_rfc_7636_specifies() {
    let verifier = Secret::new(RFC_7636_VERIFIER);

    assert_eq!(challenge_for(&verifier), RFC_7636_CHALLENGE);
}

#[test]
fn sign_in_goes_to_the_work_and_school_authority() {
    let url = authorize_url_for(&SignInRequest::generate());

    assert_eq!(url.scheme(), "https");
    assert_eq!(url.host_str(), Some("login.microsoftonline.com"));
    assert_eq!(url.path(), "/organizations/oauth2/v2.0/authorize");
}

#[test]
fn sign_in_asks_for_exactly_the_read_only_calendar_scopes() {
    let url = authorize_url_for(&SignInRequest::generate());

    assert_eq!(
        query_value(&url, "scope").as_deref(),
        Some("openid profile offline_access Calendars.Read")
    );
}

#[test]
fn sign_in_asks_for_an_authorization_code_bound_to_an_s256_challenge() {
    let request = SignInRequest::generate();

    let url = authorize_url_for(&request);

    assert_eq!(query_value(&url, "response_type").as_deref(), Some("code"));
    assert_eq!(
        query_value(&url, "code_challenge_method").as_deref(),
        Some("S256")
    );
    assert_eq!(
        query_value(&url, "code_challenge"),
        Some(challenge_for(request.verifier()))
    );
}

#[test]
fn sign_in_sends_its_state_and_loopback_redirect_to_microsoft() {
    let request = SignInRequest::generate();

    let url = authorize_url_for(&request);

    assert_eq!(query_value(&url, "state").as_deref(), Some(request.state()));
    assert_eq!(query_value(&url, "redirect_uri").as_deref(), Some(REDIRECT));
    assert_eq!(query_value(&url, "response_mode").as_deref(), Some("query"));
}

#[test]
fn each_sign_in_gets_its_own_random_state_and_verifier() {
    let first = SignInRequest::generate();
    let second = SignInRequest::generate();

    assert_ne!(first.state(), second.state());
    assert_ne!(first.verifier().expose(), second.verifier().expose());
}

#[test]
fn the_verifier_carries_32_random_bytes_as_unpadded_base64url() {
    let request = SignInRequest::generate();

    assert_eq!(request.verifier().expose().len(), 43);
    assert!(
        request
            .verifier()
            .expose()
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    );
}
