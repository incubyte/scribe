mod agenda;
mod armed;
mod connection;
mod error;
mod layout;
mod link;
mod matching;
mod meeting;
mod titles;

use std::time::Duration;

pub use agenda::{TodaysAgenda, todays_agenda};
pub use armed::{ArmedAction, ArmedEntry, ArmedMeetings, ArmedState, ArmedWatch, WatchInputs};
pub use connection::{AccountIdentity, Connection, Secret};
pub use error::{CalendarError, VaultError};
pub use layout::{
    AgendaLayout, MAX_COLUMNS, MIN_BLOCK_MINUTES, OverflowChip, PlacedMeeting, lay_out,
};
pub use link::{MAX_LISTED_ATTENDEES, MeetingLink};
pub use matching::{MatchAnchor, MatchInputs, MatchOutcome, match_meeting, meeting_candidates};
pub use meeting::{CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs};
pub use titles::meeting_named_in_titles;

pub const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(5 * 60);
pub const AGENDA_REFRESH_INTERVAL: Duration = Duration::from_secs(5 * 60);
pub const CANDIDATE_LEAD: Duration = Duration::from_secs(10 * 60);
pub const MISSED_AFTER: Duration = Duration::from_secs(5 * 60);
