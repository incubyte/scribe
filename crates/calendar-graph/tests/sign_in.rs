mod support;

use std::sync::Arc;
use std::thread;

use calendar_graph::{OutlookAccount, SignInError, challenge_for};
use scribe_core::calendar::{AccountIdentity, CalendarError, Secret, VaultError};
use support::{
    DISPLAY_NAME, EMAIL, FakeTransport, FakeVault, HeldTransport, ScriptedBrowser, error_reply,
    query_value, token_reply, unreachable,
};

const AN_HOUR: u64 = 3600;
const HALF_A_MINUTE: u64 = 30;

fn account_holder() -> AccountIdentity {
    AccountIdentity {
        display_name: DISPLAY_NAME.to_owned(),
        email: EMAIL.to_owned(),
    }
}

#[test]
fn signing_in_returns_the_account_name_and_email() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let account = OutlookAccount::new(transport, FakeVault::empty());
    let browser = ScriptedBrowser::approving();

    let identity = account.connect(&|url| browser.open(url)).unwrap();

    assert_eq!(identity, account_holder());
}

#[test]
fn signing_in_keeps_the_refresh_token_in_the_credential_vault() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let vault = FakeVault::empty();
    let account = OutlookAccount::new(transport, vault.clone());
    let browser = ScriptedBrowser::approving();

    account.connect(&|url| browser.open(url)).unwrap();

    assert_eq!(vault.stored().as_deref(), Some("refresh-1"));
}

#[test]
fn the_code_is_redeemed_at_the_work_and_school_token_endpoint() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let account = OutlookAccount::new(transport.clone(), FakeVault::empty());
    let browser = ScriptedBrowser::approving();

    account.connect(&|url| browser.open(url)).unwrap();

    let redemption = &transport.form_requests()[0];
    assert_eq!(
        redemption.url,
        "https://login.microsoftonline.com/organizations/oauth2/v2.0/token"
    );
    assert_eq!(redemption.field("grant_type"), Some("authorization_code"));
    assert_eq!(redemption.field("code"), Some("auth-code"));
}

#[test]
fn the_code_is_redeemed_with_the_verifier_whose_challenge_microsoft_was_sent() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let account = OutlookAccount::new(transport.clone(), FakeVault::empty());
    let browser = ScriptedBrowser::approving();

    account.connect(&|url| browser.open(url)).unwrap();

    let redemption = &transport.form_requests()[0];
    let verifier = Secret::new(redemption.field("code_verifier").unwrap());
    let authorize_url = browser.opened_url();
    assert_eq!(
        challenge_for(&verifier),
        query_value(&authorize_url, "code_challenge")
    );
    assert_eq!(
        redemption.field("redirect_uri"),
        Some(query_value(&authorize_url, "redirect_uri").as_str())
    );
}

#[test]
fn a_redirect_with_a_forged_state_is_rejected_without_redeeming_the_code() {
    let transport = FakeTransport::silent();
    let vault = FakeVault::empty();
    let account = OutlookAccount::new(transport.clone(), vault.clone());
    let browser = ScriptedBrowser::redirecting_with(|_| "code=stolen-code&state=forged".to_owned());

    let outcome = account.connect(&|url| browser.open(url));

    assert!(matches!(outcome, Err(SignInError::StateMismatch)));
    assert!(transport.form_requests().is_empty());
    assert_eq!(vault.stored(), None);
}

#[test]
fn cancelling_consent_is_reported_as_declined_in_plain_words() {
    let vault = FakeVault::empty();
    let account = OutlookAccount::new(FakeTransport::silent(), vault.clone());
    let browser = ScriptedBrowser::redirecting_with(|state| {
        format!("error=access_denied&error_description=AADSTS65004%3A+User+declined&state={state}")
    });

    let error = account.connect(&|url| browser.open(url)).unwrap_err();

    assert!(matches!(error, SignInError::Declined));
    assert_eq!(
        error.plain_reason(),
        "the sign-in was cancelled or consent was declined"
    );
    assert_eq!(vault.stored(), None);
}

#[test]
fn an_organisation_that_requires_admin_approval_is_reported_as_such() {
    let account = OutlookAccount::new(FakeTransport::silent(), FakeVault::empty());
    let browser = ScriptedBrowser::redirecting_with(|state| {
        format!(
            "error=consent_required&error_description=AADSTS65001%3A+admin+consent+needed&state={state}"
        )
    });

    let error = account.connect(&|url| browser.open(url)).unwrap_err();

    assert!(matches!(error, SignInError::AdminApprovalNeeded));
    assert_eq!(
        error.plain_reason(),
        "your organisation's admin needs to approve Scribe"
    );
}

#[test]
fn a_refused_code_redemption_fails_the_sign_in() {
    let transport = FakeTransport::replying(vec![error_reply("invalid_grant", "AADSTS70008")]);
    let vault = FakeVault::empty();
    let account = OutlookAccount::new(transport, vault.clone());
    let browser = ScriptedBrowser::approving();

    let outcome = account.connect(&|url| browser.open(url));

    assert!(matches!(outcome, Err(SignInError::Refused)));
    assert_eq!(vault.stored(), None);
}

#[test]
fn a_vault_that_cannot_hold_the_token_fails_the_sign_in_by_name() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let account = OutlookAccount::new(transport, FakeVault::refusing_to_save(VaultError::TooLarge));
    let browser = ScriptedBrowser::approving();

    let error = account.connect(&|url| browser.open(url)).unwrap_err();

    assert!(matches!(error, SignInError::Vault(VaultError::TooLarge)));
    assert_eq!(error.kind(), "vault");
}

#[test]
fn restoring_a_stored_connection_returns_the_account_without_a_browser() {
    let transport = FakeTransport::replying(vec![token_reply("access-2", "refresh-2", AN_HOUR)]);
    let account = OutlookAccount::new(transport.clone(), FakeVault::holding("refresh-1"));

    let identity = account.restore().unwrap();

    let refresh = &transport.form_requests()[0];
    assert_eq!(identity, Some(account_holder()));
    assert_eq!(refresh.field("grant_type"), Some("refresh_token"));
    assert_eq!(refresh.field("refresh_token"), Some("refresh-1"));
}

#[test]
fn restoring_with_nothing_stored_is_signed_out_without_asking_microsoft() {
    let transport = FakeTransport::silent();
    let account = OutlookAccount::new(transport.clone(), FakeVault::empty());

    let identity = account.restore().unwrap();

    assert_eq!(identity, None);
    assert!(transport.form_requests().is_empty());
}

#[test]
fn a_refused_refresh_means_the_user_must_reconnect() {
    let transport = FakeTransport::replying(vec![error_reply("invalid_grant", "AADSTS700082")]);
    let account = OutlookAccount::new(transport, FakeVault::holding("refresh-1"));

    let outcome = account.restore();

    assert!(matches!(outcome, Err(CalendarError::ReconnectNeeded)));
}

#[test]
fn a_refresh_that_needs_interaction_means_the_user_must_reconnect() {
    let transport =
        FakeTransport::replying(vec![error_reply("interaction_required", "AADSTS50076")]);
    let account = OutlookAccount::new(transport, FakeVault::holding("refresh-1"));

    let outcome = account.restore();

    assert!(matches!(outcome, Err(CalendarError::ReconnectNeeded)));
}

#[test]
fn a_rotated_refresh_token_replaces_the_stored_one() {
    let transport = FakeTransport::replying(vec![token_reply("access-2", "refresh-2", AN_HOUR)]);
    let vault = FakeVault::holding("refresh-1");
    let account = OutlookAccount::new(transport, vault.clone());

    account.restore().unwrap();

    assert_eq!(vault.stored().as_deref(), Some("refresh-2"));
}

#[test]
fn an_unreachable_microsoft_keeps_the_stored_token() {
    let vault = FakeVault::holding("refresh-1");
    let account = OutlookAccount::new(FakeTransport::replying(vec![unreachable()]), vault.clone());

    let outcome = account.restore();

    assert!(matches!(outcome, Err(CalendarError::Unreachable)));
    assert_eq!(vault.stored().as_deref(), Some("refresh-1"));
}

#[test]
fn disconnecting_forgets_the_stored_token() {
    let vault = FakeVault::holding("refresh-1");
    let account = OutlookAccount::new(FakeTransport::silent(), vault.clone());

    account.disconnect().unwrap();

    assert_eq!(vault.stored(), None);
}

#[test]
fn a_disconnected_account_has_no_access_token() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let account = OutlookAccount::new(transport, FakeVault::empty());
    let browser = ScriptedBrowser::approving();
    account.connect(&|url| browser.open(url)).unwrap();

    account.disconnect().unwrap();

    assert!(matches!(
        account.fresh_access_token(),
        Err(CalendarError::NotConnected)
    ));
}

#[test]
fn a_fresh_access_token_is_reused_without_asking_microsoft_again() {
    let transport = FakeTransport::replying(vec![token_reply("access-1", "refresh-1", AN_HOUR)]);
    let account = OutlookAccount::new(transport.clone(), FakeVault::empty());
    let browser = ScriptedBrowser::approving();
    account.connect(&|url| browser.open(url)).unwrap();

    let token = account.fresh_access_token().unwrap();

    assert_eq!(token.expose(), "access-1");
    assert_eq!(transport.form_requests().len(), 1);
}

#[test]
fn an_access_token_about_to_expire_is_refreshed_first() {
    let transport = FakeTransport::replying(vec![
        token_reply("access-1", "refresh-1", HALF_A_MINUTE),
        token_reply("access-2", "refresh-2", AN_HOUR),
    ]);
    let vault = FakeVault::empty();
    let account = OutlookAccount::new(transport, vault.clone());
    let browser = ScriptedBrowser::approving();
    account.connect(&|url| browser.open(url)).unwrap();

    let token = account.fresh_access_token().unwrap();

    assert_eq!(token.expose(), "access-2");
    assert_eq!(vault.stored().as_deref(), Some("refresh-2"));
}

#[test]
fn disconnecting_while_a_restore_refreshes_leaves_the_vault_empty() {
    let (transport, gate) = HeldTransport::holding(token_reply("access-2", "refresh-2", AN_HOUR));
    let vault = FakeVault::holding("refresh-1");
    let account = Arc::new(OutlookAccount::new(transport, vault.clone()));
    let restoring = thread::spawn({
        let account = Arc::clone(&account);
        move || account.restore()
    });
    gate.wait_for_request();

    account.disconnect().unwrap();
    gate.release();
    let restored = restoring.join().unwrap();

    assert_eq!(vault.stored(), None);
    assert_eq!(restored, Ok(None));
    assert!(matches!(
        account.fresh_access_token(),
        Err(CalendarError::NotConnected)
    ));
}

#[test]
fn disconnecting_while_an_access_token_refreshes_leaves_the_vault_empty() {
    let (transport, gate) = HeldTransport::holding(token_reply("access-2", "refresh-2", AN_HOUR));
    let vault = FakeVault::holding("refresh-1");
    let account = Arc::new(OutlookAccount::new(transport, vault.clone()));
    let refreshing = thread::spawn({
        let account = Arc::clone(&account);
        move || account.fresh_access_token()
    });
    gate.wait_for_request();

    let disconnecting = thread::spawn({
        let account = Arc::clone(&account);
        move || account.disconnect()
    });
    gate.release();
    refreshing.join().unwrap().unwrap();
    disconnecting.join().unwrap().unwrap();

    assert_eq!(vault.stored(), None);
}
