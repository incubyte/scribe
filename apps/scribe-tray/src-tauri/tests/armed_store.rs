use chrono::{DateTime, FixedOffset, NaiveDate};
use scribe_core::calendar::{
    ArmedAction, ArmedEntry, ArmedMeetings, CalendarEvent, DayBounds, EventId, Meeting, Person,
    Response, ShowAs, WatchInputs, todays_agenda,
};
use scribe_tray::calendar::agenda_view::Arming;
use scribe_tray::calendar::armed::{
    ArmedStore, CallStartPopover, entries_from, popover_on_call_start, settings_value,
};
use scribe_tray::calendar::selection::AutoStart;
use serde_json::json;

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()
}

fn yesterday() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 25).unwrap()
}

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn review_id() -> EventId {
    EventId::new("review")
}

fn review() -> Meeting {
    Meeting {
        event_id: review_id(),
        subject: "DXC Review".to_owned(),
        starts_at: moment("2026-10-26T11:00:00+05:30"),
        ends_at: moment("2026-10-26T11:30:00+05:30"),
        organizer: Person::default(),
        attendees: Vec::new(),
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    }
}

fn poll_without_a_call(store: &ArmedStore, now: &str) -> Vec<ArmedAction> {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let event = CalendarEvent {
        meeting: review(),
        is_all_day: false,
        is_cancelled: false,
        show_as: ShowAs::Busy,
    };
    let agenda = todays_agenda(vec![event], &day);
    let armed = store.armed();
    store.take_due_actions(&WatchInputs {
        agenda: &agenda,
        armed: &armed,
        matched: None,
        in_call: false,
        recording: false,
        now: moment(now),
    })
}

#[test]
fn the_settings_keep_only_the_event_id_and_the_date_of_each_armed_meeting() {
    let mut armed = ArmedMeetings::default();
    armed.arm(review_id(), today());

    assert_eq!(
        settings_value(&armed),
        json!([{ "event_id": "review", "date": "2026-10-26" }])
    );
}

#[test]
fn armed_meetings_read_back_from_the_settings_as_they_were_saved() {
    let saved = json!([{ "event_id": "review", "date": "2026-10-26" }]);

    assert_eq!(
        entries_from(Some(&saved)),
        vec![ArmedEntry {
            event_id: review_id(),
            date: today(),
        }]
    );
}

#[test]
fn unreadable_settings_entries_are_skipped() {
    let saved = json!([
        { "event_id": "review", "date": "not a date" },
        { "date": "2026-10-26" },
        "review",
        { "event_id": "standup", "date": "2026-10-26" }
    ]);

    assert_eq!(
        entries_from(Some(&saved)),
        vec![ArmedEntry {
            event_id: EventId::new("standup"),
            date: today(),
        }]
    );
}

#[test]
fn settings_without_armed_meetings_arm_nothing() {
    assert!(entries_from(None).is_empty());
    assert!(entries_from(Some(&json!({ "review": true }))).is_empty());
}

#[test]
fn the_first_read_of_a_new_day_prunes_earlier_days_and_asks_to_be_saved() {
    let read_yesterday = ArmedMeetings::for_day(
        vec![
            ArmedEntry {
                event_id: EventId::new("yesterdays"),
                date: yesterday(),
            },
            ArmedEntry {
                event_id: review_id(),
                date: today(),
            },
        ],
        yesterday(),
    );
    let store = ArmedStore::new(read_yesterday);

    assert!(store.prune_for(today()));
    assert!(!store.prune_for(today()));
    assert_eq!(
        settings_value(&store.armed()),
        json!([{ "event_id": "review", "date": "2026-10-26" }])
    );
}

#[test]
fn an_armed_meeting_is_shown_armed_until_it_is_missed() {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());

    assert_eq!(store.arming_of(&review_id()), Arming::Armed);

    poll_without_a_call(&store, "2026-10-26T11:05:00+05:30");

    assert_eq!(store.arming_of(&review_id()), Arming::Missed);
}

#[test]
fn a_meeting_that_is_not_armed_is_shown_off() {
    let store = ArmedStore::new(ArmedMeetings::default());

    assert_eq!(store.arming_of(&review_id()), Arming::Off);
}

#[test]
fn switching_a_missed_meeting_off_and_on_arms_it_afresh() {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());
    poll_without_a_call(&store, "2026-10-26T11:05:00+05:30");

    store.disarm(&review_id());
    store.arm(review_id(), today());

    assert_eq!(store.arming_of(&review_id()), Arming::Armed);
}

#[test]
fn a_recording_cut_short_by_sleep_is_armed_again_after_wake() {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());
    store.note_recording(&review_id());
    store.remember_interrupted(review_id());
    store.note_recorded(&review_id());

    store.resume_interrupted();
    let actions = poll_without_a_call(&store, "2026-10-26T11:20:00+05:30");

    assert_eq!(actions, vec![ArmedAction::ReportMissed(review())]);
}

#[test]
fn a_recording_that_ended_normally_is_not_armed_again_after_wake() {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());
    store.note_recording(&review_id());
    store.note_recorded(&review_id());

    store.resume_interrupted();
    let actions = poll_without_a_call(&store, "2026-10-26T11:20:00+05:30");

    assert!(actions.is_empty());
}

#[test]
fn switching_to_an_armed_meeting_records_it_as_started_by_itself() {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());

    assert_eq!(store.start_for(&review_id()), AutoStart::Yes);
}

#[test]
fn switching_to_a_meeting_that_is_not_armed_records_it_as_started_by_hand() {
    let store = ArmedStore::new(ArmedMeetings::default());

    assert_eq!(store.start_for(&review_id()), AutoStart::No);
}

#[test]
fn a_start_failure_shows_the_meeting_as_missed() {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());

    store.note_start_failed(&review_id());

    assert_eq!(store.arming_of(&review_id()), Arming::Missed);
}

#[test]
fn a_call_that_starts_an_armed_meeting_by_itself_keeps_the_popover_closed() {
    let due = [ArmedAction::StartRecording(review())];

    assert_eq!(popover_on_call_start(&due), CallStartPopover::StayClosed);
}

#[test]
fn a_call_with_nothing_starting_by_itself_opens_the_popover() {
    assert_eq!(popover_on_call_start(&[]), CallStartPopover::Open);
}

#[test]
fn a_missed_meeting_reported_as_a_call_starts_still_opens_the_popover() {
    let due = [ArmedAction::ReportMissed(review())];

    assert_eq!(popover_on_call_start(&due), CallStartPopover::Open);
}
