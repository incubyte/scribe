use std::time::{Duration, Instant};

use scribe_core::update::CheckSchedule;

const INTERVAL: Duration = Duration::from_secs(3600);
const ONE_MILLISECOND_SHORT_OF_INTERVAL: Duration = Duration::from_millis(3_599_999);

#[test]
fn the_first_check_is_due_at_launch() {
    let schedule = CheckSchedule::new(INTERVAL);

    assert!(schedule.is_check_due(Instant::now()));
}

#[test]
fn no_second_check_is_due_before_the_interval_has_passed() {
    let checked_at = Instant::now();
    let mut schedule = CheckSchedule::new(INTERVAL);
    schedule.mark_checked(checked_at);

    assert!(!schedule.is_check_due(checked_at + ONE_MILLISECOND_SHORT_OF_INTERVAL));
}

#[test]
fn a_check_is_due_again_once_the_interval_has_passed() {
    let checked_at = Instant::now();
    let mut schedule = CheckSchedule::new(INTERVAL);
    schedule.mark_checked(checked_at);

    assert!(schedule.is_check_due(checked_at + INTERVAL));
}

#[test]
fn a_check_just_after_launch_is_not_due_again_straight_away() {
    let checked_at = Instant::now();
    let mut schedule = CheckSchedule::new(INTERVAL);
    schedule.mark_checked(checked_at);

    assert!(!schedule.is_check_due(checked_at));
}

#[test]
fn a_check_asked_for_soon_is_due_at_that_moment_even_within_the_interval() {
    let checked_at = Instant::now();
    let soon = checked_at + Duration::from_secs(600);
    let mut schedule = CheckSchedule::new(INTERVAL);
    schedule.mark_checked(checked_at);

    schedule.check_soon(soon);

    assert!(schedule.is_check_due(soon));
}

#[test]
fn a_check_asked_for_soon_is_not_due_before_that_moment() {
    let checked_at = Instant::now();
    let soon = checked_at + Duration::from_secs(600);
    let mut schedule = CheckSchedule::new(INTERVAL);
    schedule.mark_checked(checked_at);

    schedule.check_soon(soon);

    assert!(!schedule.is_check_due(checked_at + Duration::from_millis(599_999)));
}

#[test]
fn asking_for_a_check_soon_never_postpones_one_already_due() {
    let launched_at = Instant::now();
    let mut schedule = CheckSchedule::new(INTERVAL);

    schedule.check_soon(launched_at + Duration::from_secs(30));

    assert!(schedule.is_check_due(launched_at));
}
