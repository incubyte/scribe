use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::DateTime;
use scribe_core::capture::{Frame, ScreenshotName, SessionMoment};
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
