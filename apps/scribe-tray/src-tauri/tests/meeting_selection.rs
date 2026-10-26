use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    CalendarEvent, DayBounds, EventId, MatchAnchor, Meeting, Person, Response, ShowAs,
    TodaysAgenda, todays_agenda,
};
use scribe_core::recording::SessionId;
use scribe_tray::calendar::selection::{MeetingSelection, chosen_meeting};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn meeting(id: &str, starts_at: &str, ends_at: &str) -> Meeting {
    Meeting {
        event_id: EventId::new(id),
        subject: id.to_owned(),
        starts_at: moment(starts_at),
        ends_at: moment(ends_at),
        organizer: Person::default(),
        attendees: Vec::new(),
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    }
}

fn review() -> Meeting {
    meeting(
        "review",
        "2026-10-26T11:00:00+05:30",
        "2026-10-26T11:30:00+05:30",
    )
}

fn agenda_of(meetings: Vec<Meeting>) -> TodaysAgenda {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let events = meetings
        .into_iter()
        .map(|meeting| CalendarEvent {
            meeting,
            is_all_day: false,
            is_cancelled: false,
            show_as: ShowAs::Busy,
        })
        .collect();
    todays_agenda(events, &day)
}

fn anchor_at(text: &str) -> MatchAnchor {
    MatchAnchor::new(moment(text))
}

fn session(text: &str) -> SessionId {
    SessionId::parse(text).unwrap()
}

#[test]
fn a_recording_during_a_meeting_is_linked_to_it() {
    let agenda = agenda_of(vec![review()]);

    let chosen = chosen_meeting(Some(&agenda), &anchor_at("2026-10-26T10:58:00+05:30"), &[]);

    assert_eq!(chosen, Some(review()));
}

#[test]
fn a_recording_with_no_meeting_around_it_is_linked_to_nothing() {
    let agenda = agenda_of(vec![review()]);

    let chosen = chosen_meeting(Some(&agenda), &anchor_at("2026-10-26T15:00:00+05:30"), &[]);

    assert_eq!(chosen, None);
}

#[test]
fn a_recording_that_fits_two_meetings_equally_is_linked_to_neither() {
    let other = meeting(
        "other",
        "2026-10-26T11:00:00+05:30",
        "2026-10-26T11:30:00+05:30",
    );
    let agenda = agenda_of(vec![review(), other]);

    let chosen = chosen_meeting(Some(&agenda), &anchor_at("2026-10-26T11:05:00+05:30"), &[]);

    assert_eq!(chosen, None);
}

#[test]
fn a_recording_that_fits_two_meetings_is_linked_to_the_one_the_teams_window_names() {
    let other = meeting(
        "other",
        "2026-10-26T11:00:00+05:30",
        "2026-10-26T11:30:00+05:30",
    );
    let agenda = agenda_of(vec![review(), other]);
    let titles = vec!["Review | Microsoft Teams".to_owned()];

    let chosen = chosen_meeting(
        Some(&agenda),
        &anchor_at("2026-10-26T11:05:00+05:30"),
        &titles,
    );

    assert_eq!(chosen, Some(review()));
}

#[test]
fn a_recording_with_no_agenda_to_look_at_is_linked_to_nothing() {
    let chosen = chosen_meeting(None, &anchor_at("2026-10-26T11:05:00+05:30"), &[]);

    assert_eq!(chosen, None);
}

#[test]
fn the_meeting_chosen_for_a_recording_is_handed_over_once_for_that_recording() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.begin(recording.clone(), Some(review()));

    assert_eq!(selection.take_for(&recording), Some(review()));
    assert_eq!(selection.take_for(&recording), None);
}

#[test]
fn another_recording_does_not_receive_the_meeting_chosen_for_this_one() {
    let selection = MeetingSelection::new();
    selection.begin(session("2026-10-26-110500"), Some(review()));

    assert_eq!(selection.take_for(&session("2026-10-26-120000")), None);
}

#[test]
fn a_new_recording_replaces_the_previous_choice() {
    let selection = MeetingSelection::new();
    selection.begin(session("2026-10-26-110500"), Some(review()));
    selection.begin(session("2026-10-26-120000"), None);

    assert_eq!(selection.take_for(&session("2026-10-26-110500")), None);
}
