use chrono::{DateTime, FixedOffset, NaiveDate};
use scribe_core::calendar::{
    ArmedAction, ArmedEntry, ArmedMeetings, CalendarEvent, DayBounds, EventId, Meeting, Person,
    Response, ShowAs, TodaysAgenda, WatchInputs, todays_agenda,
};
use scribe_tray::calendar::agenda_view::Arming;
use scribe_tray::calendar::armed::{
    ArmedStore, CallStartPopover, entries_from, popover_on_call_start, settings_value,
};
use scribe_tray::calendar::selection::{AutoStart, CallMatch, MeetingSelection};
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

fn standup() -> Meeting {
    Meeting {
        event_id: EventId::new("standup"),
        subject: "Standup".to_owned(),
        ..review()
    }
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

struct Poll<'a> {
    in_call: bool,
    matched: Option<&'a Meeting>,
    now: &'a str,
}

fn poll(store: &ArmedStore, when: &Poll) -> Vec<ArmedAction> {
    let agenda = agenda_of(vec![review(), standup()]);
    let armed = store.armed();
    store.take_due_actions(&WatchInputs {
        agenda: &agenda,
        armed: &armed,
        matched: when.matched,
        in_call: when.in_call,
        recording: false,
        now: moment(when.now),
    })
}

fn poll_without_a_call(store: &ArmedStore, now: &str) -> Vec<ArmedAction> {
    poll(
        store,
        &Poll {
            in_call: false,
            matched: None,
            now,
        },
    )
}

fn poll_in_a_call_matched_to(store: &ArmedStore, matched: Option<&Meeting>) -> Vec<ArmedAction> {
    poll(
        store,
        &Poll {
            in_call: true,
            matched,
            now: "2026-10-26T11:05:00+05:30",
        },
    )
}

fn review_armed() -> ArmedStore {
    let store = ArmedStore::new(ArmedMeetings::default());
    store.arm(review_id(), today());
    store
}

fn call_where_the_user_chose(id: Option<&EventId>) -> MeetingSelection {
    let selection = MeetingSelection::new();
    selection.note_call_started(CallMatch::decided(review(), vec![review(), standup()]));
    selection.choose(id).unwrap();
    selection
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

#[test]
fn choosing_the_armed_meeting_for_the_call_lets_it_record_by_itself() {
    let store = review_armed();
    let selection = call_where_the_user_chose(Some(&review_id()));

    let matched = selection.meeting_for_the_call(|| None);
    let actions = poll_in_a_call_matched_to(&store, matched.as_ref());

    assert_eq!(actions, vec![ArmedAction::StartRecording(review())]);
}

#[test]
fn choosing_another_meeting_for_the_call_holds_the_armed_one_back() {
    let store = review_armed();
    let selection = call_where_the_user_chose(Some(&EventId::new("standup")));

    let matched = selection.meeting_for_the_call(|| Some(review()));
    let actions = poll_in_a_call_matched_to(&store, matched.as_ref());

    assert!(actions.is_empty());
}

#[test]
fn choosing_not_a_calendar_meeting_for_the_call_holds_the_armed_one_back() {
    let store = review_armed();
    let selection = call_where_the_user_chose(None);

    let matched = selection.meeting_for_the_call(|| Some(review()));
    let actions = poll_in_a_call_matched_to(&store, matched.as_ref());

    assert!(actions.is_empty());
}

#[test]
fn a_call_seen_while_nothing_was_armed_excuses_a_meeting_armed_during_it() {
    let store = ArmedStore::new(ArmedMeetings::default());
    poll_in_a_call_matched_to(&store, None);
    store.arm(review_id(), today());

    let actions = poll_without_a_call(&store, "2026-10-26T11:20:00+05:30");

    assert!(actions.is_empty());
}
