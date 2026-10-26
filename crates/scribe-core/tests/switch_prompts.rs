use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    ArmedMeetings, CalendarEvent, DayBounds, EventId, Meeting, Person, Response, ShowAs,
    SwitchInputs, SwitchPrompts, TITLE_RECHECK_INTERVAL, TodaysAgenda, todays_agenda,
};
use std::time::Duration;

const WHEN_THE_RETRO_STARTS: &str = "2026-10-26T11:30:00+05:30";
const JUST_BEFORE_THE_RETRO: &str = "2026-10-26T11:29:00+05:30";
const AFTER_THE_RETRO_ENDED: &str = "2026-10-26T12:05:00+05:30";

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

fn retro_armed() -> ArmedMeetings {
    let mut armed = ArmedMeetings::new();
    armed.arm(
        EventId::new("retro"),
        moment(WHEN_THE_RETRO_STARTS).date_naive(),
    );
    armed
}

fn offered(prompts: &mut SwitchPrompts, inputs: &SwitchInputs) -> Option<(String, String)> {
    prompts
        .due(inputs)
        .map(|prompt| (prompt.from.subject, prompt.to.subject))
}

fn due_for(
    prompts: &mut SwitchPrompts,
    armed: &ArmedMeetings,
    now: &str,
) -> Option<(String, String)> {
    let (agenda, linked) = (agenda(), review());
    offered(
        prompts,
        &SwitchInputs {
            linked: &linked,
            agenda: &agenda,
            armed,
            titled: None,
            now: moment(now),
        },
    )
}

fn due_with_title(
    prompts: &mut SwitchPrompts,
    linked: &Meeting,
    titled: &Meeting,
) -> Option<(String, String)> {
    let agenda = agenda();
    offered(
        prompts,
        &SwitchInputs {
            linked,
            agenda: &agenda,
            armed: &ArmedMeetings::new(),
            titled: Some(titled),
            now: moment(JUST_BEFORE_THE_RETRO),
        },
    )
}

#[test]
fn an_armed_meeting_that_has_started_is_offered_while_another_is_recorded() {
    let mut prompts = SwitchPrompts::new();

    let offered = due_for(&mut prompts, &retro_armed(), WHEN_THE_RETRO_STARTS);

    assert_eq!(
        offered,
        Some(("DXC Review".to_owned(), "Sprint Retro".to_owned()))
    );
}

#[test]
fn an_armed_meeting_is_not_offered_before_it_starts() {
    let mut prompts = SwitchPrompts::new();

    let offered = due_for(&mut prompts, &retro_armed(), JUST_BEFORE_THE_RETRO);

    assert_eq!(offered, None);
}

#[test]
fn an_armed_meeting_that_already_ended_is_not_offered() {
    let mut prompts = SwitchPrompts::new();

    let offered = due_for(&mut prompts, &retro_armed(), AFTER_THE_RETRO_ENDED);

    assert_eq!(offered, None);
}

#[test]
fn a_meeting_named_in_the_teams_window_titles_is_offered() {
    let mut prompts = SwitchPrompts::new();

    let offered = due_with_title(&mut prompts, &review(), &retro());

    assert_eq!(
        offered,
        Some(("DXC Review".to_owned(), "Sprint Retro".to_owned()))
    );
}

#[test]
fn a_meeting_is_offered_only_once_even_after_the_user_keeps_recording() {
    let mut prompts = SwitchPrompts::new();
    let armed = retro_armed();
    due_for(&mut prompts, &armed, WHEN_THE_RETRO_STARTS);

    let offered_again = due_for(&mut prompts, &armed, WHEN_THE_RETRO_STARTS);

    assert_eq!(offered_again, None);
}

#[test]
fn a_meeting_that_is_neither_armed_nor_named_is_never_offered() {
    let mut prompts = SwitchPrompts::new();

    let offered = due_for(&mut prompts, &ArmedMeetings::new(), WHEN_THE_RETRO_STARTS);

    assert_eq!(offered, None);
}

#[test]
fn the_meeting_being_recorded_is_never_offered() {
    let mut prompts = SwitchPrompts::new();
    let review = review();

    let offered = due_with_title(&mut prompts, &review, &review);

    assert_eq!(offered, None);
}

#[test]
fn titles_are_reread_every_thirty_seconds() {
    assert_eq!(TITLE_RECHECK_INTERVAL, Duration::from_secs(30));
}
