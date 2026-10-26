use keyring::mock::MockCredential;
use keyring::{Entry, Error};
use scribe_core::calendar::{Secret, VaultError};
use scribe_core::ports::CredentialVault;
use scribe_tray::calendar::vault::KeyringVault;

fn mock_entry() -> Entry {
    Entry::new_with_credential(Box::new(MockCredential::default()))
}

fn entry_failing_next_call_with(error: Error) -> Entry {
    let entry = mock_entry();
    let credential: &MockCredential = entry.get_credential().downcast_ref().unwrap();
    credential.set_error(error);
    entry
}

#[test]
fn a_saved_token_is_read_back_from_the_credential_store() {
    let vault = KeyringVault::over(mock_entry());

    vault.save(&Secret::new("refresh-1")).unwrap();

    assert_eq!(vault.load().unwrap().unwrap().expose(), "refresh-1");
}

#[test]
fn an_empty_credential_store_holds_no_token() {
    let vault = KeyringVault::over(mock_entry());

    assert_eq!(vault.load().unwrap(), None);
}

#[test]
fn forgetting_removes_the_stored_token() {
    let vault = KeyringVault::over(mock_entry());
    vault.save(&Secret::new("refresh-1")).unwrap();

    vault.forget().unwrap();

    assert_eq!(vault.load().unwrap(), None);
}

#[test]
fn forgetting_when_nothing_is_stored_succeeds() {
    let vault = KeyringVault::over(mock_entry());

    assert_eq!(vault.forget(), Ok(()));
}

#[test]
fn a_token_too_large_for_the_credential_store_fails_by_name_instead_of_being_cut() {
    let vault = KeyringVault::over(entry_failing_next_call_with(Error::TooLong(
        "secret".to_owned(),
        2560,
    )));

    let outcome = vault.save(&Secret::new("refresh-1"));

    assert_eq!(outcome, Err(VaultError::TooLarge));
    assert_eq!(VaultError::TooLarge.kind(), "too_large");
}

#[test]
fn a_locked_credential_store_is_reported_as_unavailable() {
    let vault = KeyringVault::over(entry_failing_next_call_with(Error::NoStorageAccess(
        "locked".into(),
    )));

    assert_eq!(vault.load(), Err(VaultError::Unavailable));
}

#[test]
fn a_stored_value_that_is_not_text_is_reported_as_unreadable() {
    let vault = KeyringVault::over(entry_failing_next_call_with(Error::BadEncoding(vec![0xff])));

    assert_eq!(vault.load(), Err(VaultError::Unreadable));
}
