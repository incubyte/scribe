mod support;

use calendar_graph::{SignInError, identity_from};
use support::id_token_for;

#[test]
fn the_account_name_and_email_are_read_from_the_id_token() {
    let token = id_token_for("Riya Shah", "riya@example.com");

    let identity = identity_from(&token).unwrap();

    assert_eq!(identity.display_name, "Riya Shah");
    assert_eq!(identity.email, "riya@example.com");
}

#[test]
fn an_id_token_that_is_not_three_segments_is_malformed() {
    let outcome = identity_from("not-a-jwt");

    assert!(matches!(outcome, Err(SignInError::Malformed)));
}

#[test]
fn an_id_token_whose_claims_are_not_json_is_malformed() {
    let outcome = identity_from("eyJhbGciOiJSUzI1NiJ9.bm90LWpzb24.signature");

    assert!(matches!(outcome, Err(SignInError::Malformed)));
}

#[test]
fn an_id_token_without_a_name_is_malformed() {
    let outcome =
        identity_from("eyJhbGciOiJSUzI1NiJ9.eyJwcmVmZXJyZWRfdXNlcm5hbWUiOiJhQGIuY28ifQ.s");

    assert!(matches!(outcome, Err(SignInError::Malformed)));
}
