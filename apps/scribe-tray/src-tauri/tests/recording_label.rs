use std::time::Duration;

use scribe_tray::recording_label::stop_recording_label;

#[test]
fn the_recording_menu_offers_to_stop_and_shows_the_elapsed_time() {
    assert_eq!(
        stop_recording_label(Duration::from_secs(65)),
        "Stop recording (00:01:05)"
    );
}

#[test]
fn the_elapsed_time_counts_hours_on_long_calls() {
    assert_eq!(
        stop_recording_label(Duration::from_secs(2 * 3_600 + 3)),
        "Stop recording (02:00:03)"
    );
}
