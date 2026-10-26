#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum VaultError {
    #[error("the credential store could not be reached")]
    Unavailable,
    #[error("the token is too large for the credential store")]
    TooLarge,
    #[error("the stored token could not be read")]
    Unreadable,
}

impl VaultError {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            VaultError::Unavailable => "unavailable",
            VaultError::TooLarge => "too_large",
            VaultError::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CalendarError {
    #[error("no Outlook account is connected")]
    NotConnected,
    #[error("Outlook needs the user to sign in again")]
    ReconnectNeeded,
    #[error("Microsoft could not be reached")]
    Unreachable,
    #[error("Microsoft sent an answer that could not be read")]
    Malformed,
    #[error("the credential store failed: {0}")]
    CredentialStore(#[from] VaultError),
}

impl CalendarError {
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            CalendarError::NotConnected => "not_connected",
            CalendarError::ReconnectNeeded => "reconnect_needed",
            CalendarError::Unreachable => "unreachable",
            CalendarError::Malformed => "malformed",
            CalendarError::CredentialStore(_) => "credential_store",
        }
    }
}
