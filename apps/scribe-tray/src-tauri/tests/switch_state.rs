use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    ArmedMeetings, CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs,
    SwitchInputs, TITLE_RECHECK_INTERVAL, TodaysAgenda, todays_agenda,
};
use scribe_tray::calendar::switch_state::{SwitchPromptView, SwitchState};

const WHEN_THE_RETRO_STARTS: &str = "2026-10-26T11:30:00+05:30";

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn meeting(id: &str, subject: &str, window: (&str, &str)) -> Meeting {
    Meeting {
        event_id: EventId::new(id),
        subject: subject.to_owned(),
        starts_at: moment(window.0),
        ends_at: moment(window.1),
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
        "DXC Review",
        ("2026-10-26T11:00:00+05:30", "2026-10-26T11:30:00+05:30"),
    )
}

fn retro() -> Meeting {
    meeting(
        "retro",
        "Sprint Retro",
        (WHEN_THE_RETRO_STARTS, "2026-10-26T12:00:00+05:30"),
    )
}

fn agenda() -> TodaysAgenda {
    let day = DayBounds {
        start: moment("2026-10-26T00:00:00+05:30"),
        end: moment("2026-10-27T00:00:00+05:30"),
    };
    let events = [review(), retro()]
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

fn armed_retro() -> ArmedMeetings {
    let mut armed = ArmedMeetings::new();
    armed.arm(
        EventId::new("retro"),
        moment(WHEN_THE_RETRO_STARTS).date_naive(),
    );
    armed
}

fn offer_retro(state: &SwitchState) -> Option<SwitchPromptView> {
    let (agenda, armed, linked) = (agenda(), armed_retro(), review());
    state.offer(&SwitchInputs {
        linked: &linked,
        agenda: &agenda,
        armed: &armed,
        titled: None,
        now: moment(WHEN_THE_RETRO_STARTS),
    })
}

#[test]
fn an_offered_switch_stays_open_until_the_user_answers() {
    let state = SwitchState::new();

    offer_retro(&state);

    assert_eq!(
        state.open_view(),
        Some(SwitchPromptView::new("DXC Review", "Sprint Retro"))
    );
}

#[test]
fn switching_hands_over_the_next_meeting_and_closes_the_prompt() {
    let state = SwitchState::new();
    offer_retro(&state);

    let next = state.take_for_switch();

    assert_eq!(
        next.map(|meeting| meeting.subject),
        Some("Sprint Retro".to_owned())
    );
    assert_eq!(state.open_view(), None);
}

#[test]
fn keeping_the_recording_closes_the_prompt_and_never_offers_that_meeting_again() {
    let state = SwitchState::new();
    offer_retro(&state);

    state.withdraw();

    assert_eq!(state.open_view(), None);
    assert_eq!(offer_retro(&state), None);
    assert_eq!(state.take_for_switch(), None);
}

#[test]
fn a_second_meeting_is_not_offered_while_a_prompt_is_still_open() {
    let state = SwitchState::new();
    offer_retro(&state);

    let again = offer_retro(&state);

    assert_eq!(again, None);
    assert_eq!(
        state.open_view(),
        Some(SwitchPromptView::new("DXC Review", "Sprint Retro"))
    );
}

#[test]
fn leaving_the_call_withdraws_an_open_prompt_without_switching() {
    let state = SwitchState::new();
    offer_retro(&state);

    let withdrawn = state.withdraw();

    assert!(withdrawn);
    assert_eq!(state.open_view(), None);
    assert!(!state.withdraw());
}

#[test]
fn titles_are_read_at_most_once_per_recheck_interval() {
    let state = SwitchState::new();
    let first = Instant::now();

    let at_first = state.title_recheck_due(first);
    let just_after = state.title_recheck_due(first + Duration::from_secs(2));
    let after_the_interval = state.title_recheck_due(first + TITLE_RECHECK_INTERVAL);

    assert!(at_first);
    assert!(!just_after);
    assert!(after_the_interval);
}

#[test]
fn a_switch_is_in_progress_from_handing_over_the_next_meeting_until_it_finishes() {
    let state = SwitchState::new();
    offer_retro(&state);

    state.take_for_switch();
    let while_switching = state.is_switching();
    state.finish_switch();

    assert!(while_switching);
    assert!(!state.is_switching());
}

#[test]
fn no_switch_is_in_progress_when_there_was_no_prompt_to_switch_from() {
    let state = SwitchState::new();

    state.take_for_switch();

    assert!(!state.is_switching());
}
