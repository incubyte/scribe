use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::DateTime;
use scribe_core::capture::{
    Frame, ScreenshotName, ScreenshotReference, ServedImage, SessionMoment,
};
use scribe_core::ports::{ScreenshotStore, StoreError, TranscriptStore};
use scribe_core::recording::{SessionHeader, SessionId};
use scribe_core::testing::frame_filled;
use store_markdown::{MarkdownTranscriptStore, TranscriptsFolder};

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];
const TWELVE_MINUTES_THIRTY_FOUR: Duration = Duration::from_secs(12 * 60 + 34);

struct Recording {
    store: MarkdownTranscriptStore,
    id: SessionId,
    session_folder: PathBuf,
}

impl Recording {
    fn started_in(parent: &Path) -> Recording {
        let store = MarkdownTranscriptStore::new(TranscriptsFolder::at(parent.join("Scribe")));
        let started_at = DateTime::parse_from_rfc3339("2026-10-20T14:30:05+05:30").unwrap();
        let header = SessionHeader::recording(started_at, "speechanalyzer", "en-US");
        let id = store.create(&header).unwrap();
        let session_folder = parent.join("Scribe").join(id.month()).join(id.as_str());
        Recording {
            store,
            id,
            session_folder,
        }
    }

    fn save_at(&self, offset: Duration, screenshot: &Frame) -> Result<ScreenshotName, StoreError> {
        let moment = SessionMoment {
            session: self.id.clone(),
            offset,
        };
        self.store.save(&moment, screenshot)
    }

    fn screenshot_named(&self, name: &str) -> Frame {
        let decoded = image::open(self.session_folder.join(name))
            .unwrap()
            .to_rgba8();
        Frame::new(decoded.width(), decoded.height(), decoded.into_raw()).unwrap()
    }

    fn file_names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.session_folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    fn serve(&self, name: &str, max_long_edge: u32) -> Result<ServedImage, StoreError> {
        let reference =
            ScreenshotReference::new(self.id.clone(), ScreenshotName::parse(name).unwrap());
        self.store.serve(&reference, max_long_edge)
    }

    fn occupy_every_name_at(&self, offset: Duration) {
        for name in ScreenshotName::candidates(offset) {
            fs::write(self.session_folder.join(name.to_string()), b"taken").unwrap();
        }
    }
}

#[test]
fn a_saved_screenshot_lands_in_the_session_folder_named_by_its_offset() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());

    let name = recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(4, 3, RED))
        .unwrap();

    assert_eq!(name.to_string(), "00-12-34.png");
    assert!(recording.session_folder.join("00-12-34.png").is_file());
}

#[test]
fn a_saved_screenshot_is_a_png_with_the_captured_size_and_pixels() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());

    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(4, 3, RED))
        .unwrap();

    assert_eq!(
        recording.screenshot_named("00-12-34.png"),
        frame_filled(4, 3, RED)
    );
}

#[test]
fn a_second_screenshot_in_the_same_second_does_not_overwrite_the_first() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, RED))
        .unwrap();

    let second = recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, BLUE))
        .unwrap();

    assert_eq!(second.to_string(), "00-12-34-2.png");
    assert_eq!(
        recording.screenshot_named("00-12-34.png"),
        frame_filled(2, 2, RED)
    );
    assert_eq!(
        recording.screenshot_named("00-12-34-2.png"),
        frame_filled(2, 2, BLUE)
    );
}

#[test]
fn a_moment_whose_every_name_is_taken_is_refused_without_overwriting() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording.occupy_every_name_at(TWELVE_MINUTES_THIRTY_FOUR);

    let saved = recording.save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, RED));

    assert!(matches!(saved, Err(StoreError::NoFreeScreenshotName)));
    assert_eq!(
        fs::read(recording.session_folder.join("00-12-34.png")).unwrap(),
        b"taken"
    );
}

#[test]
fn saving_a_screenshot_writes_no_other_file() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());

    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, RED))
        .unwrap();

    assert_eq!(
        recording.file_names(),
        vec!["00-12-34.png".to_owned(), format!("{}.md", recording.id)]
    );
}

#[cfg(unix)]
#[test]
fn a_saved_screenshot_is_readable_only_by_the_user() {
    use std::os::unix::fs::PermissionsExt;
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());

    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, RED))
        .unwrap();

    let metadata = fs::metadata(recording.session_folder.join("00-12-34.png")).unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, 0o600);
}

#[test]
fn saving_into_a_session_folder_that_was_removed_fails() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    fs::remove_dir_all(&recording.session_folder).unwrap();

    let saved = recording.save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, RED));

    assert!(matches!(saved, Err(StoreError::Io { .. })));
    assert!(!recording.session_folder.exists());
}

fn tree_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    files.sort();
    files
}

fn collect_files(folder: &Path, files: &mut Vec<(PathBuf, Vec<u8>)>) {
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(&path, files);
        } else {
            files.push((path.clone(), fs::read(&path).unwrap()));
        }
    }
}

fn decoded_size(png: &[u8]) -> (u32, u32) {
    let decoded = image::load_from_memory(png).unwrap();
    (decoded.width(), decoded.height())
}

#[test]
fn a_session_with_no_screenshot_folder_lists_nothing() {
    let parent = tempfile::tempdir().unwrap();
    let store = MarkdownTranscriptStore::new(TranscriptsFolder::at(parent.path().join("Scribe")));
    let id = SessionId::parse("2026-10-20-143005").unwrap();

    let names = ScreenshotStore::list(&store, &id).unwrap();

    assert!(names.is_empty());
}

#[test]
fn only_files_named_like_screenshots_are_listed() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2, 2, RED))
        .unwrap();
    fs::write(recording.session_folder.join("notes.txt"), b"x").unwrap();
    fs::write(recording.session_folder.join("00-12-34.jpg"), b"x").unwrap();
    fs::write(recording.session_folder.join("shot.png"), b"x").unwrap();

    let names = ScreenshotStore::list(&recording.store, &recording.id).unwrap();

    assert_eq!(names, [ScreenshotName::at(TWELVE_MINUTES_THIRTY_FOUR)]);
}

#[test]
fn a_screenshot_over_the_limit_is_served_scaled_to_the_limit_keeping_its_ratio() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(5120, 2880, RED))
        .unwrap();

    let served = recording.serve("00-12-34.png", 1568).unwrap();

    assert_eq!((served.width, served.height), (1568, 882));
    assert_eq!(
        (served.original_width, served.original_height),
        (5120, 2880)
    );
    assert_eq!(decoded_size(&served.png), (1568, 882));
}

#[test]
fn a_portrait_screenshot_is_scaled_on_its_height() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(1000, 1800, RED))
        .unwrap();

    let served = recording.serve("00-12-34.png", 1568).unwrap();

    assert_eq!((served.width, served.height), (871, 1568));
}

#[test]
fn a_screenshot_within_the_limit_is_served_at_its_original_size_and_pixels() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(1200, 800, BLUE))
        .unwrap();

    let served = recording.serve("00-12-34.png", 1568).unwrap();

    assert!(!served.was_scaled());
    assert_eq!((served.width, served.height), (1200, 800));
    assert_eq!(decoded_size(&served.png), (1200, 800));
    assert_eq!(
        image::load_from_memory(&served.png)
            .unwrap()
            .to_rgba8()
            .get_pixel(7, 9)
            .0,
        BLUE
    );
}

#[test]
fn a_screenshot_exactly_at_the_limit_is_not_resized() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(1568, 100, RED))
        .unwrap();

    let served = recording.serve("00-12-34.png", 1568).unwrap();

    assert!(!served.was_scaled());
    assert_eq!((served.width, served.height), (1568, 100));
}

#[test]
fn serving_a_screenshot_changes_nothing_on_disk() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    recording
        .save_at(TWELVE_MINUTES_THIRTY_FOUR, &frame_filled(2000, 1000, RED))
        .unwrap();
    let before = tree_snapshot(parent.path());

    recording.serve("00-12-34.png", 1568).unwrap();

    assert_eq!(tree_snapshot(parent.path()), before);
}

#[test]
fn serving_a_screenshot_that_does_not_exist_reports_it_as_not_found() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());

    let served = recording.serve("00-12-34.png", 1568);

    assert!(matches!(served, Err(StoreError::ScreenshotNotFound { .. })));
}

#[test]
fn serving_a_file_that_is_not_a_png_reports_it_as_malformed() {
    let parent = tempfile::tempdir().unwrap();
    let recording = Recording::started_in(parent.path());
    fs::write(recording.session_folder.join("00-12-34.png"), b"not a png").unwrap();

    let served = recording.serve("00-12-34.png", 1568);

    assert!(matches!(served, Err(StoreError::Malformed { .. })));
}
