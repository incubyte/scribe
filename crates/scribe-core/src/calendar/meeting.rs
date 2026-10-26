use chrono::{DateTime, FixedOffset};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EventId(String);

impl EventId {
    pub fn new(value: impl Into<String>) -> EventId {
        EventId(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Person {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Response {
    Organizer,
    Accepted,
    Tentative,
    None,
    Declined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowAs {
    Free,
    Tentative,
    Busy,
    Away,
    WorkingElsewhere,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Meeting {
    pub event_id: EventId,
    pub subject: String,
    pub starts_at: DateTime<FixedOffset>,
    pub ends_at: DateTime<FixedOffset>,
    pub organizer: Person,
    pub attendees: Vec<Person>,
    pub is_teams: bool,
    pub is_recurring: bool,
    pub response: Response,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub meeting: Meeting,
    pub is_all_day: bool,
    pub is_cancelled: bool,
    pub show_as: ShowAs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayBounds {
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
}
