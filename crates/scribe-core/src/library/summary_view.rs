use crate::calendar::{MeetingLink, Person};
use crate::recording::SessionSummary;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PersonView {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MeetingSummaryView {
    pub subject: String,
    pub organizer: PersonView,
    pub attendees: Vec<PersonView>,
    pub attendee_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SessionSummaryView {
    pub id: String,
    pub title: String,
    pub started_at: String,
    pub duration_seconds: Option<u64>,
    pub status: &'static str,
    #[cfg_attr(feature = "serde", serde(skip_serializing_if = "Option::is_none"))]
    pub meeting: Option<MeetingSummaryView>,
}

impl From<SessionSummary> for SessionSummaryView {
    fn from(summary: SessionSummary) -> SessionSummaryView {
        SessionSummaryView {
            id: summary.id.to_string(),
            title: summary.title,
            started_at: summary.started_at.to_rfc3339(),
            duration_seconds: summary.duration_seconds,
            status: summary.status.as_str(),
            meeting: summary.meeting.map(MeetingSummaryView::from),
        }
    }
}

impl From<MeetingLink> for MeetingSummaryView {
    fn from(link: MeetingLink) -> MeetingSummaryView {
        MeetingSummaryView {
            subject: link.subject,
            organizer: link.organizer.into(),
            attendees: link.attendees.into_iter().map(PersonView::from).collect(),
            attendee_count: link.attendee_count,
        }
    }
}

impl From<Person> for PersonView {
    fn from(person: Person) -> PersonView {
        PersonView {
            name: person.name,
            email: person.email,
        }
    }
}
