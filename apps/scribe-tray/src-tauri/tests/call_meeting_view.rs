use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    ArmedMeetings, CalendarEvent, DayBounds, EventId, MatchAnchor, MatchInputs, Meeting, Person,
    Response, ShowAs, todays_agenda,
};
use scribe_tray::calendar::call_meeting_view::CallMeetingView;
use scribe_tray::calendar::selection::{CallMatch, call_match};
use serde_json::{Value, json};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn person(name: &str, email: &str) -> Person {
    Person {
        name: name.to_owned(),
        email: email.to_owned(),
    }
}

fn event(id: &str) -> CalendarEvent {
    CalendarEvent {
        meeting: Meeting {
            event_id: EventId::new(id),
            subject: format!("Meeting {id}"),
            starts_at: moment("2026-10-26T11:00:00+05:30"),
            ends_at: moment("2026-10-26T11:30:00+05:30"),
            organizer: person("Organizer", "organizer@example.com"),
            attendees: vec![person("Guest", "guest@example.com")],
            is_teams: true,
            is_recurring: false,
            response: Response::Accepted,
        },
        is_all_day: false,
        is_cancelled: false,
        show_as: ShowAs::Busy,
    }
}

fn match_for(ids: &[&str], at: &str) -> CallMatch {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let agenda = todays_agenda(ids.iter().map(|id| event(id)).collect(), &day);
    call_match(Some(MatchInputs {
        agenda: &agenda,
        anchor: MatchAnchor::new(moment(at)),
        titles: &[],
        armed: &ArmedMeetings::default(),
    }))
}

fn rendered(found: Option<&CallMatch>) -> Value {
    serde_json::to_value(CallMeetingView::of(found)).unwrap()
}

#[test]
fn a_matched_call_is_shown_with_its_meeting_selected() {
    let found = match_for(&["review"], "2026-10-26T10:58:00+05:30");

    let view = rendered(Some(&found));

    assert_eq!(view["state"], "matched");
    assert_eq!(view["selected_event_id"], "review");
    assert_eq!(view["not_a_meeting"], false);
    assert_eq!(
        view["candidates"],
        json!([{
            "event_id": "review",
            "subject": "Meeting review",
            "starts_at": "2026-10-26T11:00:00+05:30",
            "ends_at": "2026-10-26T11:30:00+05:30",
        }])
    );
}

#[test]
fn an_ambiguous_call_is_shown_with_every_candidate() {
    let found = match_for(&["a", "b"], "2026-10-26T11:05:00+05:30");

    let view = rendered(Some(&found));

    assert_eq!(view["state"], "ambiguous");
    assert_eq!(view["candidates"].as_array().unwrap().len(), 2);
}

#[test]
fn not_a_calendar_meeting_is_shown_as_the_selected_choice() {
    let mut found = match_for(&["a", "b"], "2026-10-26T11:05:00+05:30");
    found.choose(None).unwrap();

    let view = rendered(Some(&found));

    assert_eq!(view["selected_event_id"], Value::Null);
    assert_eq!(view["not_a_meeting"], true);
}

#[test]
fn no_call_is_shown_as_unmatched_with_nothing_to_choose() {
    let view = rendered(None);

    assert_eq!(view["state"], "unmatched");
    assert_eq!(view["candidates"], json!([]));
}

#[test]
fn attendees_never_reach_the_webview() {
    let found = match_for(&["review"], "2026-10-26T10:58:00+05:30");

    let text = rendered(Some(&found)).to_string();

    assert!(!text.contains("guest@example.com"));
    assert!(!text.contains("organizer@example.com"));
}
