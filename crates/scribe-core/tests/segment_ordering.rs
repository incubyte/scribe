use std::time::Duration;

use scribe_core::recording::{
    ECHO_OVERLAP_TOLERANCE, MAX_HOLD, Segment, SegmentSequencer, Speaker,
};

const BOTH_SIDES: [Speaker; 2] = [Speaker::Me, Speaker::Them];

fn line(speaker: Speaker, (from_ms, to_ms): (u64, u64), text: &str) -> Segment {
    Segment {
        speaker,
        start: Duration::from_millis(from_ms),
        end: Duration::from_millis(to_ms),
        text: text.to_owned(),
    }
}

fn millis(ms: u64) -> Duration {
    Duration::from_millis(ms)
}

#[test]
fn nothing_is_released_before_any_line_is_finished() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);

    assert!(sequencer.release_ready().is_empty());
    assert!(sequencer.drain().is_empty());
}

#[test]
fn a_single_side_releases_each_line_as_soon_as_it_is_finished() {
    let mut sequencer = SegmentSequencer::new(&[Speaker::Me]);
    sequencer.accept(line(Speaker::Me, (0, 900), "Hello"));

    let released = sequencer.release_ready();

    assert_eq!(released, vec![line(Speaker::Me, (0, 900), "Hello")]);
}

#[test]
fn my_line_waits_until_the_other_side_has_caught_up_past_its_end() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Me, (1_000, 1_400), "Are you there"));
    sequencer.advance(Speaker::Me, millis(1_500));
    sequencer.advance(Speaker::Them, millis(1_399) + ECHO_OVERLAP_TOLERANCE);
    let while_behind = sequencer.release_ready();

    sequencer.advance(Speaker::Them, millis(1_400) + ECHO_OVERLAP_TOLERANCE);
    let once_caught_up = sequencer.release_ready();

    assert!(while_behind.is_empty());
    assert_eq!(
        once_caught_up,
        vec![line(Speaker::Me, (1_000, 1_400), "Are you there")]
    );
}

#[test]
fn their_line_waits_until_my_side_has_caught_up_to_its_start() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Them, (1_000, 1_400), "Are you there"));
    sequencer.advance(Speaker::Them, millis(1_500));
    sequencer.advance(Speaker::Me, millis(900));
    let while_behind = sequencer.release_ready();

    sequencer.advance(Speaker::Me, millis(1_000));
    let once_caught_up = sequencer.release_ready();

    assert!(while_behind.is_empty());
    assert_eq!(
        once_caught_up,
        vec![line(Speaker::Them, (1_000, 1_400), "Are you there")]
    );
}

#[test]
fn an_echoed_line_never_reaches_the_transcript() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(
        Speaker::Them,
        (1_000, 2_000),
        "Let's review the budget",
    ));
    sequencer.accept(line(Speaker::Me, (1_100, 2_100), "let's review the budget"));
    sequencer.advance(Speaker::Me, millis(5_000));
    sequencer.advance(Speaker::Them, millis(5_000));

    let released = sequencer.release_ready();
    let drained = sequencer.drain();

    assert_eq!(
        released,
        vec![line(
            Speaker::Them,
            (1_000, 2_000),
            "Let's review the budget"
        )]
    );
    assert!(drained.is_empty());
}

#[test]
fn an_echo_is_caught_even_after_their_line_was_already_released() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(
        Speaker::Them,
        (1_000, 2_000),
        "Let's review the budget",
    ));
    sequencer.advance(Speaker::Me, millis(1_000));
    sequencer.advance(Speaker::Them, millis(2_000));
    let first = sequencer.release_ready();

    sequencer.accept(line(Speaker::Me, (1_500, 2_500), "let's review the budget"));
    sequencer.advance(Speaker::Me, millis(2_500));
    sequencer.advance(Speaker::Them, millis(4_000));
    let second = sequencer.release_ready();

    assert_eq!(first.len(), 1);
    assert!(second.is_empty());
}

#[test]
fn a_line_still_held_when_recording_stops_is_checked_for_echo() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(
        Speaker::Them,
        (1_000, 2_000),
        "Let's review the budget",
    ));
    sequencer.accept(line(Speaker::Me, (1_100, 2_100), "let's review the budget"));

    let drained = sequencer.drain();

    assert_eq!(
        drained,
        vec![line(
            Speaker::Them,
            (1_000, 2_000),
            "Let's review the budget"
        )]
    );
}

#[test]
fn my_line_is_held_no_longer_than_the_maximum_hold_while_waiting_for_their_side() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    let ends_at = millis(2_000);
    sequencer.accept(line(Speaker::Me, (1_000, 2_000), "Can you hear me"));
    sequencer.advance(Speaker::Them, millis(1_000));
    sequencer.advance(Speaker::Me, millis(3_499));
    let just_before_the_limit = sequencer.release_ready();

    sequencer.advance(Speaker::Me, ends_at + MAX_HOLD);
    let at_the_limit = sequencer.release_ready();

    assert!(just_before_the_limit.is_empty());
    assert_eq!(
        at_the_limit,
        vec![line(Speaker::Me, (1_000, 2_000), "Can you hear me")]
    );
}

#[test]
fn lines_from_both_sides_are_released_in_the_order_they_started() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Me, (0, 800), "Hi"));
    sequencer.accept(line(Speaker::Me, (2_000, 2_600), "Good, thanks"));
    sequencer.accept(line(Speaker::Them, (1_000, 1_700), "How are you"));
    sequencer.advance(Speaker::Me, millis(5_000));
    sequencer.advance(Speaker::Them, millis(5_000));

    let released = sequencer.release_ready();

    assert_eq!(
        released,
        vec![
            line(Speaker::Me, (0, 800), "Hi"),
            line(Speaker::Them, (1_000, 1_700), "How are you"),
            line(Speaker::Me, (2_000, 2_600), "Good, thanks"),
        ]
    );
}

#[test]
fn when_both_sides_start_speaking_together_my_line_comes_first() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Them, (1_000, 1_500), "Shall we"));
    sequencer.accept(line(Speaker::Me, (1_000, 1_500), "Let's start"));
    sequencer.advance(Speaker::Me, millis(3_000));
    sequencer.advance(Speaker::Them, millis(3_000));

    let released = sequencer.release_ready();

    assert_eq!(
        released,
        vec![
            line(Speaker::Me, (1_000, 1_500), "Let's start"),
            line(Speaker::Them, (1_000, 1_500), "Shall we"),
        ]
    );
}

#[test]
fn a_stalled_side_holds_the_other_side_for_no_longer_than_the_maximum_hold() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    let ends_at = millis(1_000);
    sequencer.accept(line(Speaker::Them, (500, 1_000), "Can you hear me"));
    sequencer.advance(Speaker::Them, millis(2_499));
    let just_before_the_limit = sequencer.release_ready();

    sequencer.advance(Speaker::Them, ends_at + MAX_HOLD);
    let at_the_limit = sequencer.release_ready();

    assert!(just_before_the_limit.is_empty());
    assert_eq!(
        at_the_limit,
        vec![line(Speaker::Them, (500, 1_000), "Can you hear me")]
    );
}

#[test]
fn stopping_drains_every_held_line_in_start_order() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Them, (2_000, 2_500), "Bye"));
    sequencer.accept(line(Speaker::Me, (1_000, 1_500), "Thanks all"));

    let drained = sequencer.drain();

    assert_eq!(
        drained,
        vec![
            line(Speaker::Me, (1_000, 1_500), "Thanks all"),
            line(Speaker::Them, (2_000, 2_500), "Bye"),
        ]
    );
    assert!(sequencer.release_ready().is_empty());
}

#[test]
fn my_line_waits_while_their_earlier_line_is_still_being_spoken() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Me, (2_000, 2_500), "Yeah"));
    sequencer.advance(Speaker::Me, millis(4_500));
    sequencer.hear(Speaker::Them, millis(4_500));
    let while_they_speak = sequencer.release_ready();

    sequencer.accept(line(Speaker::Them, (0, 5_000), "So the plan is simple"));
    sequencer.advance(Speaker::Them, millis(5_600));
    let once_their_line_is_final = sequencer.release_ready();

    assert!(while_they_speak.is_empty());
    assert_eq!(
        once_their_line_is_final,
        vec![
            line(Speaker::Them, (0, 5_000), "So the plan is simple"),
            line(Speaker::Me, (2_000, 2_500), "Yeah"),
        ]
    );
}

fn their_budget_line() -> Segment {
    line(Speaker::Them, (1_000, 2_000), "Let's review the budget")
}

#[test]
fn an_echo_is_caught_even_when_their_line_is_finalised_after_mine() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Me, (1_100, 2_100), "let's review the budget"));
    sequencer.advance(Speaker::Me, millis(4_000));
    sequencer.advance(Speaker::Them, millis(1_000));
    sequencer.hear(Speaker::Them, millis(4_000));
    let before_their_line_is_final = sequencer.release_ready();

    sequencer.accept(their_budget_line());
    sequencer.advance(Speaker::Them, millis(4_100));
    let after_their_line_is_final = sequencer.release_ready();

    assert!(before_their_line_is_final.is_empty());
    assert_eq!(after_their_line_is_final, vec![their_budget_line()]);
    assert!(sequencer.drain().is_empty());
}

const FINAL_RESULTS_WAIT: Duration = Duration::from_secs(6);

#[test]
fn their_line_waits_no_longer_than_the_final_results_wait_while_my_side_is_silent() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.limit_wait_for_finals(Speaker::Me, FINAL_RESULTS_WAIT);
    sequencer.accept(line(Speaker::Them, (5_000, 6_000), "Are you still there"));
    sequencer.advance(Speaker::Them, millis(11_000));
    sequencer.advance(Speaker::Me, millis(4_260));
    sequencer.hear(Speaker::Me, millis(10_999));
    let just_before_the_limit = sequencer.release_ready();

    sequencer.hear(Speaker::Me, millis(11_000));
    let at_the_limit = sequencer.release_ready();

    assert!(just_before_the_limit.is_empty());
    assert_eq!(
        at_the_limit,
        vec![line(Speaker::Them, (5_000, 6_000), "Are you still there")]
    );
}

#[test]
fn without_a_final_results_wait_their_line_keeps_waiting_for_my_side() {
    let mut sequencer = SegmentSequencer::new(&BOTH_SIDES);
    sequencer.accept(line(Speaker::Them, (5_000, 6_000), "Are you still there"));
    sequencer.advance(Speaker::Them, millis(30_000));
    sequencer.advance(Speaker::Me, millis(4_260));
    sequencer.hear(Speaker::Me, millis(30_000));

    assert!(sequencer.release_ready().is_empty());
}
