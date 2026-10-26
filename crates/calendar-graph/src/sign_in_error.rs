use scribe_core::calendar::VaultError;

const ADMIN_APPROVAL_CODES: [&str; 2] = ["AADSTS65001", "AADSTS90094"];
const USER_DECLINED: &str = "access_denied";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SignInError {
    #[error("the user cancelled the sign-in or declined consent")]
    Declined,
    #[error("an administrator must approve Scribe")]
    AdminApprovalNeeded,
    #[error("the sign-in was not finished in time")]
    TimedOut,
    #[error("the redirect's state did not match the sign-in")]
    StateMismatch,
    #[error("Microsoft refused the sign-in")]
    Refused,
    #[error("Microsoft could not be reached")]
    Unreachable,
    #[error("Microsoft sent an answer that could not be read")]
    Malformed,
    #[error("the redirect listener failed")]
    Listener,
    #[error("the credential store failed: {0}")]
    Vault(#[from] VaultError),
}

impl SignInError {
    #[must_use]
    pub fn from_refusal(error: &str, description: &str) -> SignInError {
        if ADMIN_APPROVAL_CODES
            .iter()
            .any(|code| description.contains(code))
        {
            return SignInError::AdminApprovalNeeded;
        }
        if error == USER_DECLINED {
            return SignInError::Declined;
        }
        SignInError::Refused
    }

    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            SignInError::Declined => "declined",
            SignInError::AdminApprovalNeeded => "admin_approval_needed",
            SignInError::TimedOut => "timed_out",
            SignInError::StateMismatch => "state_mismatch",
            SignInError::Refused => "refused",
            SignInError::Unreachable => "unreachable",
            SignInError::Malformed => "malformed",
            SignInError::Listener => "listener",
            SignInError::Vault(_) => "vault",
        }
    }

    #[must_use]
    pub fn plain_reason(&self) -> &'static str {
        match self {
            SignInError::Declined => "the sign-in was cancelled or consent was declined",
            SignInError::AdminApprovalNeeded => "your organisation's admin needs to approve Scribe",
            SignInError::TimedOut => "the sign-in wasn't finished within 5 minutes",
            SignInError::StateMismatch => "the browser's answer didn't belong to this sign-in",
            SignInError::Refused => "Microsoft refused the sign-in",
            SignInError::Unreachable => "Microsoft couldn't be reached",
            SignInError::Malformed => "Microsoft's answer couldn't be read",
            SignInError::Listener => "Scribe couldn't wait for the browser's answer",
            SignInError::Vault(_) => "the credential store couldn't keep the sign-in",
        }
    }
}
