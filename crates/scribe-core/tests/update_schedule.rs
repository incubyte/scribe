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
