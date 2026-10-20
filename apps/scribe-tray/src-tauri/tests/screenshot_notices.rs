use std::time::Duration;

use scribe_core::capture::{CaptureOutcome, ScreenshotName};
use scribe_core::testing::frame_filled;
use scribe_tray::screenshots::notices::{
    capture_failed_notice, notice_for, shortcut_unavailable_notice,
};

const RED: [u8; 4] = [255, 0, 0, 255];

#[test]
fn a_saved_screenshot_is_announced_with_its_offset_into_the_call() {
    let outcome = CaptureOutcome::SavedIntoSession {
        screenshot: frame_filled(1, 1, RED),
        name: ScreenshotName::at(Duration::from_secs(12 * 60 + 34)),
    };

    let notice = notice_for(&outcome);

    assert_eq!(
        notice.body,
        "Screenshot saved at 00:12:34 and copied to the clipboard"
    );
}

#[test]
fn a_screenshot_that_could_not_be_saved_is_announced_as_copied_only() {
    let outcome = CaptureOutcome::CopiedButNotSaved {
        screenshot: frame_filled(1, 1, RED),
    };

    let notice = notice_for(&outcome);

    assert_eq!(
        notice.body,
        "Screenshot copied, but it couldn't be saved to this meeting"
    );
}

#[test]
fn a_screenshot_taken_between_calls_is_announced_as_copied() {
    let outcome = CaptureOutcome::CopiedOnly {
        screenshot: frame_filled(1, 1, RED),
    };

    let notice = notice_for(&outcome);

    assert_eq!(notice.body, "Screenshot copied to the clipboard");
}

#[test]
fn a_failed_capture_asks_the_user_to_try_again() {
    let notice = capture_failed_notice();

    assert_eq!(notice.body, "Couldn't take the screenshot. Try again.");
}

#[test]
fn an_unavailable_shortcut_is_named_in_its_notice() {
    let notice = shortcut_unavailable_notice("⌃⌥⌘1");

    assert!(notice.body.contains("⌃⌥⌘1"));
}
