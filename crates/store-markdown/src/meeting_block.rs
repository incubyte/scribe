use std::fmt::Write as _;

use scribe_core::calendar::{EventId, MeetingLink, Person};
use serde::Deserialize;

use crate::format::{MalformedHeader, parse_timestamp, quoted, timestamp};

pub(crate) fn render_meeting(text: &mut String, link: &MeetingLink) {
    text.push_str("meeting:\n");
    let _ = writeln!(text, "  event_id: {}", quoted(link.event_id.as_str()));
    let _ = writeln!(text, "  subject: {}", quoted(&link.subject));
    let _ = writeln!(text, "  starts_at: {}", timestamp(link.starts_at));
    let _ = writeln!(text, "  ends_at: {}", timestamp(link.ends_at));
    let _ = writeln!(text, "  organizer: {}", flow_person(&link.organizer));
    render_attendees(text, &link.attendees);
    let _ = writeln!(text, "  attendee_count: {}", link.attendee_count);
}

fn render_attendees(text: &mut String, attendees: &[Person]) {
    if attendees.is_empty() {
        text.push_str("  attendees: []\n");
        return;
    }
    text.push_str("  attendees:\n");
    for attendee in attendees {
        let _ = writeln!(text, "    - {}", flow_person(attendee));
    }
}

fn flow_person(person: &Person) -> String {
    format!(
        "{{ name: {}, email: {} }}",
        quoted(&person.name),
        quoted(&person.email)
    )
}

pub(crate) fn parse_meeting(front_matter: &str) -> Option<MeetingLink> {
    let block: MeetingBlock = match serde_saphyr::from_str(front_matter) {
        Ok(block) => block,
        Err(_) => return dropped(),
    };
    let fields = block.meeting?;
    fields.into_link().map_or_else(|_| dropped(), Some)
}

fn dropped() -> Option<MeetingLink> {
    tracing::warn!(
        kind = "malformed_meeting",
        "ignored a malformed meeting block"
    );
    None
}

#[derive(Deserialize)]
struct MeetingBlock {
    meeting: Option<MeetingFields>,
}

#[derive(Deserialize)]
struct MeetingFields {
    event_id: String,
    subject: String,
    starts_at: String,
    ends_at: String,
    organizer: PersonFields,
    attendees: Vec<PersonFields>,
    attendee_count: usize,
}

#[derive(Deserialize)]
struct PersonFields {
    name: String,
    email: String,
}

impl From<PersonFields> for Person {
    fn from(fields: PersonFields) -> Person {
        Person {
            name: fields.name,
            email: fields.email,
        }
    }
}

impl MeetingFields {
    fn into_link(self) -> Result<MeetingLink, MalformedHeader> {
        Ok(MeetingLink {
            event_id: EventId::new(self.event_id),
            subject: self.subject,
            starts_at: parse_timestamp(&self.starts_at)?,
            ends_at: parse_timestamp(&self.ends_at)?,
            organizer: self.organizer.into(),
            attendees: self.attendees.into_iter().map(Person::from).collect(),
            attendee_count: self.attendee_count,
        })
    }
}
