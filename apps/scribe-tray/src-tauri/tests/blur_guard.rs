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

#[test]
fn a_popover_opened_for_a_call_stays_open_when_focus_goes_elsewhere() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown_awaiting_focus(opened);

    assert!(!guard.blur_should_hide(opened + Duration::from_secs(60)));
}

#[test]
fn focus_arriving_straight_after_a_call_opened_the_popover_does_not_make_it_hide_on_blur() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown_awaiting_focus(opened);
    guard.focused(opened + Duration::from_millis(50));

    assert!(!guard.blur_should_hide(opened + Duration::from_secs(60)));
}

#[test]
fn clicking_away_from_a_call_opened_popover_hides_it_once_it_was_clicked_into() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown_awaiting_focus(opened);
    guard.focused(opened + Duration::from_secs(10));

    assert!(guard.blur_should_hide(opened + Duration::from_secs(20)));
}

#[test]
fn opening_from_the_tray_after_a_call_opened_popover_hides_on_blur_as_before() {
    let opened = Instant::now();
    let mut guard = BlurGuard::default();
    guard.shown_awaiting_focus(opened);
    guard.shown(opened + Duration::from_secs(30));

    assert!(guard.blur_should_hide(opened + Duration::from_secs(40)));
}
