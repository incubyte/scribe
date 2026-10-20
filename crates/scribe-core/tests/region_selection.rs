use std::sync::Arc;

use scribe_core::capture::{Frame, InvalidSelection, PixelRect, ScreenshotTaker, Selection};
use scribe_core::testing::{FakeScreenCapturer, InMemoryScreenshotStore, shot_of};

fn five_by_five() -> Frame {
    Frame::new(
        5,
        5,
        vec![
            0, 0, 0, 255, 1, 1, 1, 255, 2, 2, 2, 255, 3, 3, 3, 255, 4, 4, 4, 255, //
            5, 5, 5, 255, 6, 6, 6, 255, 7, 7, 7, 255, 8, 8, 8, 255, 9, 9, 9, 255, //
            10, 10, 10, 255, 11, 11, 11, 255, 12, 12, 12, 255, 13, 13, 13, 255, 14, 14, 14,
            255, //
            15, 15, 15, 255, 16, 16, 16, 255, 17, 17, 17, 255, 18, 18, 18, 255, 19, 19, 19,
            255, //
            20, 20, 20, 255, 21, 21, 21, 255, 22, 22, 22, 255, 23, 23, 23, 255, 24, 24, 24, 255,
        ],
    )
    .unwrap()
}

fn area(x: u32, y: u32, width: u32, height: u32) -> PixelRect {
    PixelRect {
        x,
        y,
        width,
        height,
    }
}

fn taker_showing_five_by_five() -> ScreenshotTaker {
    let capturer = Arc::new(FakeScreenCapturer::showing(shot_of(five_by_five())));
    ScreenshotTaker::new(capturer, Arc::new(InMemoryScreenshotStore::default()))
}

#[test]
fn a_selection_inside_the_frame_crops_exactly_those_pixels_at_native_size() {
    let cropped = five_by_five().crop(area(1, 1, 4, 4));

    let expected = Frame::new(
        4,
        4,
        vec![
            6, 6, 6, 255, 7, 7, 7, 255, 8, 8, 8, 255, 9, 9, 9, 255, //
            11, 11, 11, 255, 12, 12, 12, 255, 13, 13, 13, 255, 14, 14, 14, 255, //
            16, 16, 16, 255, 17, 17, 17, 255, 18, 18, 18, 255, 19, 19, 19, 255, //
            21, 21, 21, 255, 22, 22, 22, 255, 23, 23, 23, 255, 24, 24, 24, 255,
        ],
    )
    .unwrap();
    assert_eq!(cropped.unwrap(), expected);
}

#[test]
fn a_selection_one_pixel_past_the_right_edge_is_rejected() {
    let cropped = five_by_five().crop(area(2, 0, 4, 4));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_selection_one_pixel_past_the_bottom_edge_is_rejected() {
    let cropped = five_by_five().crop(area(0, 2, 4, 4));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_three_by_three_selection_is_too_small() {
    let cropped = five_by_five().crop(area(0, 0, 3, 3));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_selection_four_wide_but_three_high_is_too_small() {
    let cropped = five_by_five().crop(area(0, 0, 4, 3));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_four_by_four_selection_is_large_enough() {
    let cropped = five_by_five().crop(area(0, 0, 4, 4));

    assert_eq!(cropped.unwrap().width(), 4);
}

#[test]
fn coordinates_that_overflow_are_rejected() {
    let cropped = five_by_five().crop(area(u32::MAX, u32::MAX, 4, 4));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_pending_capture_refuses_a_selection_it_cannot_crop_and_still_takes_a_valid_one() {
    let taker = taker_showing_five_by_five();
    let pending = taker.begin(None).unwrap();

    let refuses_tiny = pending.accepts(Selection::Area(area(0, 0, 2, 2)));
    let accepts_valid = pending.accepts(Selection::Area(area(1, 1, 4, 4)));
    let outcome = taker.finish(pending, Selection::Area(area(1, 1, 4, 4)));

    assert!(!refuses_tiny);
    assert!(accepts_valid);
    assert_eq!(outcome.unwrap().screenshot().height(), 4);
}

#[test]
fn a_pending_capture_always_accepts_the_whole_monitor() {
    let taker = taker_showing_five_by_five();

    let pending = taker.begin(None).unwrap();

    assert!(pending.accepts(Selection::WholeMonitor));
}
