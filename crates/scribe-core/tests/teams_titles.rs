use chrono::{DateTime, FixedOffset};
use scribe_core::calendar::{EventId, Meeting, Person, Response, meeting_named_in_titles};

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

fn meeting(subject: &str) -> Meeting {
    Meeting {
        event_id: EventId::new(subject),
        subject: subject.to_owned(),
        starts_at: moment("2026-10-26T11:00:00+05:30"),
        ends_at: moment("2026-10-26T11:30:00+05:30"),
        organizer: Person::default(),
        attendees: Vec::new(),
        is_teams: true,
        is_recurring: false,
        response: Response::Accepted,
    }
}

fn titles(texts: &[&str]) -> Vec<String> {
    texts.iter().map(|text| (*text).to_owned()).collect()
}

#[test]
fn a_teams_window_naming_one_overlapping_meeting_picks_that_meeting() {
    let standup = meeting("Anthara Standup");
    let review = meeting("DXC Review");

    let named = meeting_named_in_titles(
        &[&review, &standup],
        &titles(&["Anthara Standup | Microsoft Teams"]),
    );

    assert_eq!(named, Some(&standup));
}

#[test]
fn the_subject_is_found_whatever_its_case_or_surrounding_whitespace() {
    let standup = meeting("  Anthara STANDUP ");

    let named = meeting_named_in_titles(
        &[&standup],
        &titles(&["anthara standup  | Microsoft Teams"]),
    );

    assert_eq!(named, Some(&standup));
}

#[test]
fn one_title_naming_two_meetings_decides_nothing() {
    let standup = meeting("Standup");
    let review = meeting("Standup Review");

    let named = meeting_named_in_titles(
        &[&standup, &review],
        &titles(&["Standup Review | Microsoft Teams"]),
    );

    assert_eq!(named, None);
}

#[test]
fn two_titles_naming_different_meetings_decide_nothing() {
    let standup = meeting("Anthara Standup");
    let review = meeting("DXC Review");

    let named = meeting_named_in_titles(
        &[&standup, &review],
        &titles(&[
            "Anthara Standup | Microsoft Teams",
            "DXC Review | Microsoft Teams",
        ]),
    );

    assert_eq!(named, None);
}

#[test]
fn two_titles_naming_the_same_meeting_still_pick_it() {
    let standup = meeting("Anthara Standup");
    let review = meeting("DXC Review");

    let named = meeting_named_in_titles(
        &[&standup, &review],
        &titles(&[
            "Anthara Standup | Microsoft Teams",
            "Anthara Standup (chat)",
        ]),
    );

    assert_eq!(named, Some(&standup));
}

#[test]
fn no_titles_name_no_meeting() {
    let standup = meeting("Anthara Standup");

    assert_eq!(meeting_named_in_titles(&[&standup], &[]), None);
}

#[test]
fn a_meeting_without_a_subject_is_never_named() {
    let untitled = meeting("   ");

    let named = meeting_named_in_titles(&[&untitled], &titles(&["Anything | Microsoft Teams"]));

    assert_eq!(named, None);
}
