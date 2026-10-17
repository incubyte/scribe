use std::time::Duration;

use scribe_core::recording::{
    ECHO_OVERLAP_TOLERANCE, ECHO_SIMILARITY_THRESHOLD, EchoFilter, Segment, Speaker,
};

fn filter() -> EchoFilter {
    EchoFilter::new(ECHO_OVERLAP_TOLERANCE, ECHO_SIMILARITY_THRESHOLD)
}

fn line(speaker: Speaker, (from_ms, to_ms): (u64, u64), text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_millis(from_ms),
        end: Duration::from_millis(to_ms),
        text: text.to_owned(),
    }
}

#[test]
fn my_line_repeating_their_words_at_the_same_moment_is_dropped() {
    let mine = line(
        Speaker::Me,
        (1_000, 2_000),
        "Let's review the budget today.",
    );
    let theirs = [line(
        Speaker::Them,
        (1_100, 2_100),
        "let's review the budget today",
    )];

    assert!(filter().is_echo(&mine, &theirs));
}

#[test]
fn my_line_containing_a_run_of_their_words_is_dropped() {
    let mine = line(
        Speaker::Me,
        (1_000, 3_000),
        "so um the quarterly numbers look strong and then some",
    );
    let theirs = [line(
        Speaker::Them,
        (1_000, 2_500),
        "the quarterly numbers look strong",
    )];

    assert!(filter().is_echo(&mine, &theirs));
}

#[test]
fn my_different_words_while_they_speak_are_kept() {
    let mine = line(Speaker::Me, (1_000, 2_000), "I disagree with that plan");
    let theirs = [line(
        Speaker::Them,
        (1_000, 2_000),
        "We should ship on Friday",
    )];

    assert!(!filter().is_echo(&mine, &theirs));
}

#[test]
fn identical_words_more_than_a_second_apart_are_kept() {
    let mine = line(Speaker::Me, (5_000, 6_000), "Let's review the budget today");
    let theirs = [line(
        Speaker::Them,
        (1_000, 2_000),
        "Let's review the budget today",
    )];

    assert!(!filter().is_echo(&mine, &theirs));
}

#[test]
fn a_two_word_overlap_is_not_treated_as_echo() {
    let mine = line(Speaker::Me, (1_000, 2_000), "sounds good to me then");
    let theirs = [line(Speaker::Them, (1_000, 2_000), "that sounds good")];

    assert!(!filter().is_echo(&mine, &theirs));
}

#[test]
fn a_line_starting_just_inside_the_overlap_tolerance_is_still_compared() {
    let mine = line(Speaker::Me, (3_000, 4_000), "Let's review the budget today");
    let theirs = [line(
        Speaker::Them,
        (1_000, 2_000),
        "Let's review the budget today",
    )];

    assert!(filter().is_echo(&mine, &theirs));
}

#[test]
fn my_line_is_kept_when_nobody_else_has_spoken() {
    let mine = line(Speaker::Me, (1_000, 2_000), "Let's review the budget today");

    assert!(!filter().is_echo(&mine, &[]));
}
