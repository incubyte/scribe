use scribe_tray::other_side_alert::OTHER_SIDE_UNAVAILABLE_ON_WINDOWS;

#[test]
fn on_windows_the_user_hears_that_the_other_side_could_not_be_captured_from_the_output_device() {
    let alert = OTHER_SIDE_UNAVAILABLE_ON_WINDOWS;

    assert_eq!(
        alert.title,
        "Scribe can't capture the other side of the call"
    );
    assert!(alert.body.contains("output device"));
}

#[test]
fn on_windows_the_user_is_not_sent_to_privacy_settings_for_a_device_failure() {
    let body = OTHER_SIDE_UNAVAILABLE_ON_WINDOWS.body.to_lowercase();

    assert!(!body.contains("privacy"));
    assert!(!body.contains("permission"));
}
