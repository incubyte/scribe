use chrono::{DateTime, FixedOffset};

use super::meeting::{EventId, Meeting, Person};

pub const MAX_LISTED_ATTENDEES: usize = 50;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetingLink {
    pub event_id: EventId,
    pub subject: String,
    pub starts_at: DateTime<FixedOffset>,
    pub ends_at: DateTime<FixedOffset>,
    pub organizer: Person,
    pub attendees: Vec<Person>,
    pub attendee_count: usize,
}

impl MeetingLink {
    #[must_use]
    pub fn from_meeting(meeting: &Meeting) -> MeetingLink {
        MeetingLink {
            event_id: meeting.event_id.clone(),
            subject: meeting.subject.clone(),
            starts_at: meeting.starts_at,
            ends_at: meeting.ends_at,
            organizer: meeting.organizer.clone(),
            attendees: meeting
                .attendees
                .iter()
                .take(MAX_LISTED_ATTENDEES)
                .cloned()
                .collect(),
            attendee_count: meeting.attendees.len(),
        }
    }
}
