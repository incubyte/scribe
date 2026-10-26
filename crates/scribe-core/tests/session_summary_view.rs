use chrono::DateTime;
use scribe_core::calendar::{EventId, Meeting, MeetingLink, Person, Response};
use scribe_core::library::{MeetingSummaryView, PersonView, SessionSummaryView};
use scribe_core::recording::{SessionId, SessionStatus, SessionSummary};

fn summary(duration_seconds: Option<u64>, status: SessionStatus) -> SessionSummary {
    SessionSummary {
        id: SessionId::parse("2026-10-17-143005").unwrap(),
        title: "Call 2026-10-17 14:30".to_owned(),
        started_at: DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap(),
        duration_seconds,
        status,
        meeting: None,
    }
}

#[test]
fn a_finished_session_is_shown_with_its_start_time_length_and_status_word() {
    let view = SessionSummaryView::from(summary(Some(1_961), SessionStatus::Complete));

    assert_eq!(
        view,
        SessionSummaryView {
            id: "2026-10-17-143005".to_owned(),
            title: "Call 2026-10-17 14:30".to_owned(),
            started_at: "2026-10-17T14:30:05+05:30".to_owned(),
            duration_seconds: Some(1_961),
            status: "complete",
            meeting: None,
        }
    );
}

#[test]
fn a_session_still_recording_is_shown_without_a_length() {
    let view = SessionSummaryView::from(summary(None, SessionStatus::Recording));

    assert_eq!(view.duration_seconds, None);
    assert_eq!(view.status, "recording");
}

fn linked_review() -> MeetingLink {
    let person = |name: &str| Person {
        name: name.to_owned(),
        email: format!("{name}@example.com"),
    };
    MeetingLink::from_meeting(&Meeting {
        event_id: EventId::new("review"),
        subject: "DXC Review".to_owned(),
        starts_at: DateTime::parse_from_rfc3339("2026-10-26T11:00:00+05:30").unwrap(),
        ends_at: DateTime::parse_from_rfc3339("2026-10-26T11:45:00+05:30").unwrap(),
        organizer: person("sneha"),
        attendees: vec![person("sapan")],
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    })
}

#[test]
fn a_session_linked_to_a_meeting_shows_the_meeting_s_people() {
    let linked = SessionSummary {
        meeting: Some(linked_review()),
        ..summary(Some(60), SessionStatus::Complete)
    };

    let view = SessionSummaryView::from(linked);

    assert_eq!(
        view.meeting,
        Some(MeetingSummaryView {
            subject: "DXC Review".to_owned(),
            organizer: PersonView {
                name: "sneha".to_owned(),
                email: "sneha@example.com".to_owned(),
            },
            attendees: vec![PersonView {
                name: "sapan".to_owned(),
                email: "sapan@example.com".to_owned(),
            }],
            attendee_count: 1,
        })
    );
}
