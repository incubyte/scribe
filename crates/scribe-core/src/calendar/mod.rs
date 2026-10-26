mod agenda;
mod connection;
mod error;
mod layout;
mod meeting;

use std::time::Duration;

pub use agenda::{TodaysAgenda, todays_agenda};
pub use connection::{AccountIdentity, Connection, Secret};
pub use error::{CalendarError, VaultError};
pub use layout::{
    AgendaLayout, MAX_COLUMNS, MIN_BLOCK_MINUTES, OverflowChip, PlacedMeeting, lay_out,
};
pub use meeting::{CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs};

pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
pub const AGENDA_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
