use chrono::DateTime;
use scribe_core::calendar::{
    EventId, MAX_LISTED_ATTENDEES, Meeting, MeetingLink, Person, Response,
};

fn invitee(number: usize) -> Person {
    Person {
        name: format!("Invitee {number}"),
        email: format!("invitee{number}@example.com"),
    }
}

fn meeting_with(attendee_count: usize) -> Meeting {
    Meeting {
        event_id: EventId::new("review"),
        subject: "DXC Review".to_owned(),
        starts_at: DateTime::parse_from_rfc3339("2026-10-26T11:00:00+05:30").unwrap(),
        ends_at: DateTime::parse_from_rfc3339("2026-10-26T11:45:00+05:30").unwrap(),
        organizer: Person {
            name: "Sneha Rao".to_owned(),
            email: "sneha@example.com".to_owned(),
        },
        attendees: (0..attendee_count).map(invitee).collect(),
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    }
}

#[test]
fn a_meeting_with_sixty_invitees_lists_fifty_and_counts_sixty() {
    let link = MeetingLink::from_meeting(&meeting_with(60));

    assert_eq!(link.attendees.len(), MAX_LISTED_ATTENDEES);
    assert_eq!(link.attendee_count, 60);
}

#[test]
fn a_small_meeting_lists_every_invitee() {
    let link = MeetingLink::from_meeting(&meeting_with(3));

    assert_eq!(link.attendees, vec![invitee(0), invitee(1), invitee(2)]);
    assert_eq!(link.attendee_count, 3);
}

#[test]
fn the_link_carries_the_meeting_s_identity_and_times() {
    let meeting = meeting_with(1);

    let link = MeetingLink::from_meeting(&meeting);

    assert_eq!(
        link,
        MeetingLink {
            event_id: meeting.event_id,
            subject: "DXC Review".to_owned(),
            starts_at: meeting.starts_at,
            ends_at: meeting.ends_at,
            organizer: meeting.organizer,
            attendees: meeting.attendees,
            attendee_count: 1,
        }
    );
}
