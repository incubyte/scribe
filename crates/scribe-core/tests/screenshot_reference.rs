use scribe_core::capture::{ScreenshotReference, ServedImage};

const SESSION: &str = "2026-10-20-143005";

#[test]
fn a_listed_reference_parses_and_prints_back_unchanged() {
    let reference = ScreenshotReference::parse("2026-10-20-143005/00-12-34-2.png").unwrap();

    assert_eq!(reference.session().as_str(), SESSION);
    assert_eq!(reference.to_string(), "2026-10-20-143005/00-12-34-2.png");
}

#[test]
fn a_parent_directory_segment_is_rejected() {
    assert!(ScreenshotReference::parse("2026-10-20-143005/../00-12-34.png").is_err());
    assert!(ScreenshotReference::parse("../00-12-34.png").is_err());
    assert!(ScreenshotReference::parse("../2026-10-20-143005/00-12-34.png").is_err());
}

#[test]
fn a_backslash_is_rejected() {
    assert!(ScreenshotReference::parse("2026-10-20-143005\\00-12-34.png").is_err());
    assert!(ScreenshotReference::parse("2026-10-20-143005/..\\00-12-34.png").is_err());
}

#[test]
fn an_absolute_path_is_rejected() {
    assert!(ScreenshotReference::parse("/etc/passwd").is_err());
    assert!(ScreenshotReference::parse("/2026-10-20-143005/00-12-34.png").is_err());
    assert!(ScreenshotReference::parse("C:/2026-10-20-143005/00-12-34.png").is_err());
}

#[test]
fn extra_segments_are_rejected() {
    assert!(ScreenshotReference::parse("2026-10-20-143005/sub/00-12-34.png").is_err());
    assert!(ScreenshotReference::parse("2026-10-20-143005/00-12-34.png/").is_err());
    assert!(ScreenshotReference::parse("00-12-34.png").is_err());
}

#[test]
fn a_name_that_is_not_a_png_screenshot_name_is_rejected() {
    assert!(ScreenshotReference::parse("2026-10-20-143005/00-12-34.jpg").is_err());
    assert!(ScreenshotReference::parse("2026-10-20-143005/00-12-34").is_err());
    assert!(ScreenshotReference::parse("2026-10-20-143005/2026-10-20-143005.md").is_err());
    assert!(ScreenshotReference::parse("2026-10-20-143005/").is_err());
}

fn served(width: u32, height: u32, original: (u32, u32)) -> ServedImage {
    ServedImage {
        png: Vec::new(),
        width,
        height,
        original_width: original.0,
        original_height: original.1,
    }
}

#[test]
fn an_image_served_at_its_original_size_was_not_scaled() {
    assert!(!served(800, 600, (800, 600)).was_scaled());
}

#[test]
fn an_image_served_smaller_than_the_original_was_scaled() {
    assert!(served(400, 300, (800, 600)).was_scaled());
}

#[test]
fn an_image_whose_height_alone_differs_was_scaled() {
    assert!(served(800, 300, (800, 600)).was_scaled());
}
