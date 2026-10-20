#![cfg(target_os = "macos")]

use capture_macos::ScreenCaptureKitCapturer;
use scribe_core::capture::PlacementUnits;
use scribe_core::ports::ScreenCapturer;

#[test]
#[ignore = "needs a real display and the Screen Recording permission"]
fn a_capture_is_the_whole_monitor_under_the_pointer_at_its_native_pixel_size() {
    let capturer = ScreenCaptureKitCapturer::new();

    let shot = capturer.capture_under_pointer().unwrap();

    let scale = shot.frame.width() / shot.placement.width;
    assert_eq!(shot.placement.units, PlacementUnits::Points);
    assert!(scale >= 1);
    assert_eq!(shot.frame.width(), shot.placement.width * scale);
    assert_eq!(shot.frame.height(), shot.placement.height * scale);
}
