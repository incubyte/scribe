use chrono::{DateTime, FixedOffset, NaiveDate};
use scribe_core::calendar::{
    ArmedAction, ArmedEntry, ArmedMeetings, ArmedState, ArmedWatch, CalendarEvent, DayBounds,
    EventId, Meeting, MissedReason, Person, Response, ShowAs, TodaysAgenda, WatchInputs,
    todays_agenda,
};

const START_OF_REVIEW: &str = "2026-10-26T11:00:00+05:30";
const END_OF_REVIEW: &str = "2026-10-26T11:30:00+05:30";
const BEFORE_THE_LEAD: &str = "2026-10-26T10:49:00+05:30";
const TWO_MINUTES_EARLY: &str = "2026-10-26T10:58:00+05:30";
const FOUR_MINUTES_IN: &str = "2026-10-26T11:04:00+05:30";
const FIVE_MINUTES_IN: &str = "2026-10-26T11:05:00+05:30";
const TEN_MINUTES_IN: &str = "2026-10-26T11:10:00+05:30";
const TWENTY_MINUTES_IN: &str = "2026-10-26T11:20:00+05:30";
const AFTER_THE_END: &str = "2026-10-26T11:30:00+05:30";

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 26).unwrap()
}

fn yesterday() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 10, 25).unwrap()
}

fn review() -> Meeting {
    Meeting {
        event_id: EventId::new("review"),
        subject: "DXC Review".to_owned(),
        starts_at: moment(START_OF_REVIEW),
        ends_at: moment(END_OF_REVIEW),
        organizer: Person::default(),
        attendees: Vec::new(),
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    }
}

fn review_id() -> EventId {
    EventId::new("review")
}

fn agenda_with_review() -> TodaysAgenda {
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
    todays_agenda(vec![event], &day)
}

fn review_armed() -> ArmedMeetings {
    let mut armed = ArmedMeetings::default();
    armed.arm(review_id(), today());
    armed
}

struct Poll {
    in_call: bool,
    matched: Option<Meeting>,
    recording: bool,
}

const NO_CALL: Poll = Poll {
    in_call: false,
    matched: None,
    recording: false,
};

fn call_in_review() -> Poll {
    Poll {
        in_call: true,
        matched: Some(review()),
        recording: false,
    }
}

fn poll_at(watch: &mut ArmedWatch, poll: &Poll, now: &str) -> Vec<ArmedAction> {
    let agenda = agenda_with_review();
    let armed = review_armed();
    watch.take_due_actions(&WatchInputs {
        agenda: &agenda,
        armed: &armed,
        matched: poll.matched.as_ref(),
        in_call: poll.in_call,
        recording: poll.recording,
        now: moment(now),
    })
}

#[test]
fn arming_a_meeting_keeps_its_id_with_todays_date() {
    let armed = review_armed();

    assert!(armed.is_armed(&review_id()));
    assert_eq!(
        armed.entries(),
        [ArmedEntry {
            event_id: review_id(),
            date: today(),
        }]
    );
}

#[test]
fn meetings_armed_on_an_earlier_day_are_pruned_when_read_today() {
    let entries = vec![
        ArmedEntry {
            event_id: EventId::new("yesterdays"),
            date: yesterday(),
        },
        ArmedEntry {
            event_id: review_id(),
            date: today(),
        },
    ];

    let armed = ArmedMeetings::for_day(entries, today());

    assert!(!armed.is_armed(&EventId::new("yesterdays")));
    assert!(armed.is_armed(&review_id()));
    assert_eq!(armed.entries().len(), 1);
}

#[test]
fn arming_the_same_meeting_twice_keeps_one_entry() {
    let mut armed = review_armed();

    armed.arm(review_id(), today());

    assert_eq!(armed.entries().len(), 1);
}

#[test]
fn disarming_a_meeting_forgets_it() {
    let mut armed = review_armed();

    armed.disarm(&review_id());

    assert!(!armed.is_armed(&review_id()));
    assert!(armed.entries().is_empty());
}

#[test]
fn a_call_in_the_armed_meetings_window_starts_recording_it() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &call_in_review(), TWO_MINUTES_EARLY);

    assert_eq!(actions, vec![ArmedAction::StartRecording(review())]);
    assert_eq!(watch.state_of(&review_id()), ArmedState::Recording);
}

#[test]
fn without_a_call_nothing_starts_even_at_the_meetings_start() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &NO_CALL, START_OF_REVIEW);

    assert!(actions.is_empty());
    assert_eq!(watch.state_of(&review_id()), ArmedState::Armed);
}

#[test]
fn a_call_matched_to_another_meeting_does_not_start_the_armed_one() {
    let mut watch = ArmedWatch::new();
    let elsewhere = Poll {
        in_call: true,
        matched: None,
        recording: false,
    };

    let actions = poll_at(&mut watch, &elsewhere, START_OF_REVIEW);

    assert!(actions.is_empty());
}

#[test]
fn a_call_before_the_meetings_window_does_not_start_it() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &call_in_review(), BEFORE_THE_LEAD);

    assert!(actions.is_empty());
}

#[test]
fn four_minutes_after_the_start_without_a_call_is_not_yet_missed() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &NO_CALL, FOUR_MINUTES_IN);

    assert!(actions.is_empty());
}

#[test]
fn five_minutes_after_the_start_without_a_call_is_reported_missed_once() {
    let mut watch = ArmedWatch::new();

    let first = poll_at(&mut watch, &NO_CALL, FIVE_MINUTES_IN);
    let second = poll_at(&mut watch, &NO_CALL, TWENTY_MINUTES_IN);

    assert_eq!(first, vec![ArmedAction::ReportMissed(review())]);
    assert!(second.is_empty());
    assert_eq!(
        watch.state_of(&review_id()),
        ArmedState::Missed(MissedReason::NoCallInTime)
    );
}

#[test]
fn a_call_still_in_progress_at_the_first_poll_after_wake_starts_recording() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &call_in_review(), TWENTY_MINUTES_IN);

    assert_eq!(actions, vec![ArmedAction::StartRecording(review())]);
}

fn call_in_another_meeting() -> Poll {
    Poll {
        in_call: true,
        matched: None,
        recording: true,
    }
}

#[test]
fn a_meeting_missed_for_want_of_a_call_starts_when_its_call_begins_late() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &NO_CALL, FIVE_MINUTES_IN);

    let actions = poll_at(&mut watch, &call_in_review(), TWENTY_MINUTES_IN);

    assert_eq!(actions, vec![ArmedAction::StartRecording(review())]);
    assert_eq!(watch.state_of(&review_id()), ArmedState::Recording);
}

#[test]
fn a_late_start_that_fails_is_final() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &NO_CALL, FIVE_MINUTES_IN);
    poll_at(&mut watch, &call_in_review(), TEN_MINUTES_IN);

    watch.note_start_failed(&review_id());
    let actions = poll_at(&mut watch, &call_in_review(), TWENTY_MINUTES_IN);

    assert!(actions.is_empty());
    assert_eq!(
        watch.state_of(&review_id()),
        ArmedState::Missed(MissedReason::StartFailed)
    );
}

#[test]
fn a_late_call_after_the_meeting_has_ended_does_not_start_a_missed_meeting() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &NO_CALL, FIVE_MINUTES_IN);

    let actions = poll_at(&mut watch, &call_in_review(), AFTER_THE_END);

    assert!(actions.is_empty());
}

#[test]
fn an_armed_meeting_that_starts_during_another_call_is_not_reported_missed() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &call_in_another_meeting(), FIVE_MINUTES_IN);

    assert!(actions.is_empty());
    assert_eq!(watch.state_of(&review_id()), ArmedState::Armed);
}

#[test]
fn an_armed_meeting_overlapped_by_another_call_is_not_reported_missed_when_that_call_ends() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &call_in_another_meeting(), FOUR_MINUTES_IN);

    let actions = poll_at(&mut watch, &NO_CALL, TWENTY_MINUTES_IN);

    assert!(actions.is_empty());
}

#[test]
fn a_call_that_ended_before_the_meeting_started_does_not_excuse_missing_it() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &call_in_another_meeting(), TWO_MINUTES_EARLY);

    let actions = poll_at(&mut watch, &NO_CALL, FIVE_MINUTES_IN);

    assert_eq!(actions, vec![ArmedAction::ReportMissed(review())]);
}

#[test]
fn a_meeting_that_has_ended_never_starts() {
    let mut watch = ArmedWatch::new();

    let actions = poll_at(&mut watch, &call_in_review(), AFTER_THE_END);

    assert!(actions.is_empty());
}

#[test]
fn a_continuing_call_does_not_restart_a_finished_recording() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &call_in_review(), TWO_MINUTES_EARLY);
    watch.note_recorded(&review_id());

    let actions = poll_at(&mut watch, &call_in_review(), TWENTY_MINUTES_IN);

    assert!(actions.is_empty());
    assert_eq!(watch.state_of(&review_id()), ArmedState::Recorded);
}

#[test]
fn a_recording_interrupted_by_sleep_starts_again_while_the_call_goes_on() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &call_in_review(), TWO_MINUTES_EARLY);
    watch.note_recorded(&review_id());

    watch.forget(&review_id());
    let actions = poll_at(&mut watch, &call_in_review(), TWENTY_MINUTES_IN);

    assert_eq!(actions, vec![ArmedAction::StartRecording(review())]);
}

#[test]
fn a_recording_that_finishes_after_the_interruption_was_noted_stays_armed() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &call_in_review(), TWO_MINUTES_EARLY);

    watch.forget(&review_id());
    watch.note_recorded(&review_id());

    assert_eq!(watch.state_of(&review_id()), ArmedState::Armed);
}

#[test]
fn a_failed_start_makes_the_meeting_missed_and_is_not_retried() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &call_in_review(), TWO_MINUTES_EARLY);

    watch.note_start_failed(&review_id());
    let actions = poll_at(&mut watch, &call_in_review(), FIVE_MINUTES_IN);

    assert_eq!(
        watch.state_of(&review_id()),
        ArmedState::Missed(MissedReason::StartFailed)
    );
    assert!(actions.is_empty());
}

#[test]
fn a_meeting_already_recording_by_hand_is_not_started_again() {
    let mut watch = ArmedWatch::new();
    watch.note_recording(&review_id());

    let actions = poll_at(&mut watch, &call_in_review(), TWO_MINUTES_EARLY);

    assert!(actions.is_empty());
}

#[test]
fn nothing_starts_while_another_recording_is_running() {
    let mut watch = ArmedWatch::new();
    let busy = Poll {
        recording: true,
        ..call_in_review()
    };

    let actions = poll_at(&mut watch, &busy, TWO_MINUTES_EARLY);

    assert!(actions.is_empty());
}

#[test]
fn a_meeting_being_recorded_in_its_call_is_not_reported_missed() {
    let mut watch = ArmedWatch::new();
    let busy = Poll {
        recording: true,
        ..call_in_review()
    };

    let actions = poll_at(&mut watch, &busy, FIVE_MINUTES_IN);

    assert!(actions.is_empty());
}

#[test]
fn a_meeting_that_is_not_armed_is_never_started() {
    let mut watch = ArmedWatch::new();
    let agenda = agenda_with_review();
    let nothing_armed = ArmedMeetings::default();
    let matched = review();

    let actions = watch.take_due_actions(&WatchInputs {
        agenda: &agenda,
        armed: &nothing_armed,
        matched: Some(&matched),
        in_call: true,
        recording: false,
        now: moment(TWO_MINUTES_EARLY),
    });

    assert!(actions.is_empty());
}

#[test]
fn forgetting_a_missed_meeting_arms_it_afresh() {
    let mut watch = ArmedWatch::new();
    poll_at(&mut watch, &NO_CALL, FIVE_MINUTES_IN);

    watch.forget(&review_id());

    assert_eq!(watch.state_of(&review_id()), ArmedState::Armed);
}
