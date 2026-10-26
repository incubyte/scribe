mod connection;
mod error;

use std::time::Duration;

pub use connection::{AccountIdentity, Connection, Secret};
pub use error::{CalendarError, VaultError};

pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
