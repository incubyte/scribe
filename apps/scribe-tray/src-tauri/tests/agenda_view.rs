use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs, todays_agenda,
};
use scribe_tray::calendar::agenda_snapshot::{AgendaSnapshot, FetchedAgenda};
use scribe_tray::calendar::agenda_view::{AgendaView, Arming};
use serde_json::{Value, json};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn event(id: &str, from: &str, to: &str) -> CalendarEvent {
    CalendarEvent {
        meeting: Meeting {
            event_id: EventId::new(id),
            subject: format!("Meeting {id}"),
            starts_at: moment(&format!("2026-10-26T{from}:00+05:30")),
            ends_at: moment(&format!("2026-10-26T{to}:00+05:30")),
            organizer: Person::default(),
            attendees: Vec::new(),
            is_teams: true,
            is_recurring: true,
            response: Response::Accepted,
        },
        is_all_day: false,
        is_cancelled: false,
        show_as: ShowAs::Busy,
    }
}

fn view_of(events: Vec<CalendarEvent>) -> Value {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let snapshot = AgendaSnapshot::after_fetch(FetchedAgenda {
        agenda: todays_agenda(events, &day),
        day,
        at: moment("2026-10-26T08:55:00+05:30"),
    });
    serde_json::to_value(AgendaView::of(&snapshot)).unwrap()
}

#[test]
fn a_block_carries_what_the_webview_needs_to_place_and_name_it() {
    let view = view_of(vec![event("standup", "09:30", "10:00")]);

    assert_eq!(
        view["blocks"][0],
        json!({
            "event_id": "standup",
            "subject": "Meeting standup",
            "starts_at": "2026-10-26T09:30:00+05:30",
            "ends_at": "2026-10-26T10:00:00+05:30",
            "is_teams": true,
            "is_recurring": true,
            "top_minutes": 570,
            "height_minutes": 30,
            "column": 0,
            "columns": 1,
            "armed": false,
            "armed_state": null
        })
    );
}

#[test]
fn the_view_names_the_day_it_is_laid_out_against() {
    let view = view_of(Vec::new());

    assert_eq!(view["day_start"], json!("2026-10-26T00:00:00+05:30"));
    assert_eq!(view["blocks"], json!([]));
}

#[test]
fn a_chip_lists_the_meetings_it_hides() {
    let view = view_of(vec![
        event("a", "09:00", "10:00"),
        event("b", "09:00", "10:00"),
        event("c", "09:15", "09:45"),
    ]);

    assert_eq!(
        view["overflow"],
        json!([{
            "top_minutes": 555,
            "height_minutes": 30,
            "hidden": [{
                "event_id": "c",
                "subject": "Meeting c",
                "starts_at": "2026-10-26T09:15:00+05:30",
                "ends_at": "2026-10-26T09:45:00+05:30"
            }]
        }])
    );
}

fn marked_view_of(events: Vec<CalendarEvent>, arming: fn(&EventId) -> Arming) -> Value {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let snapshot = AgendaSnapshot::after_fetch(FetchedAgenda {
        agenda: todays_agenda(events, &day),
        day,
        at: moment("2026-10-26T08:55:00+05:30"),
    });
    serde_json::to_value(AgendaView::of(&snapshot).marked(arming)).unwrap()
}

fn arming_shown(view: &Value, index: usize) -> Value {
    let block = &view["blocks"][index];
    json!([block["armed"], block["armed_state"]])
}

fn review_missed_standup_armed(id: &EventId) -> Arming {
    match id.as_str() {
        "review" => Arming::Missed,
        "standup" => Arming::Armed,
        _ => Arming::Off,
    }
}

#[test]
fn a_block_says_whether_its_meeting_is_armed_and_whether_it_was_missed() {
    let view = marked_view_of(
        vec![
            event("standup", "09:00", "09:30"),
            event("review", "11:00", "11:30"),
            event("lunch", "13:00", "14:00"),
        ],
        review_missed_standup_armed,
    );

    assert_eq!(arming_shown(&view, 0), json!([true, null]));
    assert_eq!(arming_shown(&view, 1), json!([true, "missed"]));
    assert_eq!(arming_shown(&view, 2), json!([false, null]));
}
