use chrono::{DateTime, FixedOffset};
use scribe_tray::calendar::day_bounds::today_bounds;

fn moment(text: &str) -> DateTime<FixedOffset> {
    DateTime::parse_from_rfc3339(text).unwrap()
}

#[test]
fn today_runs_from_local_midnight_to_the_next_local_midnight() {
    let bounds = today_bounds(&moment("2026-10-26T15:42:10+05:30")).unwrap();

    assert_eq!(bounds.start, moment("2026-10-26T00:00:00+05:30"));
    assert_eq!(bounds.end, moment("2026-10-27T00:00:00+05:30"));
}

#[test]
fn a_moment_just_after_midnight_still_belongs_to_the_new_day() {
    let bounds = today_bounds(&moment("2026-10-27T00:00:01-07:00")).unwrap();

    assert_eq!(bounds.start, moment("2026-10-27T00:00:00-07:00"));
}
