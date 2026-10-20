use std::sync::Arc;

use scribe_core::capture::{Frame, InvalidSelection, PixelRect, ScreenshotTaker, Selection};
use scribe_core::testing::{
    FakeScreenCapturer, InMemoryScreenshotStore, opaque_grey_pixels, shot_of,
};

fn five_by_five() -> Frame {
    Frame::new(5, 5, opaque_grey_pixels(0..25)).unwrap()
}

fn area(x: u32, y: u32, (width, height): (u32, u32)) -> PixelRect {
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
    let cropped = five_by_five().crop(area(1, 1, (4, 4)));

    let expected = Frame::new(
        4,
        4,
        opaque_grey_pixels([6, 7, 8, 9, 11, 12, 13, 14, 16, 17, 18, 19, 21, 22, 23, 24]),
    )
    .unwrap();
    assert_eq!(cropped.unwrap(), expected);
}

#[test]
fn a_selection_one_pixel_past_the_right_edge_is_rejected() {
    let cropped = five_by_five().crop(area(2, 0, (4, 4)));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_selection_one_pixel_past_the_bottom_edge_is_rejected() {
    let cropped = five_by_five().crop(area(0, 2, (4, 4)));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_three_by_three_selection_is_too_small() {
    let cropped = five_by_five().crop(area(0, 0, (3, 3)));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_selection_four_wide_but_three_high_is_too_small() {
    let cropped = five_by_five().crop(area(0, 0, (4, 3)));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_four_by_four_selection_is_large_enough() {
    let cropped = five_by_five().crop(area(0, 0, (4, 4)));

    assert_eq!(cropped.unwrap().width(), 4);
}

#[test]
fn coordinates_that_overflow_are_rejected() {
    let cropped = five_by_five().crop(area(u32::MAX, u32::MAX, (4, 4)));

    assert_eq!(cropped.unwrap_err(), InvalidSelection);
}

#[test]
fn a_pending_capture_refuses_a_selection_it_cannot_crop_and_still_takes_a_valid_one() {
    let taker = taker_showing_five_by_five();
    let pending = taker.begin(None).unwrap();

    let refuses_tiny = pending.accepts(Selection::Area(area(0, 0, (2, 2))));
    let accepts_valid = pending.accepts(Selection::Area(area(1, 1, (4, 4))));
    let outcome = taker.finish(pending, Selection::Area(area(1, 1, (4, 4))));

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
