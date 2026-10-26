use chrono::DateTime;
use scribe_core::calendar::{EventId, Meeting, MeetingLink, Person, Response};
use scribe_core::recording::{SessionHeader, SessionId, SessionStatus, default_title};

#[test]
fn a_date_and_time_stamp_is_a_valid_session_id() {
    let id = SessionId::parse("2026-10-17-143005").unwrap();

    assert_eq!(id.as_str(), "2026-10-17-143005");
}

#[test]
fn a_same_second_suffix_is_a_valid_session_id() {
    assert!(SessionId::parse("2026-10-17-143005-2").is_ok());
    assert!(SessionId::parse("2026-10-17-143005-999").is_ok());
}

#[test]
fn an_id_with_a_path_separator_is_rejected() {
    assert!(SessionId::parse("2026-10-17/143005").is_err());
    assert!(SessionId::parse("2026-10-17-143005\\x").is_err());
}

#[test]
fn an_id_that_climbs_out_of_the_folder_is_rejected() {
    assert!(SessionId::parse("../2026-10-17-143005").is_err());
    assert!(SessionId::parse("..").is_err());
}

#[test]
fn an_empty_id_is_rejected() {
    assert!(SessionId::parse("").is_err());
}

#[test]
fn an_over_long_suffix_or_malformed_stamp_is_rejected() {
    assert!(SessionId::parse("2026-10-17-143005-1000").is_err());
    assert!(SessionId::parse("2026-10-17-1430").is_err());
    assert!(SessionId::parse("2026-10-17-143005-").is_err());
    assert!(SessionId::parse("2026-1O-17-143005").is_err());
}

#[test]
fn a_session_is_named_after_its_local_start_time() {
    let started_at = DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap();

    let id = SessionId::from_start(started_at);

    assert_eq!(id.as_str(), "2026-10-17-143005");
    assert_eq!(id.with_collision_suffix(2).as_str(), "2026-10-17-143005-2");
}

#[test]
fn a_new_call_gets_a_default_title_from_its_start_time() {
    let started_at = DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap();

    assert_eq!(default_title(started_at), "Call 2026-10-17 14:30");
}

#[test]
fn a_new_session_header_describes_a_recording_in_progress() {
    let started_at = DateTime::parse_from_rfc3339("2026-10-17T14:30:05+05:30").unwrap();

    let header = SessionHeader::recording(started_at, "speechanalyzer", "en-US");

    assert_eq!(
        header,
        SessionHeader {
            id: SessionId::parse("2026-10-17-143005").unwrap(),
            title: "Call 2026-10-17 14:30".to_owned(),
            started_at,
            ended_at: None,
            status: SessionStatus::Recording,
            engine: "speechanalyzer".to_owned(),
            language: "en-US".to_owned(),
            meeting: None,
        }
    );
}

#[test]
fn linking_a_meeting_retitles_the_session_with_the_meeting_subject() {
    let started_at = DateTime::parse_from_rfc3339("2026-10-26T10:58:00+05:30").unwrap();
    let link = MeetingLink::from_meeting(&Meeting {
        event_id: EventId::new("review"),
        subject: "DXC Review".to_owned(),
        starts_at: started_at,
        ends_at: started_at,
        organizer: Person::default(),
        attendees: Vec::new(),
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    });

    let header =
        SessionHeader::recording(started_at, "speechanalyzer", "en-US").with_meeting(link.clone());

    assert_eq!(header.title, "DXC Review");
    assert_eq!(header.meeting, Some(link.clone()));
    assert_eq!(header.summary().meeting, Some(link));
}

#[test]
fn a_session_id_knows_the_month_it_started_in() {
    let id = SessionId::parse("2026-10-19-114521").unwrap();

    assert_eq!(id.month(), "2026-10");
}

#[test]
fn a_collision_suffixed_id_keeps_its_month() {
    let id = SessionId::parse("2026-10-19-114521").unwrap();

    assert_eq!(id.with_collision_suffix(2).month(), "2026-10");
}
