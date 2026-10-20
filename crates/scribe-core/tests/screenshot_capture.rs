use std::sync::Arc;
use std::time::Duration;

use scribe_core::capture::{
    CaptureFailure, CaptureOutcome, Frame, InvalidFrame, InvalidSelection, PixelRect,
    ScreenshotName, ScreenshotTaker, Selection,
};
use scribe_core::ports::ScreenCaptureError;
use scribe_core::recording::SessionId;
use scribe_core::testing::{
    FakeScreenCapturer, InMemoryScreenshotStore, frame_filled, live_session_at, opaque_grey_pixels,
    shot_of,
};

const RED: [u8; 4] = [255, 0, 0, 255];
const FORTY_TWO_SECONDS: Duration = Duration::from_secs(42);

fn session() -> SessionId {
    SessionId::parse("2026-10-20-143005").unwrap()
}

fn monitor() -> Frame {
    frame_filled(4, 2, RED)
}

fn taker_showing(frame: Frame, store: &Arc<InMemoryScreenshotStore>) -> ScreenshotTaker {
    let capturer = Arc::new(FakeScreenCapturer::showing(shot_of(frame)));
    ScreenshotTaker::new(capturer, store.clone())
}

fn three_by_two() -> Frame {
    Frame::new(3, 2, opaque_grey_pixels(1..=6)).unwrap()
}

#[test]
fn a_capture_while_recording_is_saved_into_that_session_at_the_offset_of_the_press() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let taker = taker_showing(monitor(), &store);
    let live = live_session_at(session(), FORTY_TWO_SECONDS);

    let pending = taker.begin(Some(live)).unwrap();
    let outcome = taker.finish(pending, Selection::WholeMonitor).unwrap();

    let name = ScreenshotName::parse("00-00-42.png").unwrap();
    assert_eq!(
        outcome,
        CaptureOutcome::SavedIntoSession {
            screenshot: monitor(),
            name: name.clone(),
        }
    );
    assert_eq!(store.screenshot(&session(), &name), Some(monitor()));
    assert_eq!(store.saved_count(), 1);
}

#[test]
fn a_capture_with_screen_access_captures_without_asking_for_access() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let capturer = Arc::new(FakeScreenCapturer::showing(shot_of(monitor())));
    let taker = ScreenshotTaker::new(capturer.clone(), store);

    taker.begin(None).unwrap();

    assert_eq!(capturer.access_requests(), 0);
    assert_eq!(capturer.captures_attempted(), 1);
}

#[test]
fn a_frame_exposes_the_pixels_it_was_built_from() {
    let frame = Frame::new(1, 1, vec![1, 2, 3, 4]).unwrap();

    assert_eq!(frame.rgba(), [1, 2, 3, 4]);
}

#[test]
fn a_screenshot_that_cannot_be_saved_is_still_handed_over_for_the_clipboard() {
    let store = Arc::new(InMemoryScreenshotStore::refusing_saves());
    let taker = taker_showing(monitor(), &store);
    let live = live_session_at(session(), FORTY_TWO_SECONDS);

    let pending = taker.begin(Some(live)).unwrap();
    let outcome = taker.finish(pending, Selection::WholeMonitor).unwrap();

    assert_eq!(
        outcome,
        CaptureOutcome::CopiedButNotSaved {
            screenshot: monitor()
        }
    );
}

#[test]
fn a_failed_capture_yields_no_screenshot_and_saves_nothing() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let capturer = Arc::new(FakeScreenCapturer::failing(ScreenCaptureError::Failed(
        "the display went away".to_owned(),
    )));
    let taker = ScreenshotTaker::new(capturer, store.clone());
    let live = live_session_at(session(), FORTY_TWO_SECONDS);

    let started = taker.begin(Some(live));

    assert_eq!(started.unwrap_err(), CaptureFailure::CaptureFailed);
    assert_eq!(store.saved_count(), 0);
}

#[test]
fn a_capture_without_screen_access_asks_for_it_and_captures_and_saves_nothing() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let capturer = Arc::new(FakeScreenCapturer::showing_without_access(shot_of(
        monitor(),
    )));
    let taker = ScreenshotTaker::new(capturer.clone(), store.clone());
    let live = live_session_at(session(), FORTY_TWO_SECONDS);

    let started = taker.begin(Some(live));

    assert_eq!(started.unwrap_err(), CaptureFailure::AccessNotGranted);
    assert_eq!(capturer.access_requests(), 1);
    assert_eq!(capturer.captures_attempted(), 0);
    assert_eq!(store.saved_count(), 0);
}

#[test]
fn access_withdrawn_during_a_capture_is_reported_as_missing_access() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let capturer = Arc::new(FakeScreenCapturer::failing(
        ScreenCaptureError::AccessNotGranted,
    ));
    let taker = ScreenshotTaker::new(capturer, store);

    let started = taker.begin(None);

    assert_eq!(started.unwrap_err(), CaptureFailure::AccessNotGranted);
}

#[test]
fn a_refused_capture_leaves_the_recording_session_intact_and_the_next_press_works_once_access_is_granted()
 {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let capturer = Arc::new(FakeScreenCapturer::showing_without_access(shot_of(
        monitor(),
    )));
    let taker = ScreenshotTaker::new(capturer.clone(), store.clone());
    let live = live_session_at(session(), FORTY_TWO_SECONDS);

    let refused = taker.begin(Some(live.clone()));
    capturer.grant_access();
    let pending = taker.begin(Some(live.clone())).unwrap();
    let outcome = taker.finish(pending, Selection::WholeMonitor).unwrap();

    assert_eq!(refused.unwrap_err(), CaptureFailure::AccessNotGranted);
    assert_eq!(live.id(), &session());
    assert!(matches!(outcome, CaptureOutcome::SavedIntoSession { .. }));
}

#[test]
fn a_capture_with_nothing_recording_is_only_copied() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let taker = taker_showing(monitor(), &store);

    let pending = taker.begin(None).unwrap();
    let outcome = taker.finish(pending, Selection::WholeMonitor).unwrap();

    assert_eq!(
        outcome,
        CaptureOutcome::CopiedOnly {
            screenshot: monitor()
        }
    );
    assert_eq!(store.saved_count(), 0);
}

#[test]
fn a_region_completed_after_the_press_goes_into_the_session_of_the_press_at_its_offset() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let taker = taker_showing(frame_filled(8, 8, RED), &store);
    let live = live_session_at(session(), FORTY_TWO_SECONDS);
    let area = PixelRect {
        x: 2,
        y: 2,
        width: 4,
        height: 4,
    };

    let pending = taker.begin(Some(live)).unwrap();
    let outcome = taker.finish(pending, Selection::Area(area)).unwrap();

    let name = ScreenshotName::parse("00-00-42.png").unwrap();
    assert_eq!(
        store.screenshot(&session(), &name),
        Some(frame_filled(4, 4, RED))
    );
    assert!(matches!(outcome, CaptureOutcome::SavedIntoSession { .. }));
}

#[test]
fn an_area_reaching_past_the_monitor_edge_is_refused() {
    let store = Arc::new(InMemoryScreenshotStore::default());
    let taker = taker_showing(frame_filled(5, 5, RED), &store);
    let area = PixelRect {
        x: 2,
        y: 0,
        width: 4,
        height: 4,
    };

    let pending = taker.begin(None).unwrap();
    let outcome = taker.finish(pending, Selection::Area(area));

    assert_eq!(outcome.unwrap_err(), InvalidSelection);
}

#[test]
fn an_empty_area_is_refused() {
    let area = PixelRect {
        x: 0,
        y: 0,
        width: 0,
        height: 2,
    };

    let cropped = three_by_two().crop(area);

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn pixels_that_do_not_fill_the_frame_are_refused() {
    let frame = Frame::new(2, 2, vec![0; 12]);

    assert_eq!(frame.unwrap_err(), InvalidFrame);
}

#[test]
fn a_frame_without_area_is_refused() {
    let frame = Frame::new(0, 2, Vec::new());

    assert_eq!(frame.unwrap_err(), InvalidFrame);
}
