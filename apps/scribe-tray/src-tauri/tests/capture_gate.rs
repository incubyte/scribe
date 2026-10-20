use std::sync::Arc;

use scribe_core::capture::{PendingCapture, PixelRect, ScreenshotTaker, Selection};
use scribe_core::testing::{FakeScreenCapturer, InMemoryScreenshotStore, frame_filled, shot_of};
use scribe_tray::screenshots::capture_gate::{CaptureGate, RegionRefusal};

const GREY: [u8; 4] = [128, 128, 128, 255];

fn pending_capture() -> PendingCapture {
    let capturer = Arc::new(FakeScreenCapturer::showing(shot_of(frame_filled(
        16, 16, GREY,
    ))));
    let taker = ScreenshotTaker::new(capturer, Arc::new(InMemoryScreenshotStore::default()));
    taker.begin(None).unwrap()
}

fn area(width: u32, height: u32) -> Selection {
    Selection::Area(PixelRect {
        x: 0,
        y: 0,
        width,
        height,
    })
}

fn gate_with_a_selection_open() -> CaptureGate {
    let gate = CaptureGate::default();
    gate.claim().unwrap().park(pending_capture());
    gate
}

#[test]
fn a_press_while_a_capture_is_being_taken_is_ignored() {
    let gate = CaptureGate::default();
    let _first = gate.claim().unwrap();

    let second = gate.claim();

    assert!(second.is_none());
}

#[test]
fn either_shortcut_is_ignored_while_the_region_selector_is_open() {
    let gate = gate_with_a_selection_open();

    let next_press = gate.claim();

    assert!(next_press.is_none());
}

#[test]
fn a_selection_too_small_to_capture_keeps_the_selector_waiting() {
    let gate = gate_with_a_selection_open();

    let refused = gate.take_accepting(area(3, 3));
    let accepted = gate.take_accepting(area(4, 4));

    assert_eq!(refused.unwrap_err(), RegionRefusal::SelectionRefused);
    assert!(accepted.is_ok());
}

#[test]
fn a_completed_selection_frees_the_shortcuts_for_the_next_capture() {
    let gate = gate_with_a_selection_open();

    gate.take_accepting(area(8, 8)).unwrap();

    assert!(gate.claim().is_some());
}

#[test]
fn cancelling_drops_the_frozen_frame_and_frees_the_shortcuts() {
    let gate = gate_with_a_selection_open();

    gate.discard_pending();

    assert_eq!(gate.read_pending(|_| ()), None);
    assert!(gate.claim().is_some());
}

#[test]
fn completing_with_nothing_pending_is_refused() {
    let gate = CaptureGate::default();

    let completed = gate.take_accepting(area(8, 8));

    assert_eq!(completed.unwrap_err(), RegionRefusal::NothingPending);
}

#[test]
fn the_selector_reads_the_frame_frozen_at_the_press() {
    let gate = gate_with_a_selection_open();

    let frame_size = gate.read_pending(|pending| {
        let frame = &pending.shot().frame;
        (frame.width(), frame.height())
    });

    assert_eq!(frame_size, Some((16, 16)));
}
