use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs, todays_agenda,
};
use scribe_tray::calendar::agenda_cache::AgendaCache;
use scribe_tray::calendar::agenda_snapshot::FetchedAgenda;
use serde_json::{Value, json};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn fetched_with(ids: &[&str]) -> FetchedAgenda {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let events = ids.iter().map(|id| event(id)).collect();
    FetchedAgenda {
        agenda: todays_agenda(events, &day),
        day,
        at: moment("2026-10-26T08:55:00+05:30"),
    }
}

fn event(id: &str) -> CalendarEvent {
    CalendarEvent {
        meeting: Meeting {
            event_id: EventId::new(id),
            subject: format!("Meeting {id}"),
            starts_at: moment("2026-10-26T09:00:00+05:30"),
            ends_at: moment("2026-10-26T10:00:00+05:30"),
            organizer: Person::default(),
            attendees: Vec::new(),
            is_teams: false,
            is_recurring: false,
            response: Response::Accepted,
        },
        is_all_day: false,
        is_cancelled: false,
        show_as: ShowAs::Busy,
    }
}

fn sent(cache: &AgendaCache) -> Value {
    serde_json::to_value(cache.view()).unwrap()
}

#[test]
fn before_the_first_fetch_the_agenda_is_loading() {
    let cache = AgendaCache::new();

    assert_eq!(sent(&cache)["status"], json!("loading"));
}

#[test]
fn a_failure_before_any_fetch_leaves_nothing_to_show() {
    let cache = AgendaCache::new();

    cache.record_failure();

    assert_eq!(sent(&cache)["status"], json!("failed"));
}

#[test]
fn a_fetched_agenda_is_ready_and_fresh() {
    let cache = AgendaCache::new();

    cache.record_fetch(fetched_with(&["standup"]));

    let view = sent(&cache);
    assert_eq!(view["status"], json!("ready"));
    assert_eq!(view["stale"], json!(false));
}

#[test]
fn a_failed_refresh_keeps_the_last_fetched_meetings_and_marks_them_stale() {
    let cache = AgendaCache::new();
    cache.record_fetch(fetched_with(&["standup"]));

    cache.record_failure();

    let view = sent(&cache);
    assert_eq!(view["status"], json!("ready"));
    assert_eq!(view["stale"], json!(true));
    assert_eq!(view["blocks"][0]["event_id"], json!("standup"));
    assert_eq!(view["fetched_at"], json!("2026-10-26T08:55:00+05:30"));
}

#[test]
fn the_next_successful_fetch_clears_the_stale_mark() {
    let cache = AgendaCache::new();
    cache.record_fetch(fetched_with(&["standup"]));
    cache.record_failure();

    cache.record_fetch(fetched_with(&["standup", "review"]));

    assert_eq!(sent(&cache)["stale"], json!(false));
}

#[test]
fn clearing_forgets_the_fetched_meetings() {
    let cache = AgendaCache::new();
    cache.record_fetch(fetched_with(&["standup"]));

    cache.clear();

    assert_eq!(sent(&cache)["status"], json!("loading"));
}

#[test]
fn a_nudge_is_taken_once() {
    let cache = AgendaCache::new();

    cache.nudge();

    assert!(cache.take_nudge());
    assert!(!cache.take_nudge());
}
