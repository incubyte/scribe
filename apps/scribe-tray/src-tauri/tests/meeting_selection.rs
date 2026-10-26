use chrono::{DateTime, FixedOffset, NaiveDate};
use scribe_core::calendar::{
    ArmedMeetings, CalendarEvent, DayBounds, EventId, MatchAnchor, MatchInputs, Meeting, Person,
    Response, ShowAs, TodaysAgenda, todays_agenda,
};
use scribe_core::recording::SessionId;
use scribe_tray::calendar::selection::{
    AutoStart, CallMatch, ChooseError, FinishedChoice, MeetingSelection, call_match,
    chosen_meeting, pending_or_settled,
};

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

const NOTHING_ARMED: &ArmedMeetings = &ArmedMeetings::new();

fn inputs<'a>(agenda: &'a TodaysAgenda, at: &str, titles: &'a [String]) -> MatchInputs<'a> {
    MatchInputs {
        agenda,
        anchor: anchor_at(at),
        titles,
        armed: NOTHING_ARMED,
    }
}

fn decided(meeting: Meeting) -> CallMatch {
    CallMatch::decided(meeting, Vec::new())
}

fn linked(meeting: Meeting) -> FinishedChoice {
    FinishedChoice {
        meeting,
        start: AutoStart::No,
    }
}

#[test]
fn a_recording_during_a_meeting_is_linked_to_it() {
    let agenda = agenda_of(vec![review()]);

    let chosen = chosen_meeting(Some(inputs(&agenda, "2026-10-26T10:58:00+05:30", &[])));

    assert_eq!(chosen, Some(review()));
}

#[test]
fn a_recording_with_no_meeting_around_it_is_linked_to_nothing() {
    let agenda = agenda_of(vec![review()]);

    let chosen = chosen_meeting(Some(inputs(&agenda, "2026-10-26T15:00:00+05:30", &[])));

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

    let chosen = chosen_meeting(Some(inputs(&agenda, "2026-10-26T11:05:00+05:30", &[])));

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

    let chosen = chosen_meeting(Some(inputs(&agenda, "2026-10-26T11:05:00+05:30", &titles)));

    assert_eq!(chosen, Some(review()));
}

#[test]
fn a_recording_with_no_agenda_to_look_at_is_linked_to_nothing() {
    let chosen = chosen_meeting(None);

    assert_eq!(chosen, None);
}

#[test]
fn the_meeting_chosen_for_a_recording_is_handed_over_once_for_that_recording() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.begin_from_call(recording.clone(), || Some(review()));

    assert_eq!(selection.take_for(&recording), Some(linked(review())));
    assert_eq!(selection.take_for(&recording), None);
}

#[test]
fn another_recording_does_not_receive_the_meeting_chosen_for_this_one() {
    let selection = MeetingSelection::new();
    selection.begin_from_call(session("2026-10-26-110500"), || Some(review()));

    assert_eq!(selection.take_for(&session("2026-10-26-120000")), None);
}

#[test]
fn a_new_recording_replaces_the_previous_choice() {
    let selection = MeetingSelection::new();
    selection.begin_from_call(session("2026-10-26-110500"), || Some(review()));
    selection.begin_from_call(session("2026-10-26-120000"), || None);

    assert_eq!(selection.take_for(&session("2026-10-26-110500")), None);
}

fn other() -> Meeting {
    meeting(
        "other",
        "2026-10-26T11:00:00+05:30",
        "2026-10-26T11:30:00+05:30",
    )
}

fn id(text: &str) -> EventId {
    EventId::new(text)
}

fn two_overlapping() -> CallMatch {
    let agenda = agenda_of(vec![review(), other()]);
    call_match(Some(inputs(&agenda, "2026-10-26T11:05:00+05:30", &[])))
}

#[test]
fn a_call_during_one_meeting_preselects_it() {
    let agenda = agenda_of(vec![review()]);

    let found = call_match(Some(inputs(&agenda, "2026-10-26T10:58:00+05:30", &[])));

    assert_eq!(found.state(), "matched");
    assert_eq!(found.selected(), Some(&review()));
}

#[test]
fn a_call_that_fits_two_meetings_offers_both_and_preselects_the_top_ranked() {
    let found = two_overlapping();

    assert_eq!(found.state(), "ambiguous");
    assert_eq!(found.candidates().len(), 2);
    assert_eq!(found.selected(), Some(&found.candidates()[0]));
}

#[test]
fn a_call_named_by_the_teams_window_still_offers_the_other_candidates() {
    let agenda = agenda_of(vec![review(), other()]);
    let titles = vec!["Review | Microsoft Teams".to_owned()];

    let found = call_match(Some(inputs(&agenda, "2026-10-26T11:05:00+05:30", &titles)));

    assert_eq!(found.state(), "matched");
    assert_eq!(found.selected(), Some(&review()));
    assert_eq!(found.candidates().len(), 2);
}

#[test]
fn a_call_with_no_meeting_around_it_has_nothing_to_confirm() {
    let agenda = agenda_of(vec![review()]);

    let found = call_match(Some(inputs(&agenda, "2026-10-26T15:00:00+05:30", &[])));

    assert_eq!(found.state(), "unmatched");
    assert!(found.candidates().is_empty());
    assert_eq!(found.selected(), None);
}

#[test]
fn choosing_a_candidate_selects_it() {
    let mut found = two_overlapping();

    found.choose(Some(&id("other"))).unwrap();

    assert_eq!(found.selected(), Some(&other()));
}

#[test]
fn choosing_not_a_calendar_meeting_selects_nothing() {
    let mut found = two_overlapping();

    found.choose(None).unwrap();

    assert_eq!(found.selected(), None);
}

#[test]
fn choosing_a_meeting_that_is_not_a_candidate_is_refused() {
    let mut found = two_overlapping();
    let before = found.clone();

    let outcome = found.choose(Some(&id("elsewhere")));

    assert_eq!(outcome, Err(ChooseError::NotACandidate));
    assert_eq!(found, before);
}

#[test]
fn a_recording_adopts_the_choice_made_for_the_call() {
    let mut pending = two_overlapping();
    pending.choose(Some(&id("other"))).unwrap();

    let adopted = pending_or_settled(Some(pending), || Some(review()));

    assert_eq!(adopted.selected(), Some(&other()));
}

#[test]
fn a_recording_adopts_not_a_calendar_meeting_without_asking_the_calendar_again() {
    let mut pending = two_overlapping();
    pending.choose(None).unwrap();

    let adopted = pending_or_settled(Some(pending), || Some(review()));

    assert_eq!(adopted.selected(), None);
}

#[test]
fn a_recording_without_a_call_falls_back_to_matching_the_calendar() {
    let adopted = pending_or_settled(None, || Some(review()));

    assert_eq!(adopted.selected(), Some(&review()));
}

#[test]
fn the_selection_offers_the_call_match_until_the_call_ends() {
    let selection = MeetingSelection::new();
    selection.note_call_started(two_overlapping());

    assert_eq!(selection.current(), Some(two_overlapping()));

    selection.note_call_ended();

    assert_eq!(selection.current(), None);
}

#[test]
fn choosing_before_recording_decides_the_meeting_the_recording_is_linked_to() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.choose(Some(&id("other"))).unwrap();

    selection.begin_from_call(recording.clone(), || None);

    assert_eq!(selection.take_for(&recording), Some(linked(other())));
}

#[test]
fn the_latest_choice_made_while_recording_wins() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.begin_from_call(recording.clone(), || None);

    selection.choose(Some(&id("other"))).unwrap();
    selection.choose(Some(&id("review"))).unwrap();

    assert_eq!(selection.take_for(&recording), Some(linked(review())));
}

#[test]
fn a_correction_made_while_recording_survives_the_call_ending() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.begin_from_call(recording.clone(), || None);
    selection.note_call_ended();

    selection.choose(Some(&id("other"))).unwrap();

    assert_eq!(selection.take_for(&recording), Some(linked(other())));
}

#[test]
fn choosing_when_there_is_no_call_is_refused() {
    let selection = MeetingSelection::new();

    assert_eq!(
        selection.choose(Some(&id("review"))),
        Err(ChooseError::NoCall)
    );
}

fn armed_other() -> ArmedMeetings {
    let mut armed = ArmedMeetings::default();
    armed.arm(id("other"), NaiveDate::from_ymd_opt(2026, 10, 26).unwrap());
    armed
}

#[test]
fn a_call_that_fits_two_meetings_matches_the_armed_one() {
    let agenda = agenda_of(vec![review(), other()]);
    let armed = armed_other();

    let found = call_match(Some(MatchInputs {
        armed: &armed,
        ..inputs(&agenda, "2026-10-26T11:05:00+05:30", &[])
    }));

    assert_eq!(found.state(), "matched");
    assert_eq!(found.selected(), Some(&other()));
}

#[test]
fn a_recording_started_for_an_armed_meeting_is_linked_to_it_and_marked_automatic() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.prepare(decided(other()), AutoStart::Yes);

    selection.begin_from_call(recording.clone(), || None);

    assert_eq!(selection.auto_started_session(), Some(recording.clone()));
    assert_eq!(
        selection.take_for(&recording),
        Some(FinishedChoice {
            meeting: other(),
            start: AutoStart::Yes,
        })
    );
}

#[test]
fn a_recording_started_for_a_meeting_shows_that_meeting_in_the_live_view() {
    let selection = MeetingSelection::new();
    selection.prepare(decided(review()), AutoStart::No);

    selection.begin_from_call(session("2026-10-26-110500"), || None);

    let shown = selection.current().unwrap();
    assert_eq!(shown.state(), "matched");
    assert_eq!(shown.selected(), Some(&review()));
}

#[test]
fn a_recording_started_by_hand_is_never_treated_as_automatic() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");

    selection.begin_from_call(recording.clone(), || Some(review()));

    assert_eq!(selection.auto_started_session(), None);
    assert_eq!(selection.take_for(&recording), Some(linked(review())));
}

#[test]
fn a_start_that_failed_does_not_decide_the_next_recording() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.prepare(decided(other()), AutoStart::Yes);
    selection.abandon_prepared();

    selection.begin_from_call(recording.clone(), || Some(review()));

    assert_eq!(selection.auto_started_session(), None);
    assert_eq!(selection.take_for(&recording), Some(linked(review())));
}

#[test]
fn the_meeting_being_recorded_is_known_until_the_recording_is_handed_over() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.prepare(decided(review()), AutoStart::No);
    selection.begin_from_call(recording.clone(), || None);

    assert_eq!(selection.active_meeting(), Some(review()));

    selection.take_for(&recording);

    assert_eq!(selection.active_meeting(), None);
}

#[test]
fn a_recording_started_for_a_meeting_still_offers_the_meetings_overlapping_it() {
    let selection = MeetingSelection::new();
    selection.prepare(
        CallMatch::decided(other(), vec![review(), other()]),
        AutoStart::Yes,
    );

    selection.begin_from_call(session("2026-10-26-110500"), || None);

    let shown = selection.current().unwrap();
    assert_eq!(shown.selected(), Some(&other()));
    assert_eq!(shown.candidates(), [other(), review()]);
}

#[test]
fn a_correction_made_while_recording_becomes_the_calls_choice_after_stop() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.begin_from_call(recording.clone(), || None);
    selection.choose(Some(&id("other"))).unwrap();

    selection.take_for(&recording);

    assert_eq!(selection.current().unwrap().selected(), Some(&other()));
}

#[test]
fn a_second_recording_in_the_same_call_keeps_the_correction_made_in_the_first() {
    let selection = MeetingSelection::new();
    let first = session("2026-10-26-110500");
    let second = session("2026-10-26-111500");
    selection.note_call_started(two_overlapping());
    selection.begin_from_call(first.clone(), || None);
    selection.choose(Some(&id("other"))).unwrap();
    selection.take_for(&first);

    selection.begin_from_call(second.clone(), || Some(review()));

    assert_eq!(selection.take_for(&second), Some(linked(other())));
}

#[test]
fn choosing_not_a_calendar_meeting_for_the_call_holds_back_armed_meetings() {
    let selection = MeetingSelection::new();
    selection.note_call_started(two_overlapping());

    selection.choose(None).unwrap();

    assert!(selection.is_call_chosen_by_user());
}

#[test]
fn choosing_not_a_calendar_meeting_on_a_recording_holds_back_armed_meetings_for_the_rest_of_the_call()
 {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.begin_from_call(recording.clone(), || None);
    selection.choose(None).unwrap();

    selection.take_for(&recording);

    assert!(selection.is_call_chosen_by_user());
}

#[test]
fn a_call_the_calendar_could_not_match_does_not_hold_back_armed_meetings() {
    let selection = MeetingSelection::new();

    selection.note_call_started(CallMatch::settled(None));

    assert!(!selection.is_call_chosen_by_user());
}

#[test]
fn a_new_call_forgets_that_the_last_one_was_not_a_calendar_meeting() {
    let selection = MeetingSelection::new();
    selection.note_call_started(two_overlapping());
    selection.choose(None).unwrap();

    selection.note_call_ended();

    assert!(!selection.is_call_chosen_by_user());
}

#[test]
fn choosing_another_meeting_for_the_call_holds_back_armed_meetings() {
    let selection = MeetingSelection::new();
    selection.note_call_started(two_overlapping());

    selection.choose(Some(&id("other"))).unwrap();

    assert!(selection.is_call_chosen_by_user());
}

#[test]
fn choosing_another_meeting_on_a_recording_holds_back_armed_meetings_for_the_rest_of_the_call() {
    let selection = MeetingSelection::new();
    let recording = session("2026-10-26-110500");
    selection.note_call_started(two_overlapping());
    selection.begin_from_call(recording.clone(), || None);
    selection.choose(Some(&id("other"))).unwrap();

    selection.take_for(&recording);

    assert!(selection.is_call_chosen_by_user());
}

#[test]
fn a_call_the_calendar_matched_by_itself_does_not_hold_back_armed_meetings() {
    let selection = MeetingSelection::new();

    selection.note_call_started(two_overlapping());

    assert!(!selection.is_call_chosen_by_user());
}
