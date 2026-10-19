use std::time::Duration;

use scribe_core::recording::QuietWatch;

const LIMIT: Duration = Duration::from_secs(10);
const ONE_MILLISECOND_SHORT_OF_LIMIT: Duration = Duration::from_millis(9_999);

#[test]
fn a_call_that_has_just_started_is_not_quiet_yet() {
    let watch = QuietWatch::new(LIMIT);

    assert!(!watch.is_report_due());
}

#[test]
fn a_call_goes_quiet_once_nobody_has_spoken_for_exactly_the_limit() {
    let mut watch = QuietWatch::new(LIMIT);

    watch.hear_through(LIMIT);

    assert!(watch.is_report_due());
}

#[test]
fn a_call_is_not_quiet_one_millisecond_before_the_limit() {
    let mut watch = QuietWatch::new(LIMIT);

    watch.hear_through(ONE_MILLISECOND_SHORT_OF_LIMIT);

    assert!(!watch.is_report_due());
}

#[test]
fn speech_restarts_the_quiet_period_so_the_call_goes_quiet_a_full_limit_after_it() {
    let mut watch = QuietWatch::new(LIMIT);
    watch.hear_through(Duration::from_secs(4));
    watch.hear_speech();

    watch.hear_through(Duration::from_secs(13));
    let due_before_full_limit = watch.is_report_due();
    watch.hear_through(Duration::from_secs(14));

    assert!(!due_before_full_limit);
    assert!(watch.is_report_due());
}

#[test]
fn a_quiet_call_is_announced_only_once_while_the_quiet_continues() {
    let mut watch = QuietWatch::new(LIMIT);
    watch.hear_through(LIMIT);
    watch.mark_reported();

    watch.hear_through(Duration::from_secs(60));

    assert!(!watch.is_report_due());
}

#[test]
fn speech_after_an_announcement_lets_the_next_quiet_period_be_announced() {
    let mut watch = QuietWatch::new(LIMIT);
    watch.hear_through(LIMIT);
    watch.mark_reported();
    watch.hear_through(Duration::from_secs(12));
    watch.hear_speech();

    watch.hear_through(Duration::from_secs(21));
    let due_before_full_limit = watch.is_report_due();
    watch.hear_through(Duration::from_secs(22));

    assert!(!due_before_full_limit);
    assert!(watch.is_report_due());
}

#[test]
fn a_lagging_lane_reporting_an_earlier_time_does_not_move_the_clock_back() {
    let mut watch = QuietWatch::new(LIMIT);
    watch.hear_through(LIMIT);

    watch.hear_through(Duration::from_secs(2));

    assert!(watch.is_report_due());
}
