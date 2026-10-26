use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs, todays_agenda,
};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn today() -> DayBounds {
    DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    }
}

fn event(id: &str, starts_at: &str, ends_at: &str) -> CalendarEvent {
    CalendarEvent {
        meeting: Meeting {
            event_id: EventId::new(id),
            subject: format!("Meeting {id}"),
            starts_at: moment(starts_at),
            ends_at: moment(ends_at),
            organizer: Person {
                name: "Asha".to_owned(),
                email: "asha@incubyte.co".to_owned(),
            },
            attendees: Vec::new(),
            is_teams: true,
            is_recurring: false,
            response: Response::Accepted,
        },
        is_all_day: false,
        is_cancelled: false,
        show_as: ShowAs::Busy,
    }
}

fn standup() -> CalendarEvent {
    event(
        "standup",
        "2026-10-26T09:00:00+05:30",
        "2026-10-26T09:15:00+05:30",
    )
}

fn ids(events: Vec<CalendarEvent>) -> Vec<String> {
    todays_agenda(events, &today())
        .meetings()
        .iter()
        .map(|meeting| meeting.event_id.as_str().to_owned())
        .collect()
}

#[test]
fn an_all_day_event_is_left_out() {
    let all_day = CalendarEvent {
        is_all_day: true,
        ..standup()
    };

    assert!(ids(vec![all_day]).is_empty());
}

#[test]
fn a_cancelled_event_is_left_out() {
    let cancelled = CalendarEvent {
        is_cancelled: true,
        ..standup()
    };

    assert!(ids(vec![cancelled]).is_empty());
}

#[test]
fn an_event_the_user_declined_is_left_out() {
    let mut declined = standup();
    declined.meeting.response = Response::Declined;

    assert!(ids(vec![declined]).is_empty());
}

#[test]
fn an_event_shown_as_free_is_left_out() {
    let free = CalendarEvent {
        show_as: ShowAs::Free,
        ..standup()
    };

    assert!(ids(vec![free]).is_empty());
}

#[test]
fn a_tentative_meeting_stays() {
    let mut tentative = standup();
    tentative.meeting.response = Response::Tentative;
    tentative.show_as = ShowAs::Tentative;

    assert_eq!(ids(vec![tentative]), ["standup"]);
}

#[test]
fn a_recurring_occurrence_stays() {
    let mut occurrence = standup();
    occurrence.meeting.is_recurring = true;

    assert_eq!(ids(vec![occurrence]), ["standup"]);
}

#[test]
fn meetings_are_listed_in_start_order() {
    let late = event(
        "review",
        "2026-10-26T16:00:00+05:30",
        "2026-10-26T17:00:00+05:30",
    );

    assert_eq!(ids(vec![late, standup()]), ["standup", "review"]);
}

#[test]
fn a_meeting_that_ended_before_midnight_yesterday_is_left_out() {
    let yesterday = event(
        "old",
        "2026-10-25T09:00:00+05:30",
        "2026-10-25T10:00:00+05:30",
    );

    assert!(ids(vec![yesterday]).is_empty());
}

#[test]
fn a_meeting_running_past_midnight_stays() {
    let late_night = event(
        "deploy",
        "2026-10-26T23:30:00+05:30",
        "2026-10-27T00:30:00+05:30",
    );

    assert_eq!(ids(vec![late_night]), ["deploy"]);
}

#[test]
fn a_meeting_starting_at_the_next_midnight_is_left_out() {
    let tomorrow = event(
        "next",
        "2026-10-27T00:00:00+05:30",
        "2026-10-27T01:00:00+05:30",
    );

    assert!(ids(vec![tomorrow]).is_empty());
}

#[test]
fn no_events_make_an_empty_agenda() {
    assert!(todays_agenda(Vec::new(), &today()).meetings().is_empty());
}
