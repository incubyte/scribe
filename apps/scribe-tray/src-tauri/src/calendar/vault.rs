use keyring::{Entry, Error};
use scribe_core::calendar::{Secret, VaultError};
use scribe_core::ports::CredentialVault;

const SERVICE: &str = "com.incubyte.scribe";
const ACCOUNT: &str = "outlook-refresh-token";

pub struct KeyringVault {
    entry: Entry,
}

impl KeyringVault {
    pub fn open() -> Result<KeyringVault, VaultError> {
        Entry::new(SERVICE, ACCOUNT)
            .map(KeyringVault::over)
            .map_err(|error| vault_error(&error))
    }

    #[must_use]
    pub fn over(entry: Entry) -> KeyringVault {
        KeyringVault { entry }
    }
}

impl CredentialVault for KeyringVault {
    fn load(&self) -> Result<Option<Secret>, VaultError> {
        match self.entry.get_secret() {
            Ok(bytes) => String::from_utf8(bytes)
                .map(|token| Some(Secret::new(token)))
                .map_err(|_| VaultError::Unreadable),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(vault_error(&error)),
        }
    }

    fn save(&self, secret: &Secret) -> Result<(), VaultError> {
        self.entry
            .set_secret(secret.expose().as_bytes())
            .map_err(|error| vault_error(&error))
    }

    fn forget(&self) -> Result<(), VaultError> {
        match self.entry.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(vault_error(&error)),
        }
    }
}

fn vault_error(error: &Error) -> VaultError {
    match error {
        Error::TooLong(..) => VaultError::TooLarge,
        Error::BadEncoding(_) => VaultError::Unreadable,
        _ => VaultError::Unavailable,
    }
}
