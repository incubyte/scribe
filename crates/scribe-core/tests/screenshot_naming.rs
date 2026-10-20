use std::time::Duration;

use scribe_core::capture::{InvalidScreenshotName, ScreenshotName};

const TWELVE_MINUTES_THIRTY_FOUR: Duration = Duration::from_secs(12 * 60 + 34);

fn name_at(offset: Duration) -> String {
    ScreenshotName::at(offset).to_string()
}

#[test]
fn a_screenshot_at_the_start_of_the_call_is_named_zero() {
    let name = name_at(Duration::ZERO);

    assert_eq!(name, "00-00-00.png");
}

#[test]
fn a_screenshot_is_named_by_its_offset_into_the_call() {
    let name = name_at(TWELVE_MINUTES_THIRTY_FOUR);

    assert_eq!(name, "00-12-34.png");
}

#[test]
fn the_part_of_a_second_already_passed_does_not_change_the_name() {
    let name = name_at(TWELVE_MINUTES_THIRTY_FOUR + Duration::from_millis(999));

    assert_eq!(name, "00-12-34.png");
}

#[test]
fn a_call_longer_than_ninety_nine_hours_keeps_every_hour_digit() {
    let name = name_at(Duration::from_secs(100 * 3_600 + 1));

    assert_eq!(name, "100-00-01.png");
}

#[test]
fn the_first_name_tried_for_a_moment_has_no_suffix() {
    let first = ScreenshotName::candidates(TWELVE_MINUTES_THIRTY_FOUR).next();

    assert_eq!(first, Some(ScreenshotName::at(TWELVE_MINUTES_THIRTY_FOUR)));
}

#[test]
fn a_second_screenshot_in_the_same_second_is_suffixed_two() {
    let second = ScreenshotName::candidates(TWELVE_MINUTES_THIRTY_FOUR).nth(1);

    assert_eq!(second.unwrap().to_string(), "00-12-34-2.png");
}

#[test]
fn up_to_ninety_nine_screenshots_can_share_one_second() {
    let candidates: Vec<String> = ScreenshotName::candidates(TWELVE_MINUTES_THIRTY_FOUR)
        .map(|name| name.to_string())
        .collect();

    assert_eq!(candidates.len(), 99);
    assert_eq!(candidates.last().unwrap(), "00-12-34-99.png");
}

#[test]
fn suffixed_names_sort_after_the_first_screenshot_of_their_second() {
    let mut names = vec![
        ScreenshotName::parse("00-12-34-2.png").unwrap(),
        ScreenshotName::parse("00-12-35.png").unwrap(),
        ScreenshotName::parse("00-12-34.png").unwrap(),
    ];

    names.sort();

    assert_eq!(
        names,
        vec![
            ScreenshotName::parse("00-12-34.png").unwrap(),
            ScreenshotName::parse("00-12-34-2.png").unwrap(),
            ScreenshotName::parse("00-12-35.png").unwrap(),
        ]
    );
}

#[test]
fn a_saved_name_reads_back_as_the_same_moment() {
    let name = ScreenshotName::parse("00-12-34-2.png").unwrap();

    assert_eq!(name.offset(), TWELVE_MINUTES_THIRTY_FOUR);
    assert_eq!(name.to_string(), "00-12-34-2.png");
}

#[test]
fn a_name_that_climbs_out_of_the_session_folder_is_rejected() {
    assert_eq!(
        ScreenshotName::parse("../00-12-34.png"),
        Err(InvalidScreenshotName)
    );
    assert_eq!(ScreenshotName::parse(".."), Err(InvalidScreenshotName));
}

#[test]
fn a_name_with_folder_separators_is_rejected() {
    assert_eq!(
        ScreenshotName::parse("00/12/34.png"),
        Err(InvalidScreenshotName)
    );
}

#[test]
fn a_name_that_is_not_a_png_is_rejected() {
    assert_eq!(
        ScreenshotName::parse("00-12-34"),
        Err(InvalidScreenshotName)
    );
    assert_eq!(
        ScreenshotName::parse("00-12-34.jpg"),
        Err(InvalidScreenshotName)
    );
}

#[test]
fn suffixes_outside_two_to_ninety_nine_are_rejected() {
    assert_eq!(
        ScreenshotName::parse("00-12-34-1.png"),
        Err(InvalidScreenshotName)
    );
    assert_eq!(
        ScreenshotName::parse("00-12-34-100.png"),
        Err(InvalidScreenshotName)
    );
    assert_eq!(
        ScreenshotName::parse("00-12-34-02.png"),
        Err(InvalidScreenshotName)
    );
}

#[test]
fn a_name_written_differently_from_how_scribe_writes_it_is_rejected() {
    assert_eq!(
        ScreenshotName::parse("0000-12-34.png"),
        Err(InvalidScreenshotName)
    );
    assert_eq!(
        ScreenshotName::parse("00-75-00.png"),
        Err(InvalidScreenshotName)
    );
}
