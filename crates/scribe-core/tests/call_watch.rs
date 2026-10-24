use std::time::{Duration, Instant};

use scribe_core::call::CallWatch;

const GRACE: Duration = Duration::from_secs(30);
const ONE_MILLISECOND_SHORT_OF_GRACE: Duration = Duration::from_millis(29_999);

#[test]
fn nothing_is_due_before_any_call_app_takes_the_microphone() {
    let mut watch = CallWatch::new(GRACE);

    watch.observe(false, Instant::now());

    assert!(!watch.is_start_due());
}

#[test]
fn a_call_app_taking_the_microphone_makes_a_call_start_due() {
    let mut watch = CallWatch::new(GRACE);

    watch.observe(true, Instant::now());

    assert!(watch.is_start_due());
}

#[test]
fn a_reported_call_is_not_due_again_while_the_microphone_stays_in_use() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.mark_reported();

    watch.observe(true, started + Duration::from_secs(2));

    assert!(!watch.is_start_due());
}

#[test]
fn a_release_one_millisecond_short_of_the_grace_keeps_the_same_call() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.mark_reported();
    watch.observe(false, started);

    watch.observe(false, started + ONE_MILLISECOND_SHORT_OF_GRACE);
    watch.observe(true, started + ONE_MILLISECOND_SHORT_OF_GRACE);

    assert!(!watch.is_start_due());
}

#[test]
fn retaking_the_microphone_during_a_brief_release_restarts_the_grace() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.mark_reported();
    watch.observe(false, started);
    watch.observe(true, started + Duration::from_secs(10));
    watch.observe(false, started + Duration::from_secs(20));

    watch.observe(false, started + GRACE);
    watch.observe(true, started + GRACE);

    assert!(!watch.is_start_due());
}

#[test]
fn a_release_lasting_exactly_the_grace_ends_the_call() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.mark_reported();
    watch.observe(false, started);

    watch.observe(false, started + GRACE);
    watch.observe(true, started + GRACE);

    assert!(watch.is_start_due());
}

#[test]
fn taking_the_microphone_again_after_a_call_ended_makes_a_new_call_start_due() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.mark_reported();
    watch.observe(false, started);
    watch.observe(false, started + GRACE);

    watch.observe(true, started + GRACE + Duration::from_secs(60));

    assert!(watch.is_start_due());
}

#[test]
fn an_app_already_holding_the_microphone_at_the_first_observation_makes_a_call_start_due() {
    let mut watch = CallWatch::new(GRACE);

    watch.observe(true, Instant::now());
    watch.observe(true, Instant::now());

    assert!(watch.is_start_due());
}

#[test]
fn a_call_still_counts_during_the_grace_after_the_call_app_lets_go() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.observe(false, started);

    watch.observe(false, started + ONE_MILLISECOND_SHORT_OF_GRACE);

    assert!(watch.is_in_call());
}

#[test]
fn a_call_no_longer_counts_once_the_grace_has_passed() {
    let started = Instant::now();
    let mut watch = CallWatch::new(GRACE);
    watch.observe(true, started);
    watch.observe(false, started);

    watch.observe(false, started + GRACE);

    assert!(!watch.is_in_call());
}

#[test]
fn no_call_counts_before_a_call_app_takes_the_microphone() {
    let watch = CallWatch::new(GRACE);

    assert!(!watch.is_in_call());
}
