use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{
    CalendarEvent, DayBounds, EventId, MatchAnchor, MatchInputs, MatchOutcome, Meeting, Person,
    Response, ShowAs, TodaysAgenda, match_meeting, todays_agenda,
};

const START_OF_REVIEW: &str = "2026-10-26T11:00:00+05:30";
const END_OF_REVIEW: &str = "2026-10-26T11:30:00+05:30";

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
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
    meeting("review", START_OF_REVIEW, END_OF_REVIEW)
}

fn matched_at(meetings: Vec<Meeting>, anchor: &str) -> MatchOutcome {
    let agenda = agenda_of(meetings);
    match_meeting(&MatchInputs {
        agenda: &agenda,
        anchor: MatchAnchor::new(moment(anchor)),
    })
}

fn ids_of(meetings: &[Meeting]) -> Vec<&str> {
    meetings
        .iter()
        .map(|meeting| meeting.event_id.as_str())
        .collect()
}

#[test]
fn a_call_that_starts_two_minutes_early_matches_the_teams_meeting() {
    let outcome = matched_at(vec![review()], "2026-10-26T10:58:00+05:30");

    assert_eq!(outcome, MatchOutcome::Matched(review()));
}

#[test]
fn a_call_that_started_inside_a_meeting_stays_matched_to_it_when_a_later_meeting_has_begun() {
    let following = meeting("following", END_OF_REVIEW, "2026-10-26T12:00:00+05:30");
    let outcome = matched_at(vec![review(), following], "2026-10-26T11:05:00+05:30");

    assert_eq!(outcome, MatchOutcome::Matched(review()));
}

#[test]
fn a_call_with_no_meeting_around_it_matches_nothing() {
    let outcome = matched_at(vec![review()], "2026-10-26T14:00:00+05:30");

    assert_eq!(outcome, MatchOutcome::Unmatched);
}

#[test]
fn two_accepted_teams_meetings_at_once_leave_the_match_ambiguous_with_the_nearer_start_first() {
    let earlier = meeting("earlier", "2026-10-26T10:30:00+05:30", END_OF_REVIEW);
    let outcome = matched_at(vec![earlier, review()], "2026-10-26T10:55:00+05:30");

    let MatchOutcome::Ambiguous(ranked) = outcome else {
        panic!("expected an ambiguous match");
    };
    assert_eq!(ids_of(&ranked), vec!["review", "earlier"]);
}

#[test]
fn a_teams_meeting_outranks_one_that_is_not_on_teams() {
    let in_person = Meeting {
        is_teams: false,
        ..meeting("in-person", START_OF_REVIEW, END_OF_REVIEW)
    };
    let outcome = matched_at(vec![in_person, review()], "2026-10-26T11:05:00+05:30");

    assert_eq!(outcome, MatchOutcome::Matched(review()));
}

#[test]
fn an_accepted_meeting_outranks_a_tentative_one() {
    let tentative = Meeting {
        response: Response::Tentative,
        ..meeting("tentative", START_OF_REVIEW, END_OF_REVIEW)
    };
    let outcome = matched_at(vec![tentative, review()], "2026-10-26T11:05:00+05:30");

    assert_eq!(outcome, MatchOutcome::Matched(review()));
}

#[test]
fn a_tentative_meeting_outranks_one_the_user_has_not_answered() {
    let tentative = Meeting {
        response: Response::Tentative,
        ..meeting("tentative", START_OF_REVIEW, END_OF_REVIEW)
    };
    let unanswered = Meeting {
        response: Response::None,
        ..meeting("unanswered", START_OF_REVIEW, END_OF_REVIEW)
    };
    let outcome = matched_at(
        vec![unanswered, tentative.clone()],
        "2026-10-26T11:05:00+05:30",
    );

    assert_eq!(outcome, MatchOutcome::Matched(tentative));
}

#[test]
fn a_call_exactly_ten_minutes_before_a_meeting_can_match_it() {
    let outcome = matched_at(vec![review()], "2026-10-26T10:50:00+05:30");

    assert_eq!(outcome, MatchOutcome::Matched(review()));
}

#[test]
fn a_call_just_over_ten_minutes_before_a_meeting_does_not_match_it() {
    let outcome = matched_at(vec![review()], "2026-10-26T10:49:59+05:30");

    assert_eq!(outcome, MatchOutcome::Unmatched);
}

#[test]
fn a_call_at_the_moment_a_meeting_ends_does_not_match_it() {
    let outcome = matched_at(vec![review()], END_OF_REVIEW);

    assert_eq!(outcome, MatchOutcome::Unmatched);
}

#[test]
fn a_call_one_second_before_a_meeting_ends_still_matches_it() {
    let outcome = matched_at(vec![review()], "2026-10-26T11:29:59+05:30");

    assert_eq!(outcome, MatchOutcome::Matched(review()));
}

#[test]
fn the_match_states_are_named_for_logging() {
    assert_eq!(MatchOutcome::Unmatched.kind(), "unmatched");
    assert_eq!(MatchOutcome::Matched(review()).kind(), "matched");
    assert_eq!(MatchOutcome::Ambiguous(vec![review()]).kind(), "ambiguous");
}
