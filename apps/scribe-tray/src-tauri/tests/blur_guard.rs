use std::time::{Duration, Instant};

use scribe_tray::blur_guard::BlurGuard;

const SETTLE: Duration = Duration::from_millis(300);

#[test]
fn losing_focus_straight_after_opening_does_not_hide_the_popover() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown(opened);

    assert!(!guard.blur_should_hide(opened + Duration::from_millis(100)));
}

#[test]
fn losing_focus_once_settled_hides_the_popover() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown(opened);

    assert!(guard.blur_should_hide(opened + SETTLE));
}

#[test]
fn the_tray_click_that_caused_a_blur_hide_does_not_reopen_the_popover() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown(opened);
    let blurred = opened + Duration::from_secs(5);
    guard.blur_should_hide(blurred);

    assert!(!guard.click_may_show(blurred + Duration::from_millis(50)));
}

#[test]
fn a_later_tray_click_reopens_the_popover() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown(opened);
    let blurred = opened + Duration::from_secs(5);
    guard.blur_should_hide(blurred);

    assert!(guard.click_may_show(blurred + SETTLE));
}

#[test]
fn the_first_tray_click_opens_the_popover() {
    assert!(BlurGuard::default().click_may_show(Instant::now()));
}

#[test]
fn a_folder_picker_taking_focus_does_not_hide_the_popover() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown(opened);
    guard.dialog_opened();

    assert!(!guard.blur_should_hide(opened + Duration::from_secs(5)));
}

#[test]
fn losing_focus_after_the_folder_picker_closes_hides_the_popover_again() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown(opened);
    guard.dialog_opened();
    guard.dialog_closed();

    assert!(guard.blur_should_hide(opened + Duration::from_secs(5)));
}
